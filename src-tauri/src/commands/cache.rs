use tauri::State;
use crate::AppState;

#[tauri::command]
pub async fn clear_source_search_cache(state: State<'_, AppState>) -> Result<u32, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.clear_source_search_cache().map(|n| n as u32).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn clear_douban_search_cache(state: State<'_, AppState>) -> Result<u32, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.clear_douban_search_cache().map(|n| n as u32).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
