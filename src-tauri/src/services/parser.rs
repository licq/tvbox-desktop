use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SubscriptionJson {
    #[serde(rename = "lives")]
    pub lives: Option<Vec<LiveChannelJson>>,
    #[serde(rename = "vods")]
    pub vods: Option<Vec<VodItemJson>>,
}

#[derive(Debug, Deserialize)]
pub struct LiveChannelJson {
    pub name: String,
    pub logo: Option<String>,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VodItemJson {
    pub name: String,
    #[serde(rename = "type")]
    pub vtype: Option<String>,
    pub poster: Option<String>,
    pub description: Option<String>,
    pub episodes: Option<Vec<EpisodeJson>>,
}

#[derive(Debug, Deserialize)]
pub struct EpisodeJson {
    pub name: String,
    pub url: String,
}

pub struct Parser;

impl Parser {
    pub fn parse_subscription(content: &str) -> Result<SubscriptionJson, String> {
        serde_json::from_str(content).map_err(|e| format!("JSON解析失败: {}", e))
    }

    pub fn parse_episodes(episodes: Option<Vec<EpisodeJson>>) -> String {
        match episodes {
            Some(eps) => serde_json::to_string(&eps).unwrap_or_else(|_| "[]".to_string()),
            None => "[]".to_string(),
        }
    }
}
