use tauri::State;
use crate::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSearchResult {
    pub source_key: String,
    pub source_name: String,
    pub items: Vec<crate::services::xb6v::ScrapedCatalogItem>,
}

#[tauri::command]
pub async fn search_all_sources(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<SourceSearchResult>, String> {
    let registry = state.provider_registry.read().await;
    let results = registry.search_all(&keyword).await;
    Ok(results.into_iter().map(|r| SourceSearchResult {
        source_key: r.source_key,
        source_name: r.source_name,
        items: r.items,
    }).collect())
}
