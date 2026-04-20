use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize, Serialize)]
pub struct SubscriptionJson {
    #[serde(rename = "lives")]
    pub lives: Option<Vec<LiveChannelJson>>,
    #[serde(rename = "vods")]
    pub vods: Option<Vec<VodItemJson>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LiveChannelJson {
    pub name: String,
    pub logo: Option<String>,
    pub url: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VodItemJson {
    pub name: String,
    #[serde(rename = "type")]
    pub vtype: Option<String>,
    pub poster: Option<String>,
    pub description: Option<String>,
    pub episodes: Option<Vec<EpisodeJson>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EpisodeJson {
    pub name: String,
    pub url: String,
}

pub struct Parser;

impl Parser {
    pub fn detect_source_kind(content: &str) -> &'static str {
        if let Ok(value) = serde_json::from_str::<Value>(content) {
            if let Some(obj) = value.as_object() {
                if obj.get("sites").is_some() || obj.get("parses").is_some() {
                    return "tvbox_config";
                }

                if obj.get("lives").is_some() && has_tvbox_top_level_markers(obj) {
                    return "tvbox_config";
                }
            }
        }

        "simple_json"
    }

    pub fn parse_subscription(content: &str) -> Result<SubscriptionJson, String> {
        let parsed: SubscriptionJson =
            serde_json::from_str(content).map_err(|e| format!("JSON解析失败: {}", e))?;

        if has_meaningful_subscription_payload(&parsed) {
            Ok(parsed)
        } else {
            Err("订阅内容缺少有效的 lives 或 vods 数据".to_string())
        }
    }

    pub fn parse_episodes(episodes: Option<Vec<EpisodeJson>>) -> String {
        match episodes {
            Some(eps) => serde_json::to_string(&eps).unwrap_or_else(|_| "[]".to_string()),
            None => "[]".to_string(),
        }
    }
}

fn has_meaningful_subscription_payload(parsed: &SubscriptionJson) -> bool {
    parsed.lives.as_ref().is_some_and(|items| !items.is_empty())
        || parsed.vods.as_ref().is_some_and(|items| !items.is_empty())
}

fn has_tvbox_top_level_markers(obj: &serde_json::Map<String, Value>) -> bool {
    const TVBOX_HINT_KEYS: &[&str] = &[
        "spider",
        "wallpaper",
        "logo",
        "ijk",
        "ads",
        "flags",
        "rules",
        "drives",
        "sites",
        "parses",
        "homepage",
        "hotSearch",
        "recommend",
        "recommendSites",
        "player",
    ];

    obj.keys()
        .any(|key| TVBOX_HINT_KEYS.contains(&key.as_str()))
}

#[cfg(test)]
mod tests {
    use super::Parser;

    #[test]
    fn detects_simple_json_subscription() {
        let input = r#"{"lives":[{"name":"CCTV-1","url":"https://a.example/live.m3u8"}]}"#;
        assert_eq!(Parser::detect_source_kind(input), "simple_json");
    }

    #[test]
    fn detects_tvbox_subscription() {
        let input = r#"{"sites":[{"key":"site-a","name":"线路A","api":"https://x.example/api.php/provide/vod/"}],"lives":[{"name":"直播","url":"https://live.example/list.txt"}]}"#;
        assert_eq!(Parser::detect_source_kind(input), "tvbox_config");
    }

    #[test]
    fn detects_lives_only_tvbox_subscription_when_tvbox_markers_exist() {
        let input = r#"{
            "lives":[{"name":"直播","url":"https://live.example/list.txt"}],
            "spider":"./jar/custom.jar"
        }"#;
        assert_eq!(Parser::detect_source_kind(input), "tvbox_config");
    }

    #[test]
    fn keeps_simple_json_with_extra_non_tvbox_top_level_field() {
        let input = r#"{
            "name":"demo",
            "lives":[{"name":"CCTV-1","url":"https://a.example/live.m3u8"}]
        }"#;
        assert_eq!(Parser::detect_source_kind(input), "simple_json");
    }

    #[test]
    fn rejects_simple_json_without_meaningful_payload() {
        let input = r#"{}"#;
        assert!(Parser::parse_subscription(input).is_err());
    }

    #[test]
    fn rejects_simple_json_error_payload() {
        let input = r#"{"error":"upstream failed"}"#;
        assert!(Parser::parse_subscription(input).is_err());
    }
}
