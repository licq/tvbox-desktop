use async_trait::async_trait;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

/// zxzj native scraper for https://www.zxzjys.com/
pub struct ZxzjScraper {
    base: NativeScraper,
}

impl ZxzjScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("zxzj", "🍊在线┃秒播", "https://www.zxzjys.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // URL format: /vodsearch/keyword-------------.html
        let url = format!("{}/vodsearch/{}-------------.html", self.base.base_url, keyword);
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
        let url = format!("{}/voddetail/{}.html", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        let item = self.parse_detail_page(&body, ids)?;
        Ok(item)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        // zxzj play pages are resolved by resolver.rs
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "zxzj".to_string(),
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

        // zxzj search results have links like /voddetail/1778.html
        for line in body.lines() {
            let line = line.trim();
            if line.contains("/voddetail/") {
                if let Some(title) = self.extract_title_from_line(line) {
                    if let Some(id) = self.extract_id_from_voddetail_line(line) {
                        items.push(ScrapedCatalogItem {
                            source_item_key: id,
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
        }

        Ok(items)
    }

    fn extract_id_from_voddetail_line(&self, line: &str) -> Option<String> {
        // Extract id from /voddetail/1234.html
        if let Some(pos) = line.find("/voddetail/") {
            let remaining = &line[pos + 11..]; // skip "/voddetail/" (11 chars)
            if let Some(end_pos) = remaining.find(".html") {
                let id = &remaining[..end_pos];
                if !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) {
                    return Some(id.to_string());
                }
            }
        }
        None
    }

    fn parse_home_results(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // Similar to parse_search_results but for homepage
        self.parse_search_results(body)
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // Extract episode list from detail page
        let mut episodes = Vec::new();
        let mut order_index = 0i64;

        // Search for all /vodplay/ patterns in the body
        // Since HTML may be on one line, we search the entire body
        let mut search_start = 0;
        while let Some(pos) = body[search_start..].find("/vodplay/") {
            let absolute_pos = search_start + pos;
            let remaining = &body[absolute_pos..];

            // Extract URL: find the closing quote after /vodplay/XXX-XXX-X.html"
            if let Some(close_quote) = remaining.find('"') {
                let url_with_quote = &remaining[..close_quote];
                let play_url = format!("https://www.zxzjys.com{}", url_with_quote);

                // Extract label: text after the > before </a>
                let after_url = &remaining[close_quote + 1..];
                if let Some(gt_pos) = after_url.find('>') {
                    let text_start = &after_url[gt_pos + 1..];
                    if let Some(lt_pos) = text_start.find('<') {
                        let label = &text_start[..lt_pos];
                        let label = label.trim();
                        if !label.is_empty() && label.len() < 50 {
                            episodes.push(ScrapedCatalogEpisode {
                                source_name: "zxzj".to_string(),
                                episode_label: label.to_string(),
                                play_url,
                                order_index,
                            });
                            order_index += 1;
                        }
                    }
                }

                search_start = absolute_pos + 1;
            } else {
                break;
            }
        }

        if episodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(ScrapedCatalogItem {
            source_item_key: ids.to_string(),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        }))
    }

    fn extract_title_from_line(&self, line: &str) -> Option<String> {
        // Search result title is in title="..." attribute
        // e.g. title="蜡笔小新：爆盛！功夫男孩〜拉面大乱〜"
        if let Some(title_start) = line.find("title=\"") {
            let remaining = &line[title_start + 7..];
            if let Some(title_end) = remaining.find('"') {
                let title = &remaining[..title_end];
                let title = title.trim();
                if !title.is_empty() && title.len() < 200 {
                    return Some(title.to_string());
                }
            }
        }
        None
    }

}



impl Default for ZxzjScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for ZxzjScraper {
    fn source_key(&self) -> &str { &self.base.site_key }
    fn source_name(&self) -> &str { &self.base.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.home().await
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        Ok(vec![]) // No categories implemented yet
    }

    async fn category(&self, _type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        Ok(vec![]) // Not implemented
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
    async fn test_search_then_detail_then_play() {
        let scraper = ZxzjScraper::new();
        test_scraper(&scraper, "zxzj", TEST_KEYWORD).await
            .expect("zxzj test failed");
    }
}
