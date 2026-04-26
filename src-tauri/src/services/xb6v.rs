use crate::services::playback_runtime::build_runtime_target;
use crate::services::PlaybackTarget;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScrapedCatalogEpisode {
    pub source_name: String,
    pub episode_label: String,
    pub play_url: String,
    pub order_index: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScrapedCatalogItem {
    pub source_item_key: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
    pub detail_json: Option<String>,
    pub episodes: Vec<ScrapedCatalogEpisode>,
}

pub fn runtime_targets_for_item(item: &ScrapedCatalogItem) -> Vec<PlaybackTarget> {
    let source_key = runtime_source_key_for_item(item);
    item.episodes
        .iter()
        .enumerate()
        .map(|(index, episode)| {
            let mut target =
                build_runtime_target(&episode.play_url, &source_key, Some((index + 1) as i64));
            target.sort_hint = episode.order_index as i32;
            target.meta = Some(runtime_target_label(episode));
            target
        })
        .collect()
}

fn runtime_source_key_for_item(item: &ScrapedCatalogItem) -> String {
    let Some(detail_json) = item.detail_json.as_deref() else {
        return "default".to_string();
    };
    let Ok(detail) = serde_json::from_str::<serde_json::Value>(detail_json) else {
        return "default".to_string();
    };
    let source = detail
        .get("source")
        .and_then(|value| value.as_str())
        .unwrap_or("default");

    if source == "guard" {
        return detail
            .get("guard_key")
            .and_then(|value| value.as_str())
            .unwrap_or(source)
            .to_string();
    }

    source.to_string()
}

fn runtime_target_label(episode: &ScrapedCatalogEpisode) -> String {
    let source_name = episode.source_name.trim();
    let episode_label = episode.episode_label.trim();

    if source_name.is_empty() {
        return episode_label.to_string();
    }
    if episode_label.is_empty() || source_name == episode_label {
        return source_name.to_string();
    }

    format!("{source_name}:{episode_label}")
}
