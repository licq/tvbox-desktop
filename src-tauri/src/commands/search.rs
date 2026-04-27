use tauri::State;
use crate::AppState;
use crate::services::xb6v::ScrapedCatalogEpisode;
use crate::services::playback_types::PlaybackTarget;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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
    log::info!("[search_all_sources] Command called with keyword: {}", keyword);
    let registry = state.provider_registry.read().await;
    log::info!("[search_all_sources] Registry acquired, providers count: {}", registry.count());
    let results = registry.search_all(&keyword).await;
    log::info!("[search_all_sources] Returning {} results", results.len());
    Ok(results.into_iter().map(|r| SourceSearchResult {
        source_key: r.source_key,
        source_name: r.source_name,
        items: r.items,
    }).collect())
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderDetailResult {
    pub title: Option<String>,
    pub poster: Option<String>,
    pub summary: Option<String>,
    pub episodes: Vec<ScrapedCatalogEpisode>,
}

#[tauri::command]
pub async fn provider_detail(
    source: String,
    ids: String,
    state: State<'_, AppState>,
) -> Result<ProviderDetailResult, String> {
    let registry = state.provider_registry.read().await;
    let provider = registry.get(&source).ok_or_else(|| format!("provider not found: {}", source))?;
    match provider.detail(&ids).await {
        Ok(Some(item)) => Ok(ProviderDetailResult {
            title: Some(item.title),
            poster: item.poster,
            summary: item.summary,
            episodes: item.episodes,
        }),
        Ok(None) => Ok(ProviderDetailResult {
            title: None,
            poster: None,
            summary: None,
            episodes: Vec::new(),
        }),
        Err(e) => {
            log::warn!("[provider_detail] {} failed: {}", source, e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn provider_play(
    source: String,
    flag: String,
    play_url: String,
    state: State<'_, AppState>,
) -> Result<Vec<PlaybackTarget>, String> {
    let registry = state.provider_registry.read().await;
    let provider = registry.get(&source).ok_or_else(|| format!("provider not found: {}", source))?;
    provider.play(&flag, &play_url).await.map_err(|e| {
        log::warn!("[provider_play] {} failed: {}", source, e);
        e.to_string()
    })
}
