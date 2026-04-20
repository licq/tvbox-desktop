use crate::models::{CatalogDetail, HomePayload, VodItem};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_vod_items(
    vtype: Option<String>,
    page: u32,
    state: State<'_, AppState>,
) -> Result<Vec<VodItem>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .get_vod_items(vtype, page)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_vod_detail(id: i64, state: State<'_, AppState>) -> Result<VodItem, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_vod_detail(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn search_vod(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<VodItem>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.search_vod(&keyword).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_library_home(state: State<'_, AppState>) -> Result<HomePayload, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_library_home().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_catalog_detail(
    id: i64,
    state: State<'_, AppState>,
) -> Result<CatalogDetail, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_catalog_detail(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
