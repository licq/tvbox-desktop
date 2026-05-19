use crate::models::{PlayHistory, ResolvedPlayback};
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Response for segment proxy including Content-Range metadata for hls.js buffer tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentProxyResponse {
    /// Base64-encoded response body.
    pub data: String,
    /// Content-Range header value if the response is a partial content (206), e.g. "bytes 0-1023/2048".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_range: Option<String>,
    /// HTTP status code of the upstream response.
    pub status: u16,
}

#[tauri::command]
pub async fn fetch_hls_segment(
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
    referer: Option<String>,
    range_start: Option<u64>,
    range_end: Option<u64>,
) -> Result<SegmentProxyResponse, String> {
    let range = match (range_start, range_end) {
        (Some(start), Some(end)) => {
            Some(format!("bytes={}-{}", start, end))
        }
        (Some(start), None) => {
            Some(format!("bytes={}-", start))
        }
        (None, Some(end)) => {
            Some(format!("bytes=0-{}", end))
        }
        (None, None) => None,
    };
    crate::services::resolver::fetch_hls_segment_internal(
        &url,
        headers.as_ref(),
        referer.as_deref(),
        range.as_deref(),
    )
    .await
}

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
    force_refresh: Option<bool>,
    state: State<'_, AppState>,
) -> Result<ResolvedPlayback, String> {
    crate::services::playback_runtime::resolve_playback_for_input(
        &state.storage,
        &input,
        episode_id,
        force_refresh.unwrap_or(false),
    )
    .await
}

#[tauri::command]
pub async fn fetch_hls_manifest(
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
    referer: Option<String>,
) -> Result<String, String> {
    crate::services::resolver::fetch_hls_manifest_internal(&url, headers.as_ref(), referer.as_deref())
        .await
}
