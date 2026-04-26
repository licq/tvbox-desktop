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
pub async fn get_catalog_types(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .get_distinct_item_types()
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
        // Try provider-based detail resolution first
        if let Some(ref detail_json) = detail.item.detail_json {
            let parsed: serde_json::Value = match serde_json::from_str(detail_json) {
                Ok(v) => v,
                Err(_) => { /* fall through to scrape_catalog_detail_from_json */ return Ok(detail); }
            };
            let source = parsed.get("source").and_then(|v| v.as_str()).unwrap_or("");
            let ids = parsed.get("ids").and_then(|v| v.as_str()).unwrap_or("");
            let detail_key = if !ids.is_empty() { ids } else {
                parsed.get("url").and_then(|v| v.as_str()).unwrap_or("")
            };

            if !source.is_empty() && !detail_key.is_empty() {
                let registry = state.provider_registry.read().await;
                if let Some(provider) = registry.get(source) {
                    match provider.detail(detail_key).await {
                        Ok(Some(scraped)) if !scraped.episodes.is_empty() => {
                            let storage = state.storage.clone();
                            let storage2 = storage.clone();
                            tokio::task::spawn_blocking({
                                let scraped = scraped.clone();
                                move || storage.replace_catalog_item_detail(id, &scraped).map_err(|e| e.to_string())
                            }).await.map_err(|e| e.to_string())??;

                            return tokio::task::spawn_blocking(move || {
                                storage2.get_catalog_detail(id).map_err(|e| e.to_string())
                            }).await.map_err(|e| e.to_string())?;
                        }
                        Ok(_) => {}
                        Err(e) => log::warn!("Provider detail failed for {}: {}", source, e),
                    }
                }
            }
        }

        // Fall back to scrape_catalog_detail_from_json
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
