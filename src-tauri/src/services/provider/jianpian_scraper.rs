use async_trait::async_trait;
use std::sync::Arc;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

/// jianpian native scraper for https://www.jpj.com/
/// Uses guard://csp_JPJGuard/... URLs for playback that need resolution.
pub struct JianpianScraper {
    base: NativeScraper,
}

impl JianpianScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("jianpian", "🐭荐片┃P2P", "https://www.jpj.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/search?keyword={}", self.base.base_url, keyword);
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
        // jianpian uses guard://csp_JPJGuard/... URLs that need guard resolution
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "jianpian".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Resolvable,
            resolver_key: Some("csp_JPJGuard".to_string()),
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
                    let source_item_key = format!("jianpian-{}", items.len());
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
            if line.contains("/play/") || line.contains("guard://") {
                if let Some(label) = self.extract_episode_label(line) {
                    let play_url = self.extract_play_url(line);
                    if !play_url.is_empty() {
                        episodes.push(ScrapedCatalogEpisode {
                            source_name: "jianpian".to_string(),
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
            source_item_key: format!("jianpian-{}", ids),
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
                    return format!("https://www.jpj.com{}", url);
                }
                if url.starts_with("guard://") {
                    return url.to_string();
                }
                return url.to_string();
            }
        }
        String::new()
    }
}

impl Default for JianpianScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for JianpianScraper {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::provider::scraper_tests::test_scraper;

    const TEST_KEYWORD: &str = "功夫";

    #[tokio::test]
    #[ignore]
    async fn test_search_then_detail_then_play() {
        let scraper = JianpianScraper::new();
        test_scraper(&scraper, "jianpian", TEST_KEYWORD).await
            .expect("jianpian test failed");
    }
}
