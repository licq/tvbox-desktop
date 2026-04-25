use crate::AppState;
use crate::models::DoubanHot;
use crate::models::DoubanHotItem;
use tauri::State;
use serde::{Deserialize, Serialize};

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
    let crawler = crate::services::douban::DoubanCrawler::new();
    let items = crawler.fetch_all().await?;
    state.storage.clear_douban_hot().map_err(|e| e.to_string())?;
    state.storage.upsert_douban_hot(&items).map_err(|e| e.to_string())?;
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
pub async fn get_douban_hot_by_type(
    state: State<'_, AppState>,
    item_type: String,
) -> Result<Vec<DoubanHot>, String> {
    state.storage.get_douban_hot_by_type(&item_type).map_err(|e| e.to_string())
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