use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub kind: String,
    pub enabled: bool,
    pub last_refreshed_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub type SourceSubscription = Subscription;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannel {
    pub id: i64,
    pub subscription_id: i64,
    pub name: String,
    pub logo: Option<String>,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VodItem {
    pub id: i64,
    pub subscription_id: i64,
    pub name: String,
    pub vtype: String,
    pub poster: Option<String>,
    pub description: Option<String>,
    pub episodes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayHistory {
    pub id: i64,
    pub item_type: String,
    pub item_id: i64,
    pub progress: f64,
    pub last_played: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSubscription {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSite {
    pub id: i64,
    pub subscription_id: i64,
    pub site_key: String,
    pub site_name: String,
    pub api: Option<String>,
    pub ext: Option<String>,
    pub searchable: bool,
    pub quick_search: bool,
    pub filterable: bool,
    pub source_type: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanHot {
    pub id: i64,              // 豆瓣 subject ID
    pub name: String,
    pub year: Option<i32>,
    pub poster: Option<String>,
    pub rating: Option<f64>,
    pub rank: i32,
    pub updated_at: String,
    pub item_type: String,    // 新增: "movie" | "series" | "variety" | "anime"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanHotItem {    // 单个搜索结果项
    pub source: String,        // "zxzj" | "jpvod" | "xb6v"
    pub source_name: String,
    pub detail_url: String,
    pub item_type: String,   // "movie" | "series" | "variety" | "anime" | "generic"
    pub title: Option<String>,
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<DoubanHotItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSource {
    pub url: String,
    pub subscription_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedLiveChannel {
    pub id: i64,
    pub name: String,
    pub logo: Option<String>,
    pub category: Option<String>,
    pub sources: Vec<ChannelSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannelGroupItem {
    pub name: String,
    pub source_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannelGroup {
    pub category: String,
    pub channels: Vec<LiveChannelGroupItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeCatalogItem {
    pub id: i64,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub progress: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomePayload {
    pub continue_watching: Vec<HomeCatalogItem>,
    pub latest_updates: Vec<HomeCatalogItem>,
    pub featured: Vec<HomeCatalogItem>,
    pub douban_hot: Vec<DoubanHot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogDetailItem {
    pub id: i64,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
    pub detail_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEpisode {
    pub id: i64,
    pub episode_label: String,
    pub play_url: String,
    pub order_index: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEpisodeGroup {
    pub source_name: String,
    pub episodes: Vec<CatalogEpisode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogDetail {
    pub item: CatalogDetailItem,
    pub episode_groups: Vec<CatalogEpisodeGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCandidate {
    pub url: String,
    pub label: String,
    pub kind: String,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedPlayback {
    pub status: String,
    pub candidates: Vec<PlaybackCandidate>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub subscription_name: String,
    pub live_count: i32,
    pub movie_count: i32,
    pub series_count: i32,
    pub variety_count: i32,
    pub anime_count: i32,
    pub other_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoubanSubjectMeta {
    pub douban_id: i64,
    pub title: String,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub director: Vec<String>,
    pub writer: Vec<String>,
    pub actors: Vec<String>,
    pub genre: Vec<String>,
    pub country: Vec<String>,
    pub language: Vec<String>,
    pub release_date: Vec<String>,
    pub runtime: Option<String>,
    pub summary: Option<String>,
    pub poster: Option<String>,
}
