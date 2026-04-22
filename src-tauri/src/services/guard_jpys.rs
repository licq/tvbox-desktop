use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardListItem {
    pub item_id: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
}

pub fn parse_jpys_list_payload(
    _site_key: &str,
    item_type: &str,
    payload: &str,
) -> Result<Vec<GuardListItem>, String> {
    let root: Value = serde_json::from_str(payload).map_err(|error| error.to_string())?;
    let list = root
        .get("list")
        .and_then(Value::as_array)
        .ok_or_else(|| "jpys list payload missing list".to_string())?;

    Ok(list
        .iter()
        .filter_map(|entry| {
            Some(GuardListItem {
                item_id: entry.get("vod_id")?.as_str()?.to_string(),
                title: entry.get("vod_name")?.as_str()?.to_string(),
                item_type: item_type.to_string(),
                poster: entry
                    .get("vod_pic")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                summary: entry
                    .get("vod_content")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .or_else(|| {
                        entry
                            .get("type_name")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    }),
            })
        })
        .collect())
}

pub fn parse_jpys_detail_payload(
    site_key: &str,
    item_id: &str,
    payload: &str,
) -> Option<ScrapedCatalogItem> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let entry = root.get("list")?.as_array()?.first()?;
    let title = entry.get("vod_name")?.as_str()?.to_string();
    let poster = entry
        .get("vod_pic")
        .and_then(Value::as_str)
        .map(str::to_string);
    let summary = entry
        .get("vod_content")
        .and_then(Value::as_str)
        .map(str::to_string);
    let play_from = entry.get("vod_play_from")?.as_str()?;
    let play_url = entry.get("vod_play_url")?.as_str()?;

    let sources: Vec<&str> = play_from.split("$$$").collect();
    let groups: Vec<&str> = play_url.split("$$$").collect();
    let mut episodes = Vec::new();

    for (source_index, source_name) in sources.iter().enumerate() {
        let group = groups.get(source_index).copied().unwrap_or_default();
        for episode in group.split('#') {
            let mut parts = episode.split('$');
            let Some(label) = parts
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let Some(encoded) = parts
                .next()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };
            let mut ids = encoded.split('-');
            let Some(source_id) = ids.next().filter(|value| !value.is_empty()) else {
                continue;
            };
            let Some(episode_id) = ids.next().filter(|value| !value.is_empty()) else {
                continue;
            };

            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.trim().to_string(),
                episode_label: label.to_string(),
                play_url: super::encode_guard_play_target(
                    "csp_JpysGuard",
                    site_key,
                    item_id,
                    source_id,
                    episode_id,
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
            r#"{{"source":"guard","guard_key":"csp_JpysGuard","site_key":"{}","item_id":"{}","item_type":"movie"}}"#,
            site_key, item_id
        )),
        episodes,
    })
}

pub fn parse_jpys_play_payload(payload: &str) -> Option<String> {
    let root: Value = serde_json::from_str(payload).ok()?;
    root.get("url").and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{parse_jpys_detail_payload, parse_jpys_list_payload, parse_jpys_play_payload};

    #[test]
    fn parses_jpys_category_list() {
        let payload = r#"{
          "list":[
            {"vod_id":"1419","vod_name":"复仇双雄","vod_pic":"https://img.example.com/a.jpg","type_name":"动作"}
          ]
        }"#;

        let items = parse_jpys_list_payload("文采", "movie", payload).expect("list should parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "复仇双雄");
        assert_eq!(items[0].item_id, "1419");
    }

    #[test]
    fn parses_jpys_detail_payload() {
        let payload = r#"{
          "list":[
            {
              "vod_id":"1419",
              "vod_name":"复仇双雄",
              "vod_pic":"https://img.example.com/a.jpg",
              "vod_content":"剧情简介",
              "vod_play_from":"线路A$$$线路B",
              "vod_play_url":"正片$1-1-1#预告$1-1-2$$$正片$1-2-1"
            }
          ]
        }"#;

        let detail =
            parse_jpys_detail_payload("文采", "1419", payload).expect("detail should parse");
        assert_eq!(detail.title, "复仇双雄");
        assert_eq!(detail.episodes.len(), 3);
        assert!(detail.episodes[0].play_url.starts_with("guard://"));
    }

    #[test]
    fn parses_jpys_play_payload() {
        let payload = r#"{"url":"https://media.example.com/demo/index.m3u8"}"#;
        let resolved = parse_jpys_play_payload(payload).expect("play payload should parse");
        assert_eq!(resolved, "https://media.example.com/demo/index.m3u8");
    }
}
