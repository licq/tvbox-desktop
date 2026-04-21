use crate::models::Subscription;
use crate::services::{Parser, TvboxConfigParser};
use crate::AppState;
use encoding_rs::{GB18030, GBK};
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
    let fetched = match fetch_subscription_content(&subscription.url).await {
        Ok(fetched) => fetched,
        Err(e) => {
            let error = format!("网络请求失败: {}", e);
            persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
            return Err(error);
        }
    };
    let response_text = fetched.body.clone();
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
                    let error = enrich_simple_parse_error(&e, &fetched);
                    persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
                    return Err(error);
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

#[derive(Clone)]
struct FetchedContent {
    body: String,
    final_url: String,
    status: u16,
    content_type: Option<String>,
}

async fn fetch_subscription_content(url: &str) -> Result<FetchedContent, reqwest::Error> {
    let primary = fetch_text_no_proxy(url).await?;
    if looks_like_json(&primary.body) {
        return Ok(primary);
    }

    if let Some(fragment) = extract_json_object_fragment(&primary.body) {
        return Ok(FetchedContent {
            body: fragment,
            final_url: primary.final_url,
            status: primary.status,
            content_type: primary.content_type,
        });
    }

    for candidate_url in extract_candidate_urls(&primary.body) {
        if candidate_url == url {
            continue;
        }

        match fetch_text_no_proxy(&candidate_url).await {
            Ok(candidate_body) if looks_like_json(&candidate_body.body) => {
                log::info!("入口返回非 JSON，已自动跟进候选配置地址: {}", candidate_url);
                return Ok(candidate_body);
            }
            Ok(candidate_body) => {
                if let Some(fragment) = extract_json_object_fragment(&candidate_body.body) {
                    log::info!("候选地址返回页面，已提取嵌入 JSON: {}", candidate_url);
                    return Ok(FetchedContent {
                        body: fragment,
                        final_url: candidate_body.final_url,
                        status: candidate_body.status,
                        content_type: candidate_body.content_type,
                    });
                }
            }
            Err(e) => {
                log::warn!("尝试候选配置地址失败: {} ({})", candidate_url, e);
            }
        }
    }

    Ok(primary)
}

async fn fetch_text_no_proxy(url: &str) -> Result<FetchedContent, reqwest::Error> {
    let client = reqwest::Client::builder().no_proxy().build()?;
    let response = client.get(url).send().await?;
    let final_url = response.url().to_string();
    let status = response.status().as_u16();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let bytes = response.bytes().await?;
    let body = decode_response_body(&bytes, content_type.as_deref());

    Ok(FetchedContent {
        body,
        final_url,
        status,
        content_type,
    })
}

fn looks_like_json(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn extract_candidate_urls(content: &str) -> Vec<String> {
    let normalized_content = content
        .replace("\\/", "/")
        .replace("\\u003a", ":")
        .replace("\\u002f", "/")
        .replace("\\x3a", ":")
        .replace("\\x2f", "/");

    let decoded_percent_content = decode_common_percent_encoding(&normalized_content);

    let trimmed = content.trim();
    let mut urls: Vec<String> = if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        vec![trimmed.to_string()]
    } else {
        Vec::new()
    };

    let mut extracted: Vec<String> = decoded_percent_content
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
    urls.append(&mut extracted);

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

fn extract_json_object_fragment(content: &str) -> Option<String> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }

    let fragment = content[start..=end].trim();
    if looks_like_json(fragment) && fragment.contains(':') {
        Some(fragment.to_string())
    } else {
        None
    }
}

fn enrich_simple_parse_error(base_error: &str, fetched: &FetchedContent) -> String {
    if !base_error.starts_with("JSON解析失败") {
        return base_error.to_string();
    }

    let preview: String = fetched
        .body
        .chars()
        .take(160)
        .collect::<String>()
        .replace('\n', " ")
        .replace('\r', " ");

    format!(
        "{} | 响应状态={} | 类型={} | 最终地址={} | 内容预览={}",
        base_error,
        fetched.status,
        fetched.content_type.as_deref().unwrap_or("unknown"),
        fetched.final_url,
        preview
    )
}

fn decode_response_body(bytes: &[u8], content_type: Option<&str>) -> String {
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return text;
    }

    let charset = extract_charset(content_type);
    if let Some(charset) = charset {
        if charset.contains("gb18030") {
            let (decoded, _, _) = GB18030.decode(bytes);
            return decoded.into_owned();
        }
        if charset.contains("gbk") || charset.contains("gb2312") {
            let (decoded, _, _) = GBK.decode(bytes);
            return decoded.into_owned();
        }
    }

    let (decoded_gbk, _, _) = GBK.decode(bytes);
    let gbk_text = decoded_gbk.into_owned();
    if gbk_text.contains("http://") || gbk_text.contains("https://") || gbk_text.contains("复制") {
        return gbk_text;
    }

    String::from_utf8_lossy(bytes).into_owned()
}

fn extract_charset(content_type: Option<&str>) -> Option<String> {
    let content_type = content_type?;
    let lower = content_type.to_ascii_lowercase();
    let marker = "charset=";
    let index = lower.find(marker)?;
    Some(
        lower[index + marker.len()..]
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string(),
    )
}

fn decode_common_percent_encoding(content: &str) -> String {
    let mut result = content.to_string();
    let replacements = [
        ("%3A", ":"),
        ("%3a", ":"),
        ("%2F", "/"),
        ("%2f", "/"),
        ("%5C", "\\"),
        ("%5c", "\\"),
        ("%3D", "="),
        ("%3d", "="),
    ];
    for (from, to) in replacements {
        result = result.replace(from, to);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::{
        decode_common_percent_encoding, extract_candidate_urls, extract_json_object_fragment,
        looks_like_json,
    };

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

    #[test]
    fn extracts_escaped_urls() {
        let html = r#"var data = "https:\/\/example.com\/config.json";"#;
        let urls = extract_candidate_urls(html);
        assert_eq!(urls.first().map(String::as_str), Some("https://example.com/config.json"));
    }

    #[test]
    fn decodes_percent_escaped_http_tokens() {
        let encoded = "https%3A%2F%2Fexample.com%2Ftv";
        let decoded = decode_common_percent_encoding(encoded);
        assert_eq!(decoded, "https://example.com/tv");
    }

    #[test]
    fn extracts_embedded_json_fragment() {
        let html = r#"<html><script>window.cfg={"sites":[],"lives":[]};</script></html>"#;
        let fragment = extract_json_object_fragment(html).expect("fragment expected");
        assert!(fragment.contains("\"sites\""));
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
