use async_trait::async_trait;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

pub struct YicaiScraper {
    base: NativeScraper,
}

impl YicaiScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("溢彩", "💡流光┃秒播", "https://www.yicai.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/search?wd={}", self.base.base_url, keyword);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_search_results(&body))
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_search_results(&body))
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/{}", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_detail_page(&body, ids))
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "溢彩".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Resolvable,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        referer: None,
        }])
    }

    fn parse_search_results(&self, body: &str) -> Vec<ScrapedCatalogItem> {
        let mut items = Vec::new();
        for line in body.lines() {
            let line = line.trim();
            if line.contains("/vod/") || line.contains("/play/") || line.contains("/detail/") {
                if let Some(title) = self.extract_title(line) {
                    items.push(ScrapedCatalogItem {
                        source_item_key: format!("{}-{}", "溢彩", items.len()),
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
        items
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Option<ScrapedCatalogItem> {
        let mut episodes = Vec::new();
        let mut order_index = 0i64;
        for line in body.lines() {
            let line = line.trim();
            if line.contains("/play/") || line.contains("/vod/") {
                if let Some(label) = self.extract_title(line) {
                    let play_url = self.extract_url(line);
                    if !play_url.is_empty() {
                        episodes.push(ScrapedCatalogEpisode {
                            source_name: "溢彩".to_string(),
                            episode_label: label,
                            play_url,
                            order_index,
                        });
                        order_index += 1;
                    }
                }
            }
        }
        if episodes.is_empty() { return None; }
        Some(ScrapedCatalogItem {
            source_item_key: format!("{}-{}", "溢彩", ids),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        })
    }

    fn extract_title(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find('>') {
            let remaining = &line[start+1..];
            if let Some(end) = remaining.find('<') {
                let text = &remaining[..end];
                let text = text.trim();
                if !text.is_empty() && text.len() < 200 && !text.contains("href") && !text.contains("<") {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn extract_url(&self, line: &str) -> String {
        if let Some(href_start) = line.find("href=\"") {
            let remaining = &line[href_start + 6..];
            if let Some(href_end) = remaining.find('"') {
                let url = &remaining[..href_end];
                if url.starts_with('/') {
                    return format!("{}{}", self.base.base_url, url);
                }
                return url.to_string();
            }
        }
        String::new()
    }
}

impl Default for YicaiScraper {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl VideoProvider for YicaiScraper {
    fn source_key(&self) -> &str { &self.base.site_key }
    fn source_name(&self) -> &str { &self.base.site_name }
    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { self.home().await }
    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> { Ok(vec![]) }
    async fn category(&self, _type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { Ok(vec![]) }
    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { self.search(keyword).await }
    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> { self.detail(ids).await }
    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> { self.play(flag, play_url).await }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::provider::scraper_tests::test_scraper;

    const TEST_KEYWORD: &str = "功夫";

    #[tokio::test]
    #[ignore]
    async fn test_search_then_detail_then_play() {
        let scraper = YicaiScraper::new();
        test_scraper(&scraper, "溢彩", TEST_KEYWORD).await
            .expect("溢彩 test failed");
    }
}
