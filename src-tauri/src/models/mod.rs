use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

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
pub struct DoubanHot {
    pub id: i64,
    pub name: String,
    pub year: Option<i32>,
    pub poster: Option<String>,
    pub rating: Option<f64>,
    pub rank: i32,
    pub updated_at: String,
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
