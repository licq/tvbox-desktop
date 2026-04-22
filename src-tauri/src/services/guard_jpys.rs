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
    let item_type = entry
        .get("type_name")
        .and_then(Value::as_str)
        .map(infer_item_type)
        .unwrap_or("movie");
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
            let Some((source_id, episode_id)) = split_guard_episode_token(encoded) else {
                continue;
            };

            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.trim().to_string(),
                episode_label: label.to_string(),
                play_url: super::encode_guard_play_target(
                    "csp_JpysGuard",
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
        item_type: item_type.to_string(),
        poster,
        summary,
        detail_json: Some(format!(
            r#"{{"source":"guard","guard_key":"csp_JpysGuard","site_key":"{}","item_id":"{}","item_type":"{}"}}"#,
            site_key, item_id, item_type
        )),
        episodes,
    })
}

pub fn parse_jpys_play_payload(payload: &str) -> Option<String> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let url = root.get("url").and_then(Value::as_str)?.trim();
    if !is_playable_media_url(url) {
        return None;
    }

    Some(url.to_string())
}

fn split_guard_episode_token(token: &str) -> Option<(String, String)> {
    let mut segments = token
        .split('-')
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let source_id = segments.next()?.to_string();
    let remainder: Vec<_> = segments.collect();
    if remainder.is_empty() {
        return None;
    }

    Some((source_id, remainder.join("-")))
}

fn infer_item_type(type_name: &str) -> &'static str {
    if ["剧", "连续剧", "电视剧", "短剧"]
        .iter()
        .any(|needle| type_name.contains(needle))
    {
        "series"
    } else if ["综艺", "真人秀", "脱口秀"]
        .iter()
        .any(|needle| type_name.contains(needle))
    {
        "variety"
    } else if ["动漫", "动画", "番剧"]
        .iter()
        .any(|needle| type_name.contains(needle))
    {
        "anime"
    } else {
        "movie"
    }
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
    use super::{parse_jpys_detail_payload, parse_jpys_list_payload, parse_jpys_play_payload};
    use crate::services::guard::decode_guard_play_target;

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
        assert_ne!(detail.episodes[0].play_url, detail.episodes[1].play_url);

        let first = decode_guard_play_target(&detail.episodes[0].play_url).expect("decode first");
        let second = decode_guard_play_target(&detail.episodes[1].play_url).expect("decode second");
        assert_eq!(first.source_id, "1");
        assert_eq!(first.episode_id, "1-1");
        assert_eq!(second.source_id, "1");
        assert_eq!(second.episode_id, "1-2");
    }

    #[test]
    fn infers_non_movie_item_type_for_detail_payload() {
        let payload = r#"{
          "list":[
            {
              "vod_id":"2001",
              "vod_name":"厨房秘事",
              "vod_pic":"https://img.example.com/series.jpg",
              "vod_content":"剧集简介",
              "type_name":"电视剧",
              "vod_play_from":"线路A",
              "vod_play_url":"第1集$2-1-1"
            }
          ]
        }"#;

        let detail =
            parse_jpys_detail_payload("文采", "2001", payload).expect("detail should parse");
        assert_eq!(detail.item_type, "series");
        assert_eq!(
            detail.detail_json.as_deref(),
            Some(
                r#"{"source":"guard","guard_key":"csp_JpysGuard","site_key":"文采","item_id":"2001","item_type":"series"}"#
            )
        );
    }

    #[test]
    fn parses_jpys_play_payload_for_playable_media() {
        let payload = r#"{"url":"https://media.example.com/demo/index.m3u8"}"#;
        let resolved = parse_jpys_play_payload(payload).expect("play payload should parse");
        assert_eq!(resolved, "https://media.example.com/demo/index.m3u8");
    }

    #[test]
    fn rejects_non_playable_or_external_required_jpys_play_payloads() {
        let external = r#"{"url":"https://pan.baidu.com/s/1-demo"}"#;
        let html_page = r#"{"url":"https://media.example.com/player/share?id=42"}"#;

        assert_eq!(parse_jpys_play_payload(external), None);
        assert_eq!(parse_jpys_play_payload(html_page), None);
    }
}
