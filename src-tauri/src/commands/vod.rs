use crate::models::{CatalogDetail, HomeCatalogItem, HomePayload, VodItem};
use crate::services::scrape_catalog_detail_from_json;
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
pub async fn get_catalog_items(
    item_type: Option<String>,
    keyword: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<HomeCatalogItem>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .get_catalog_items(item_type, keyword)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_catalog_detail(
    id: i64,
    state: State<'_, AppState>,
) -> Result<CatalogDetail, String> {
    let storage = state.storage.clone();
    let detail = tokio::task::spawn_blocking({
        let storage = storage.clone();
        move || storage.get_catalog_detail(id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    if detail.episode_groups.is_empty() {
        if let Some(detail_json) = detail.item.detail_json.clone() {
            match scrape_catalog_detail_from_json(&detail_json).await {
                Ok(Some(scraped)) if !scraped.episodes.is_empty() => {
                    tokio::task::spawn_blocking({
                        let storage = storage.clone();
                        let scraped = scraped.clone();
                        move || {
                            storage
                                .replace_catalog_item_detail(id, &scraped)
                                .map_err(|e| e.to_string())
                        }
                    })
                    .await
                    .map_err(|e| e.to_string())??;

                    return tokio::task::spawn_blocking(move || {
                        storage.get_catalog_detail(id).map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("抓取 catalog detail 失败 item_id={id}: {error}");
                }
            }
        }
    }

    Ok(detail)
}
