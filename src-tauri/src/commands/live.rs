use crate::models::{LiveChannelGroup, MergedLiveChannel};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn get_live_channels(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<MergedLiveChannel>, String> {
    match category {
        Some(cat) => state
            .storage
            .get_merged_live_channels_by_category(&cat)
            .map_err(|e| e.to_string()),
        None => state
            .storage
            .get_merged_live_channels()
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub async fn get_live_channel_groups(
    state: State<'_, AppState>,
) -> Result<Vec<LiveChannelGroup>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_live_channel_groups().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn get_live_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state
        .storage
        .get_live_categories()
        .map_err(|e| e.to_string())
}
