use crate::models::{PlayHistory, ResolvedPlayback};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_play_history(
    item_type: String,
    item_id: i64,
    progress: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .save_play_history(&item_type, item_id, progress)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_play_history(state: State<'_, AppState>) -> Result<Vec<PlayHistory>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_play_history().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn resolve_playback(
    input: String,
    episode_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<ResolvedPlayback, String> {
    crate::services::playback_runtime::resolve_playback_for_input(
        &state.storage,
        &input,
        episode_id,
    )
    .await
}

#[tauri::command]
pub async fn fetch_hls_manifest(
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    crate::services::resolver::fetch_hls_manifest_internal(&url, headers.as_ref())
        .await
}

#[tauri::command]
pub async fn play_from_source_detail(
    detail_url: String,
    source: String,
) -> Result<String, String> {
    match source.as_str() {
        "zxzj" => crate::services::zxzj::extract_player_url_from_detail(&detail_url).await,
        "jpvod" | "jianpian" => {
            let html = crate::services::jianpian::fetch_detail_page(&detail_url)
                .await
                .map_err(|e| format!("fetch failed: {}", e))?;
            // Try extract_player_url first (jianpian JSONP pattern)
            if let Some(url) = crate::services::jianpian::extract_player_url(&html) {
                return Ok(url);
            }
            // Try parse_detail_page to get episodes
            if let Some(item) = crate::services::jianpian::parse_detail_page(&detail_url, &html) {
                if let Some(first_ep) = item.episodes.first() {
                    // Fallback: return the play URL directly
                    // Note: actual resolution happens in the player page via playbackStore
                    return Ok(first_ep.play_url.clone());
                }
            }
            Err(format!("无法从页面提取播放地址"))
        }
        _ => Err(format!("Unknown source: {}", source)),
    }
}
