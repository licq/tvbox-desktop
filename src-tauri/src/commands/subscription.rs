use crate::models::Subscription;
use crate::services::Parser;
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
        storage.add_subscription(&name, &url).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_subscriptions(
    state: State<'_, AppState>,
) -> Result<Vec<Subscription>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_subscriptions().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn delete_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.delete_subscription(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn refresh_subscription(
    id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();

    // Get subscription info
    let subscription = {
        let storage = storage.clone();
        tokio::task::spawn_blocking(move || storage.get_subscription(id))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?
    };

    // Fetch content from URL
    let response = reqwest::get(&subscription.url)
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    // Parse JSON
    let parsed = Parser::parse_subscription(&response).map_err(|e| e.to_string())?;

    // Prepare data for storage
    let lives: Vec<_> = parsed
        .lives
        .unwrap_or_default()
        .into_iter()
        .map(|live| {
            (
                live.name,
                live.logo,
                live.url,
                live.category,
            )
        })
        .collect();

    let vods: Vec<_> = parsed
        .vods
        .unwrap_or_default()
        .into_iter()
        .map(|vod| {
            let episodes = Parser::parse_episodes(vod.episodes);
            (
                vod.name,
                vod.vtype.unwrap_or_default(),
                vod.poster,
                vod.description,
                episodes,
            )
        })
        .collect();

    // Update database
    tokio::task::spawn_blocking(move || {
        storage
            .refresh_subscription(id, lives, vods)
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
        storage.toggle_subscription(id, enabled).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
