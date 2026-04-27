use async_trait::async_trait;
use std::sync::Arc;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

/// libvio native scraper for https://www.libvio.com/
/// Uses direct m3u8/MP4 URLs for playback.
pub struct LibvioScraper {
    base: NativeScraper,
}

impl LibvioScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("libvio", "🌟立播┃秒播", "https://www.libvio.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/search?wd={}", self.base.base_url, keyword);
        let body = self.base.fetch_text(&url).await?;

        let items = self.parse_search_results(&body)?;
        Ok(items)
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        let items = self.parse_home_results(&body)?;
        Ok(items)
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/detail/{}", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        let item = self.parse_detail_page(&body, ids)?;
        Ok(item)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        // libvio uses direct m3u8/MP4 URLs
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "libvio".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }])
    }

    /// Parse search results from HTML.
    /// Returns empty vec if HTML structure can't be determined.
    fn parse_search_results(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let mut items = Vec::new();

        // Try to find patterns like: <a href="/detail/xxx">title</a>
        for line in body.lines() {
            let line = line.trim();
            if line.contains("/detail/") {
                if let Some(title) = self.extract_title_from_line(line) {
                    let source_item_key = format!("libvio-{}", items.len());
                    items.push(ScrapedCatalogItem {
                        source_item_key,
                        title,
                        item_type: "movie".to_string(),
                        poster: None,
                        summary: None,
                        detail_json: None,
                        episodes: vec![],
                    });
                }
            }
        }

        Ok(items)
    }

    fn parse_home_results(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // Similar to parse_search_results but for homepage
        self.parse_search_results(body)
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // Extract episode list from detail page
        let mut episodes = Vec::new();
        let mut order_index = 0i64;

        for line in body.lines() {
            let line = line.trim();
            if line.contains("/play/") || line.contains(".m3u8") || line.contains(".mp4") {
                if let Some(label) = self.extract_episode_label(line) {
                    let play_url = self.extract_play_url(line);
                    if !play_url.is_empty() {
                        episodes.push(ScrapedCatalogEpisode {
                            source_name: "libvio".to_string(),
                            episode_label: label,
                            play_url,
                            order_index,
                        });
                        order_index += 1;
                    }
                }
            }
        }

        if episodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(ScrapedCatalogItem {
            source_item_key: format!("libvio-{}", ids),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        }))
    }

    fn extract_title_from_line(&self, line: &str) -> Option<String> {
        // Try to extract text content near the link
        if let Some(start) = line.find('>') {
            let remaining = &line[start+1..];
            if let Some(end) = remaining.find('<') {
                let text = &remaining[..end];
                let text = text.trim();
                if !text.is_empty() && text.len() < 100 && !text.contains("href") {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn extract_episode_label(&self, line: &str) -> Option<String> {
        self.extract_title_from_line(line)
    }

    fn extract_play_url(&self, line: &str) -> String {
        // Extract the href URL from an anchor tag
        if let Some(href_start) = line.find("href=\"") {
            let remaining = &line[href_start + 6..];
            if let Some(href_end) = remaining.find('"') {
                let url = &remaining[..href_end];
                if url.starts_with('/') {
                    return format!("https://www.libvio.com{}", url);
                }
                return url.to_string();
            }
        }
        // Also check for direct m3u8/mp4 URLs in the line
        if let Some(m3u8_pos) = line.find(".m3u8") {
            let start = if m3u8_pos > 10 { m3u8_pos - 50 } else { 0 };
            let end = (m3u8_pos + 5).min(line.len());
            let candidate = &line[start..end];
            if candidate.starts_with("http") || candidate.starts_with("/") {
                return candidate.to_string();
            }
        }
        if let Some(mp4_pos) = line.find(".mp4") {
            let start = if mp4_pos > 10 { mp4_pos - 50 } else { 0 };
            let end = (mp4_pos + 4).min(line.len());
            let candidate = &line[start..end];
            if candidate.starts_with("http") || candidate.starts_with("/") {
                return candidate.to_string();
            }
        }
        String::new()
    }
}

impl Default for LibvioScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for LibvioScraper {
    fn source_key(&self) -> &str { &self.base.site_key }
    fn source_name(&self) -> &str { &self.base.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.home().await
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        Ok(vec![])
    }

    async fn category(&self, _type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        Ok(vec![])
    }

    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.search(keyword).await
    }

    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        self.detail(ids).await
    }

    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        self.play(flag, play_url).await
    }
}
