use crate::models::DoubanSubjectMeta;
use crate::services::douban::DoubanSubjectScraper;
use crate::AppState;
use crate::models::DoubanHot;
use crate::models::DoubanHotItem;
use tauri::{AppHandle, State};
use serde::{Deserialize, Serialize};
use base64::Engine;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Bounded in-memory cache for proxy_image results
// Key: URL, Value: base64-encoded image data
// Bounded to prevent unbounded memory growth.
const IMAGE_CACHE_CAPACITY: usize = 500;
static IMAGE_CACHE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Insert into bounded image cache, evicting oldest entry when full.
fn cache_image(url: String, b64: String) {
    let mut cache = IMAGE_CACHE.lock().unwrap();
    // Evict one entry when at capacity to keep cache bounded
    if cache.len() >= IMAGE_CACHE_CAPACITY && !cache.contains_key(&url) {
        if let Some(key) = cache.keys().next().cloned() {
            cache.remove(&key);
        }
    }
    cache.insert(url, b64);
}

/// Static HTTP client for proxy_image (reused across calls instead of creating one per call)
static PROXY_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to create proxy HTTP client")
});

#[tauri::command]
pub async fn get_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    state.storage.get_douban_hot().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    let crawler = crate::services::douban::DoubanCrawler::new();
    let items = crawler.fetch_hot_list().await?;
    state.storage.clear_douban_hot().map_err(|e| e.to_string())?;
    state.storage.upsert_douban_hot(&items).map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
pub async fn get_matched_hot_list(state: State<'_, AppState>) -> Result<Vec<MatchedHotItem>, String> {
    // Get Douban items and match with VOD items from subscriptions
    let douban_items = state.storage.get_douban_hot().map_err(|e| e.to_string())?;
    let vod_items = state.storage.get_vod_items(None, 0).map_err(|e| e.to_string())?;

    let matched = douban_items
        .into_iter()
        .filter_map(|douban| {
            // Try to find matching VOD item
            vod_items.iter().find(|vod| {
                fuzzy_match(&douban.name, &vod.name, douban.year)
            }).map(|vod| MatchedHotItem {
                douban,
                vod_id: vod.id,
                vod_name: vod.name.clone(),
            })
        })
        .collect();

    Ok(matched)
}

#[tauri::command]
pub async fn fetch_all_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    log::info!("[fetch_all_douban_hot] Starting...");
    let crawler = crate::services::douban::DoubanCrawler::new();
    let items = crawler.fetch_all().await?;
    log::info!("[fetch_all_douban_hot] Fetched {} items from Douban", items.len());
    state.storage.clear_douban_hot().map_err(|e| e.to_string())?;
    state.storage.upsert_douban_hot(&items).map_err(|e| e.to_string())?;
    log::info!("[fetch_all_douban_hot] Saved {} items to DB", items.len());
    Ok(items)
}

#[tauri::command]
pub async fn search_vod_sources(
    title: String,
    _item_type: Option<String>,
) -> Result<Vec<DoubanHotItem>, String> {
    let search = crate::services::search::SearchService::new();
    let results = search.search_all(&title).await;
    Ok(results)
}

#[tauri::command]
pub async fn get_douban_hot_by_id(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Option<DoubanHot>, String> {
    state.storage.get_douban_hot_by_id(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_douban_hot_by_type(
    state: State<'_, AppState>,
    item_type: String,
) -> Result<Vec<DoubanHot>, String> {
    state.storage.get_douban_hot_by_type(&item_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn proxy_image(url: String) -> Result<String, String> {
    // Validate URL is doubanio.com
    if !url.contains("doubanio.com") {
        return Err("Only doubanio.com URLs are allowed".to_string());
    }

    // Check cache first
    {
        let cache = IMAGE_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&url) {
            return Ok(cached.clone());
        }
    }

    // Use static/reusable HTTP client (prevents creating new connection pool per call)
    let resp = PROXY_CLIENT
        .get(&url)
        .header("Referer", "https://movie.douban.com/")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    // Cache the result (bounded to prevent unbounded memory growth)
    cache_image(url.clone(), b64.clone());

    Ok(b64)
}

fn fuzzy_match(douban_name: &str, vod_name: &str, douban_year: Option<i32>) -> bool {
    // Normalize names
    let d = normalize_name(douban_name);
    let v = normalize_name(vod_name);

    // Calculate similarity
    let similarity = calculate_similarity(&d, &v);
    if similarity < 0.8 {
        return false;
    }

    // Year check (if both have year info)
    if let (Some(dy), Some(vy)) = (douban_year, extract_year(vod_name)) {
        if (dy - vy).abs() > 1 {
            return false;
        }
    }

    true
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn calculate_similarity(a: &str, b: &str) -> f64 {
    // Simple Jaccard similarity based on character n-grams
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() || b_chars.is_empty() {
        return 0.0;
    }

    let a_ngrams: std::collections::HashSet<String> = (0..a_chars.len())
        .filter_map(|i| {
            if i + 2 <= a_chars.len() {
                Some(a_chars[i..i+2].iter().collect())
            } else {
                None
            }
        })
        .collect();

    let b_ngrams: std::collections::HashSet<String> = (0..b_chars.len())
        .filter_map(|i| {
            if i + 2 <= b_chars.len() {
                Some(b_chars[i..i+2].iter().collect())
            } else {
                None
            }
        })
        .collect();

    let intersection = a_ngrams.intersection(&b_ngrams).count();
    let union = a_ngrams.union(&b_ngrams).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn extract_year(name: &str) -> Option<i32> {
    // Extract 4-digit year from name like "Movie (2023)" or "Movie 2023"
    let re = regex::Regex::new(r"\((\d{4})\)|(\d{4})").ok()?;
    for cap in re.captures_iter(name) {
        if let Some(m1) = cap.get(1) {
            return m1.as_str().parse().ok();
        }
        if let Some(m2) = cap.get(2) {
            return m2.as_str().parse().ok();
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedHotItem {
    pub douban: DoubanHot,
    pub vod_id: i64,
    pub vod_name: String,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn fetch_all_douban_hot_persists_to_database() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        let app_data_dir = std::env::temp_dir().join(format!("tvbox-douban-test-{}", unique));
        std::fs::create_dir_all(&app_data_dir).expect("temp app data dir should create");

        let storage = crate::services::Storage::new(app_data_dir.clone())
            .expect("storage should initialize");

        // Test one category first
        let crawler = crate::services::douban::DoubanCrawler::new();
        let items = crawler.fetch_all().await.expect("fetch_all should succeed");
        println!("fetch_all returned {} items", items.len());

        // Persist to DB
        storage.clear_douban_hot().expect("clear should work");
        storage.upsert_douban_hot(&items).expect("upsert should work");

        // Verify data was persisted
        let count = items.len();
        assert!(count >= 100, "expected at least 100 items (4 categories x 30), got {}", count);

        // Check DB has data (get_douban_hot has LIMIT 100, so may be less)
        let persisted = storage.get_douban_hot().expect("get_douban_hot should work");
        println!("DB returned {} items (limited to 100)", persisted.len());
        assert!(persisted.len() >= 100, "DB should return at least 100 items");

        // Check item_type distribution (these queries don't have LIMIT so they should sum to count)
        let movie_count = storage.get_douban_hot_by_type("movie").expect("movie query should work").len();
        let series_count = storage.get_douban_hot_by_type("series").expect("series query should work").len();
        let variety_count = storage.get_douban_hot_by_type("variety").expect("variety query should work").len();
        let anime_count = storage.get_douban_hot_by_type("anime").expect("anime query should work").len();
        println!("movie={}, series={}, variety={}, anime={}", movie_count, series_count, variety_count, anime_count);
        assert_eq!(movie_count + series_count + variety_count + anime_count, count, "type counts should sum to total");

        // Cleanup
        std::fs::remove_dir_all(&app_data_dir).ok();
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_douban_subject_metadata(
    app: AppHandle,
    state: State<'_, AppState>,
    item_id: i64,
) -> Result<Option<DoubanSubjectMeta>, String> {
    // 获取 item 的 title 和 douban_id
    let title = {
        match state.storage.get_catalog_detail(item_id) {
            Ok(detail) => Some(detail.item.title),
            Err(_) => None,
        }
    };

    let douban_id = if let Some(ref t) = title {
        let douban_items = state.storage.get_douban_hot().map_err(|e| e.to_string())?;
        let normalized = t.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase();

        let mut best_match: Option<i64> = None;
        let mut best_score = 0.8f64;

        for item in douban_items.iter().take(500) {
            let item_normalized = item.name
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .to_lowercase();

            let score = calculate_similarity(&normalized, &item_normalized);
            if score > best_score {
                best_score = score;
                best_match = Some(item.id);
            }
        }
        best_match
    } else {
        None
    };

    if let Some(dbid) = douban_id {
        let meta = DoubanSubjectScraper::scrape(&app, dbid).await;
        match meta {
            Ok(m) => Ok(Some(m)),
            Err(e) => {
                log::warn!("Failed to fetch Douban meta for {}: {}", dbid, e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_douban_metadata_by_id(
    app: AppHandle,
    state: State<'_, AppState>,
    douban_id: i64,
) -> Result<Option<DoubanSubjectMeta>, String> {
    // OPTIMIZATION 3: Check cache first
    if let Ok(Some(cached)) = state.storage.get_douban_subject_meta(douban_id) {
        log::info!("[fetch_douban_metadata_by_id] Cache hit for douban_id={}", douban_id);
        return Ok(Some(cached));
    }

    let meta = DoubanSubjectScraper::scrape(&app, douban_id).await;
    match meta {
        Ok(m) => {
            // Cache the result
            if let Err(e) = state.storage.upsert_douban_subject_meta(&m) {
                log::warn!("[fetch_douban_metadata_by_id] Failed to cache: {}", e);
            }
            Ok(Some(m))
        }
        Err(e) => {
            log::warn!("Failed to fetch Douban meta for {}: {}", douban_id, e);
            Ok(None)
        }
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn search_douban_subject_by_keyword(
    app: AppHandle,
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Option<DoubanSubjectMeta>, String> {
    // Cache check: if we have a non-expired cached result, return it immediately
    match state.storage.get_douban_search_cache(&keyword) {
        Ok(Some((cached_json, expired))) => {
            match serde_json::from_str::<DoubanSubjectMeta>(&cached_json) {
                Ok(meta) => {
                    if expired {
                        // Background refresh
                        let app = app.clone();
                        let kw = keyword.clone();
                        let storage = state.storage.clone();
                        tokio::spawn(async move {
                            log::info!("[search_douban] Background refresh for keyword: {}", kw);
                            refresh_douban_search_cache(&app, &kw, &storage).await;
                        });
                    }
                    return Ok(Some(meta));
                }
                Err(e) => {
                    log::warn!("[search_douban] Cache deserialize failed: {}", e);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("[search_douban] Cache check failed: {}", e);
        }
    }

    // Step 1: Try DB hot list first (fast path, no WebView needed)
    {
        let douban_items = state.storage.get_douban_hot().map_err(|e| e.to_string())?;
        let normalized = keyword.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase();

        for item in douban_items.iter().take(500) {
            let item_normalized = item.name
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .to_lowercase();

            let score = calculate_similarity(&normalized, &item_normalized);
            if score > 0.8 {
                if let Ok(Some(cached)) = state.storage.get_douban_subject_meta(item.id) {
                    log::info!("[search_douban_subject_by_keyword] Cache hit for douban_id={}", item.id);
                    if let Ok(json) = serde_json::to_string(&cached) {
                        let _ = state.storage.set_douban_search_cache(&keyword, &json);
                    }
                    return Ok(Some(cached));
                }
                log::info!("[search_douban_subject_by_keyword] Found in hot list, scraping douban_id={}", item.id);
                let meta = DoubanSubjectScraper::scrape(&app, item.id).await;
                match meta {
                    Ok(m) => {
                        let _ = state.storage.upsert_douban_subject_meta(&m);
                        if let Ok(json) = serde_json::to_string(&m) {
                            let _ = state.storage.set_douban_search_cache(&keyword, &json);
                        }
                        return Ok(Some(m));
                    }
                    Err(e) => log::warn!("Failed to scrape hot list item {}: {}", item.id, e),
                }
            }
        }
    }

    // Step 2: Search Douban via WebView (handles anti-scraping JS challenges)
    log::info!("[search_douban_subject_by_keyword] Using WebView to search Douban for keyword: {}", keyword);
    if let Some(meta) = search_and_scrape_douban(&app, &keyword, &state.storage).await {
        log::info!("[search_douban_subject_by_keyword] Found result via WebView search for: {}", keyword);
        return Ok(Some(meta));
    }

    log::info!("[search_douban_subject_by_keyword] No matching Douban subject found for keyword: {}", keyword);
    Ok(None)
}

/// Shared logic: search Douban via WebView and return the first successful scrape result.
/// Returns the meta if found, None otherwise. On success, also writes to douban_search_cache.
async fn search_and_scrape_douban(
    app: &AppHandle,
    keyword: &str,
    storage: &crate::services::Storage,
) -> Option<DoubanSubjectMeta> {
    let found_ids = match DoubanSubjectScraper::search_subject_ids(app, keyword).await {
        Ok(ids) => ids,
        Err(e) => {
            log::warn!("[search_and_scrape] WebView search failed for '{}': {}", keyword, e);
            return None;
        }
    };
    for douban_id in found_ids {
        log::info!("[search_and_scrape] Found douban_id={} via WebView search for '{}'", douban_id, keyword);
        if let Ok(Some(cached)) = storage.get_douban_subject_meta(douban_id) {
            log::info!("[search_and_scrape] Cache hit for douban_id={}", douban_id);
            if let Ok(json) = serde_json::to_string(&cached) {
                let _ = storage.set_douban_search_cache(keyword, &json);
            }
            return Some(cached);
        }
        match DoubanSubjectScraper::scrape(app, douban_id).await {
            Ok(m) => {
                let _ = storage.upsert_douban_subject_meta(&m);
                if let Ok(json) = serde_json::to_string(&m) {
                    let _ = storage.set_douban_search_cache(keyword, &json);
                }
                return Some(m);
            }
            Err(e) => {
                log::warn!("[search_and_scrape] Scrape failed for {}: {}", douban_id, e);
                continue;
            }
        }
    }
    None
}

/// Helper: refresh douban search cache by re-scraping via WebView (used for background refresh)
async fn refresh_douban_search_cache(app: &AppHandle, keyword: &str, storage: &crate::services::Storage) {
    search_and_scrape_douban(app, keyword, storage).await;
}