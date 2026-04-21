use crate::models::Subscription;
use crate::services::{Parser, TvboxConfigParser};
use crate::AppState;
use reqwest::Response;
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
    let response = match fetch_with_proxy_fallback(&subscription.url).await {
        Ok(response) => response,
        Err(e) => {
            let error = format!("网络请求失败: {}", e);
            persist_refresh_failure(storage.clone(), id, &fallback_kind, &error).await?;
            return Err(error);
        }
    };
    let status = response.status();
    log::info!("响应状态: {}", status);
    let response_text = match response.text().await {
        Ok(response_text) => response_text,
        Err(e) => {
            let error = format!("读取响应失败: {}", e);
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

async fn fetch_with_proxy_fallback(url: &str) -> Result<Response, reqwest::Error> {
    match reqwest::get(url).await {
        Ok(response) => Ok(response),
        Err(initial_error) => {
            let proxy_env_exists = std::env::var_os("http_proxy").is_some()
                || std::env::var_os("https_proxy").is_some()
                || std::env::var_os("HTTP_PROXY").is_some()
                || std::env::var_os("HTTPS_PROXY").is_some();

            if !proxy_env_exists {
                return Err(initial_error);
            }

            let client = reqwest::Client::builder().no_proxy().build()?;
            match client.get(url).send().await {
                Ok(response) => {
                    log::warn!("代理请求失败后已回退直连: {}", url);
                    Ok(response)
                }
                Err(_) => Err(initial_error),
            }
        }
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
