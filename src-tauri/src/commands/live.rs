use crate::AppState;
use crate::models::MergedLiveChannel;
use tauri::State;
use serde::{Deserialize, Serialize};

#[tauri::command]
pub fn get_live_channels(state: State<'_, AppState>, category: Option<String>) -> Result<Vec<MergedLiveChannel>, String> {
    match category {
        Some(cat) => state.storage.get_merged_live_channels_by_category(&cat)
            .map_err(|e| e.to_string()),
        None => state.storage.get_merged_live_channels()
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn get_live_channel_groups(state: State<'_, AppState>) -> Result<Vec<LiveChannelGroup>, String> {
    // Returns categories with channels grouped
    let categories = state.storage.get_live_categories().map_err(|e| e.to_string())?;

    let mut groups = Vec::new();
    for cat in categories {
        // Filter out non-TV categories
        if ["原创IP", "视频源", "手工绘画", "生活杂谈", "一起看", "电子榨菜"].contains(&cat.as_str()) {
            continue;
        }

        let channels = state.storage.get_merged_live_channels_by_category(&cat)
            .map_err(|e| e.to_string())?;

        if !channels.is_empty() {
            groups.push(LiveChannelGroup {
                category: cat,
                channels,
            });
        }
    }

    // Sort by predefined order
    groups.sort_by(|a, b| {
        let order = |c: &str| {
            match c {
                "央视频道" | "央视IPV4" => 0,
                "卫视频道" | "卫视IPV4" => 1,
                "港台" => 2,
                "运动体育" => 3,
                _ => 4,
            }
        };
        order(&a.category).cmp(&order(&b.category))
    });

    Ok(groups)
}

#[tauri::command]
pub fn get_live_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.storage.get_live_categories().map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannelGroup {
    pub category: String,
    pub channels: Vec<MergedLiveChannel>,
}
