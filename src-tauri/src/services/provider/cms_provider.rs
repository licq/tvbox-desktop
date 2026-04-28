// src-tauri/src/services/provider/cms_provider.rs
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use super::{VideoProvider, ProviderError};
use super::traits::CatalogCategory;
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};

pub struct CmsProvider {
    site_key: String,
    site_name: String,
    api_url: String,
    client: Client,
}

impl CmsProvider {
    pub fn new(site_key: String, site_name: String, api_url: String, client: Client) -> Self {
        Self { site_key, site_name, api_url, client }
    }

    /// 构建 CMS API URL
    fn build_url(&self, ac: &str, extra: &[(&str, &str)]) -> String {
        let base = self.api_url.trim_end_matches('/').to_string();
        let sep = if base.contains('?') { "&" } else { "?" };
        let mut url = format!("{}{}ac={}", base, sep, ac);
        for (k, v) in extra {
            url.push_str(&format!("&{}={}", k, v));
        }
        url
    }

    /// 解析 CMS JSON 响应为 ScrapedCatalogItem 列表
    fn parse_cms_list(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let root: Value = serde_json::from_str(body)?;
        // CMS 标准响应格式: { list: [{ vod_id, vod_name, vod_pic, vod_content, type_name, ... }] }
        let list = root.get("list").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::Parse("CMS response missing 'list' array".to_string())
        })?;

        let mut items = Vec::new();
        for item in list {
            let vod_id = item.get("vod_id").and_then(|v| v.as_str()).unwrap_or("");
            let vod_name = item.get("vod_name").and_then(|v| v.as_str()).unwrap_or("");
            if vod_id.is_empty() || vod_name.is_empty() {
                continue;
            }
            let poster = item.get("vod_pic").and_then(|v| v.as_str()).map(|s| s.to_string());
            let summary = item.get("vod_content").and_then(|v| v.as_str()).map(|s| s.to_string());
            let raw_type = item.get("type_name").and_then(|v| v.as_str()).unwrap_or("movie");
            let item_type = crate::services::provider::normalize_item_type(raw_type);

            items.push(ScrapedCatalogItem {
                source_item_key: format!("{}:{}", self.site_key, vod_id),
                title: vod_name.to_string(),
                item_type,
                poster,
                summary,
                detail_json: Some(serde_json::json!({
                    "source": self.site_key,
                    "ids": vod_id,
                }).to_string()),
                episodes: Vec::new(),
            });
        }
        Ok(items)
    }
}

#[async_trait]
impl VideoProvider for CmsProvider {
    fn source_key(&self) -> &str { &self.site_key }
    fn source_name(&self) -> &str { &self.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("list", &[("t", "1"), ("pg", "1")]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        Ok(vec![
            CatalogCategory { type_id: "1".to_string(), type_name: "电影".to_string() },
            CatalogCategory { type_id: "2".to_string(), type_name: "电视剧".to_string() },
            CatalogCategory { type_id: "3".to_string(), type_name: "综艺".to_string() },
            CatalogCategory { type_id: "4".to_string(), type_name: "动漫".to_string() },
        ])
    }

    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("list", &[("t", type_id), ("pg", &page.to_string())]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("videolist", &[("wd", keyword)]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("detail", &[("ids", ids)]);
        let body = self.client.get(&url).send().await?.text().await?;
        let root: Value = serde_json::from_str(&body)?;
        let list = root.get("list").and_then(|v| v.as_array());
        let Some(first) = list.and_then(|l| l.first()) else {
            return Ok(None);
        };

        let vod_id = first.get("vod_id").and_then(|v| v.as_str()).unwrap_or("");
        let vod_name = first.get("vod_name").and_then(|v| v.as_str()).unwrap_or("");
        let vod_actor = first.get("vod_actor").and_then(|v| v.as_str()).unwrap_or("");
        let vod_director = first.get("vod_director").and_then(|v| v.as_str()).unwrap_or("");

        let mut summary_parts = Vec::new();
        if !vod_director.is_empty() { summary_parts.push(format!("导演: {}", vod_director)); }
        if !vod_actor.is_empty() { summary_parts.push(format!("演员: {}", vod_actor)); }

        // 解析剧集
        let mut episodes = Vec::new();
        if let Some(vod_play_url) = first.get("vod_play_url").and_then(|v| v.as_str()) {
            let separator = if vod_play_url.contains("$$$") {
                "$$$"
            } else if vod_play_url.contains('#') {
                "#"
            } else {
                ""
            };

            if !separator.is_empty() {
                for (i, part) in vod_play_url.split(separator).enumerate() {
                    if let Some(dollar_pos) = part.find('$') {
                        let label = &part[..dollar_pos];
                        let url = &part[dollar_pos + 1..];
                        episodes.push(ScrapedCatalogEpisode {
                            source_name: self.site_name.clone(),
                            episode_label: label.to_string(),
                            play_url: url.to_string(),
                            order_index: (i + 1) as i64,
                        });
                    }
                }
            }
        }

        let raw_type = first.get("type_name").and_then(|v| v.as_str()).unwrap_or("movie");
        let item_type = crate::services::provider::normalize_item_type(raw_type);

        Ok(Some(ScrapedCatalogItem {
            source_item_key: format!("{}:{}", self.site_key, vod_id),
            title: vod_name.to_string(),
            item_type,
            poster: first.get("vod_pic").and_then(|v| v.as_str()).map(|s| s.to_string()),
            summary: if summary_parts.is_empty() { None } else { Some(summary_parts.join("\n")) },
            detail_json: Some(serde_json::json!({
                "source": self.site_key,
                "ids": vod_id,
            }).to_string()),
            episodes,
        }))
    }

    async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        let target = PlaybackTarget {
            episode_id: None,
            source_key: self.site_key.clone(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Resolvable,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        };
        Ok(vec![target])
    }
}
