use crate::models::LiveChannel;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_live_channels(
    category: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<LiveChannel>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_live_channels(category).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_live_categories(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_live_categories().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
