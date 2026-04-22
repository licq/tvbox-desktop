use crate::services::guard::encode_guard_play_target;
use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use serde_json::Value;

pub fn parse_jpj_list_payload(
    _site_key: &str,
    item_type: &str,
    payload: &str,
) -> Result<Vec<GuardListItem>, String> {
    let root: Value = serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let list = root
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| "jpj list payload missing data array".to_string())?;

    Ok(list
        .iter()
        .filter_map(|entry| {
            Some(GuardListItem {
                item_id: string_field(entry, "id")?.to_string(),
                title: string_field(entry, "title")?.to_string(),
                item_type: item_type.to_string(),
                poster: optional_string_field(entry, "cover"),
                summary: optional_string_field(entry, "intro"),
            })
        })
        .collect())
}

pub fn parse_jpj_detail_payload(
    site_key: &str,
    item_id: &str,
    payload: &str,
) -> Option<ScrapedCatalogItem> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let entry = root.get("data")?;
    let title = string_field(entry, "title")?.to_string();
    let poster = optional_string_field(entry, "cover");
    let summary = optional_string_field(entry, "intro");
    let mut episodes = Vec::new();

    for source in entry.get("play_sources")?.as_array()? {
        let Some(source_name) = string_field(source, "name").map(str::to_string) else {
            continue;
        };
        let Some(source_id) = string_field(source, "id").map(str::to_string) else {
            continue;
        };
        let Some(entries) = source.get("episodes").and_then(Value::as_array) else {
            continue;
        };

        for episode in entries {
            let Some(episode_id) = string_field(episode, "id").map(str::to_string) else {
                continue;
            };
            let Some(episode_label) = string_field(episode, "name").map(str::to_string) else {
                continue;
            };
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label,
                play_url: encode_guard_play_target(
                    "csp_JPJGuard",
                    site_key,
                    item_id,
                    &source_id,
                    &episode_id,
                ),
                order_index: episodes.len() as i64,
            });
        }
    }

    Some(ScrapedCatalogItem {
        source_item_key: format!("guard:{}:{}", site_key, item_id),
        title,
        item_type: "movie".to_string(),
        poster,
        summary,
        detail_json: Some(format!(
            r#"{{"source":"guard","guard_key":"csp_JPJGuard","site_key":"{}","item_id":"{}","item_type":"movie"}}"#,
            site_key, item_id
        )),
        episodes,
    })
}

pub fn parse_jpj_play_payload(payload: &str) -> Option<String> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let url = root
        .get("data")
        .and_then(|value| value.get("url"))
        .and_then(Value::as_str)?
        .trim();
    if !is_playable_media_url(url) {
        return None;
    }

    Some(url.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardListItem {
    pub item_id: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty())
}

fn optional_string_field(value: &Value, key: &str) -> Option<String> {
    string_field(value, key).map(str::to_string)
}

fn is_playable_media_url(url: &str) -> bool {
    let normalized = url.trim().to_lowercase();
    if !(normalized.starts_with("http://") || normalized.starts_with("https://")) {
        return false;
    }
    if [
        "pan.baidu.com",
        "drive.uc.cn",
        "pan.quark.cn",
        "aliyundrive.com",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
    {
        return false;
    }

    normalized.contains(".m3u8") || normalized.contains(".mp4")
}

#[cfg(test)]
mod tests {
    use super::{parse_jpj_detail_payload, parse_jpj_list_payload, parse_jpj_play_payload};

    #[test]
    fn parses_jpj_category_list() {
        let payload = r#"{
          "data":[
            {"id":"71483","title":"龙之家族 第二季","cover":"https://img.example.com/b.jpg"}
          ]
        }"#;

        let items = parse_jpj_list_payload("贱贱", "series", payload).expect("list should parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, "71483");
        assert_eq!(items[0].title, "龙之家族 第二季");
    }

    #[test]
    fn parses_jpj_detail_payload() {
        let payload = r#"{
          "data":{
            "id":"71483",
            "title":"龙之家族 第二季",
            "cover":"https://img.example.com/b.jpg",
            "intro":"剧情简介",
            "play_sources":[
              {"id":"1","name":"荐片A","episodes":[{"id":"1","name":"第01集"},{"id":"2","name":"第02集"}]}
            ]
          }
        }"#;

        let detail = parse_jpj_detail_payload("贱贱", "71483", payload).expect("detail should parse");
        assert_eq!(detail.title, "龙之家族 第二季");
        assert_eq!(detail.episodes.len(), 2);
        assert!(detail.episodes[0].play_url.starts_with("guard://"));
    }

    #[test]
    fn parses_jpj_play_payload() {
        let payload = r#"{"data":{"url":"https://media.example.com/demo.mp4"}}"#;
        let resolved = parse_jpj_play_payload(payload).expect("play payload should parse");
        assert_eq!(resolved, "https://media.example.com/demo.mp4");
    }
}
