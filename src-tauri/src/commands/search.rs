use tauri::State;
use crate::AppState;
use crate::services::xb6v::ScrapedCatalogEpisode;
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

#[tauri::command]
pub async fn provider_detail(
    source: String,
    ids: String,
    state: State<'_, AppState>,
) -> Result<Vec<ScrapedCatalogEpisode>, String> {
    let registry = state.provider_registry.read().await;
    let provider = registry.get(&source).ok_or_else(|| format!("provider not found: {}", source))?;
    match provider.detail(&ids).await {
        Ok(Some(item)) => Ok(item.episodes),
        Ok(None) => Ok(Vec::new()),
        Err(e) => {
            log::warn!("[provider_detail] {} failed: {}", source, e);
            Err(e.to_string())
        }
    }
}
