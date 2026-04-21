use crate::models::Subscription;
use crate::services::{Parser, TvboxConfigParser};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn add_subscription(
    name: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Subscription, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .add_subscription(&name, &url)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_subscriptions(state: State<'_, AppState>) -> Result<Vec<Subscription>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_subscriptions().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_subscription(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.delete_subscription(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_subscription(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let storage = state.storage.clone();

    // Get subscription info
    let subscription = {
        let storage = storage.clone();
        tokio::task::spawn_blocking(move || storage.get_subscription(id))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
    };
    let fallback_kind = subscription.kind.clone();

    // Fetch content from URL
    log::info!(
        "开始获取订阅: {} url={}",
        subscription.name,
        subscription.url
    );
    let response_text = match fetch_subscription_content(&subscription.url).await {
        Ok(response_text) => response_text,
        Err(e) => {
            let error = format!("网络请求失败: {}", e);
            persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
            return Err(error);
        }
    };
    log::info!("响应长度: {}", response_text.len());

    // Parse JSON
    log::info!(
        "订阅 {} 响应长度: {}",
        subscription.name,
        response_text.len()
    );

    let source_kind = Parser::detect_source_kind(&response_text);

    match source_kind {
        "tvbox_config" => {
            let parsed = match TvboxConfigParser::parse(&response_text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    persist_refresh_failure(storage.clone(), id, &fallback_kind, &e).await?;
                    return Err(e);
                }
            };
            log::info!(
                "TVBox配置解析完成: {} sites, {} parses, {} lives",
                parsed.sites.len(),
                parsed.parses.len(),
                parsed.lives.len()
            );

            let refresh_storage = storage.clone();
            let refresh_result = tokio::task::spawn_blocking(move || {
                refresh_storage
                    .refresh_tvbox_subscription(id, &response_text, &parsed)
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())?;

            if let Err(error) = refresh_result {
                persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                return Err(error);
            }
        }
        _ => {
            let parsed = match Parser::parse_subscription(&response_text) {
                Ok(parsed) => parsed,
                Err(e) => {
                    persist_refresh_failure(storage.clone(), id, &fallback_kind, &e).await?;
                    return Err(e);
                }
            };
            log::info!(
                "简单订阅解析完成: {} lives, {} vods",
                parsed.lives.as_ref().map_or(0, |l| l.len()),
                parsed.vods.as_ref().map_or(0, |v| v.len())
            );

            let lives_data: Vec<_> = parsed
                .lives
                .unwrap_or_default()
                .into_iter()
                .map(|live| (live.name, live.logo, live.url, live.category))
                .collect();

            let vods_data: Vec<_> = parsed
                .vods
                .unwrap_or_default()
                .into_iter()
                .map(|vod| {
                    (
                        vod.name,
                        vod.vtype.unwrap_or_default(),
                        vod.poster,
                        vod.description,
                        Parser::parse_episodes(vod.episodes),
                    )
                })
                .collect();

            log::info!(
                "提取简单源数据: {} lives, {} vods",
                lives_data.len(),
                vods_data.len()
            );

            let refresh_storage = storage.clone();
            let refresh_result = tokio::task::spawn_blocking(move || {
                refresh_storage
                    .refresh_subscription(id, lives_data, vods_data)
                    .map_err(|e| e.to_string())
            })
            .await
            .map_err(|e| e.to_string())?;

            if let Err(error) = refresh_result {
                persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                return Err(error);
            }
        }
    }

    Ok(())
}

async fn fetch_subscription_content(url: &str) -> Result<String, reqwest::Error> {
    let primary = fetch_text_no_proxy(url).await?;
    if looks_like_json(&primary) {
        return Ok(primary);
    }

    for candidate_url in extract_candidate_urls(&primary) {
        if candidate_url == url {
            continue;
        }

        match fetch_text_no_proxy(&candidate_url).await {
            Ok(candidate_body) if looks_like_json(&candidate_body) => {
                log::info!("入口返回非 JSON，已自动跟进候选配置地址: {}", candidate_url);
                return Ok(candidate_body);
            }
            Ok(_) => {}
            Err(e) => {
                log::warn!("尝试候选配置地址失败: {} ({})", candidate_url, e);
            }
        }
    }

    Ok(primary)
}

async fn fetch_text_no_proxy(url: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::builder().no_proxy().build()?;
    client.get(url).send().await?.text().await
}

fn looks_like_json(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn extract_candidate_urls(content: &str) -> Vec<String> {
    let mut urls: Vec<String> = content
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(ch, '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}')
        })
        .filter_map(|raw| {
            let cleaned = raw
                .trim_matches(|ch: char| matches!(ch, ',' | ';' | '"' | '\'' | ')' | ']'))
                .replace("&amp;", "&");
            if cleaned.starts_with("http://") || cleaned.starts_with("https://") {
                Some(cleaned)
            } else {
                None
            }
        })
        .collect();

    urls.sort_by_key(|url| {
        let lower = url.to_ascii_lowercase();
        let score = if lower.ends_with(".json") {
            0
        } else if lower.contains("json") || lower.contains("config") || lower.contains("tv") {
            1
        } else {
            2
        };
        (score, url.len())
    });
    urls.dedup();
    urls
}

#[cfg(test)]
mod tests {
    use super::{extract_candidate_urls, looks_like_json};

    #[test]
    fn detects_json_like_payload() {
        assert!(looks_like_json("{\"a\":1}"));
        assert!(looks_like_json(" [1,2,3]"));
        assert!(!looks_like_json("<html>"));
    }

    #[test]
    fn extracts_and_prioritizes_candidate_urls() {
        let html = r#"
            <a href="https://example.com/page">page</a>
            <a href="https://example.com/config.json">json</a>
        "#;
        let urls = extract_candidate_urls(html);
        assert_eq!(urls.first().map(String::as_str), Some("https://example.com/config.json"));
    }
}

async fn persist_refresh_failure(
    storage: crate::services::Storage,
    id: i64,
    kind: &str,
    error: &str,
) -> Result<(), String> {
    let kind = kind.to_string();
    let error = error.to_string();
    tokio::task::spawn_blocking(move || {
        storage
            .record_subscription_refresh_failure(id, &kind, &error)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn toggle_subscription(
    id: i64,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .toggle_subscription(id, enabled)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
