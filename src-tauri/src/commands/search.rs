use tauri::State;
use crate::AppState;
use crate::services::xb6v::ScrapedCatalogEpisode;
use crate::services::xb6v::ScrapedCatalogItem;
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
    let pairs = registry.all_provider_pairs();
    let storage = state.storage.clone();
    log::info!("[search_all_sources] Registry acquired, providers count: {}", pairs.len());

    let mut handles = Vec::new();
    for (source_key, provider) in pairs {
        let storage = storage.clone();
        let kw = keyword.clone();
        let sk = source_key.clone();

        handles.push(tokio::spawn(async move {
            // Check cache first
            match storage.get_source_search_cache(&sk, &kw) {
                Ok(Some((cached_json, expired))) => {
                    // Deserialize cached results
                    match serde_json::from_str::<Vec<ScrapedCatalogItem>>(&cached_json) {
                        Ok(items) => {
                            if expired && !items.is_empty() {
                                // Background refresh: spawn fire-and-forget task
                                let storage = storage.clone();
                                let provider = provider.clone();
                                let kw = kw.clone();
                                let sk = sk.clone();
                                tokio::spawn(async move {
                                    log::info!("[search_all_sources] Background refresh for {}/{}", sk, kw);
                                    match provider.search(&kw).await {
                                        Ok(new_items) if !new_items.is_empty() => {
                                            if let Ok(json) = serde_json::to_string(&new_items) {
                                                let _ = storage.set_source_search_cache(&sk, &kw, &json);
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            }
                            let name = provider.source_name().to_string();
                            return Some(SourceSearchResult { source_key: sk, source_name: name, items });
                        }
                        Err(e) => {
                            log::warn!("[search_all_sources] Cache deserialize failed for {}: {}", sk, e);
                            // Fall through to real fetch
                        }
                    }
                }
                Ok(None) => {} // No cache, fall through to real fetch
                Err(e) => {
                    log::warn!("[search_all_sources] Cache check failed for {}: {}", sk, e);
                }
            }

            // No valid cache: fetch from provider in real time
            match provider.search(&kw).await {
                Ok(items) => {
                    let name = provider.source_name().to_string();
                    if !items.is_empty() {
                        // Cache result if non-empty
                        if let Ok(json) = serde_json::to_string(&items) {
                            let _ = storage.set_source_search_cache(&sk, &kw, &json);
                        }
                    } else {
                        // Delete stale cache when provider now returns empty
                        // (e.g. auete search disabled, old home-page cache still present)
                        let _ = storage.delete_source_search_cache(&sk, &kw);
                    }
                    Some(SourceSearchResult { source_key: sk, source_name: name, items })
                }
                Err(_) => None,
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Some(result) = handle.await.unwrap_or(None) {
            results.push(result);
        }
    }

    log::info!("[search_all_sources] Returning {} results", results.len());
    Ok(results)
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
