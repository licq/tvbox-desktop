use async_trait::async_trait;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

pub struct YpansoScraper {
    base: NativeScraper,
}

impl YpansoScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("YpanSo", "🐟盘她┃三盘", "https://www.ypanso.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/vod/search.html?wd={}", self.base.base_url, keyword);
        let body = self.base.fetch_text(&url).await?;
        let items = self.parse_search_results(&body);
        Ok(items)
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_search_results(&body))
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/vod/detail/id/{}.html", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_detail_page(&body, ids))
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "YpanSo".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }])
    }

    fn parse_search_results(&self, body: &str) -> Vec<ScrapedCatalogItem> {
        let mut items = Vec::new();

        // Strategy: Find the search results <ul> which is inside <div class="stui-pannel_bd">
        // The sidebar hot list also has stui-pannel_bd but its <ul> comes AFTER the marker
        // without the proper section boundary. We need to find the </ul> that bounds the
        // search results section.
        //
        // Key difference:
        // - Search results: <div class="stui-pannel_bd"><ul class="stui-vodlist__media col-pd clearfix">...</ul></div>
        // - Sidebar: has "active" class in the <ul>

        let search_pattern = "/vod/detail/id/";

        // Find the search results section - look for marker in a proper section context
        let marker = "stui-pannel_bd\"><ul";
        let marker_pos = match body.find(marker) {
            Some(pos) => pos,
            None => return items,
        };

        // The marker "stui-pannel_bd\"><ul" is 19 chars, where "<ul" starts at offset 16
        // marker_pos points to the start of the marker, so <ul starts at marker_pos + 16
        let ul_start = marker_pos + 16;
        let ul_end_marker = "</ul>";
        let section_end = match body[ul_start..].find(ul_end_marker) {
            Some(offset) => ul_start + offset,
            None => return items,
        };

        // Extract content between the ul start and </ul>
        let section_content = &body[ul_start..section_end];

        // If empty (no <li>), return nothing
        // Note: HTML uses <li class="..."> not <li> so check for "<li" instead
        if !section_content.contains("<li") {
            return items;
        }

        // Extract items
        let mut section_pos = 0;
        while let Some(pos) = section_content[section_pos..].find(search_pattern) {
            // pos is relative to section_content[section_pos..]; convert to absolute
            let abs_pos = section_pos + pos;
            let remaining = &section_content[abs_pos..];

            if let Some(html_pos) = remaining.find(".html") {
                let url_part = &remaining[15..html_pos];
                if !url_part.is_empty() && url_part.len() < 50 {
                    let id = url_part.to_string();
                    let after_url = &remaining[html_pos + 5..];
                    if let Some(title_start) = after_url.find("title=\"") {
                        let title_remaining = &after_url[title_start + 7..];
                        if let Some(title_end) = title_remaining.find('"') {
                            let title = &title_remaining[..title_end];
                            if !title.is_empty() && title.len() < 200 {
                                items.push(ScrapedCatalogItem {
                                    source_item_key: id,
                                    title: title.to_string(),
                                    item_type: "movie".to_string(),
                                    poster: None,
                                    summary: None,
                                    detail_json: None,
                                    episodes: vec![],
                                });
                            }
                        }
                    }
                    section_pos = abs_pos + 1;
                } else {
                    section_pos = abs_pos + 1;
                }
            } else {
                section_pos = abs_pos + 1;
            }
        }
        items
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Option<ScrapedCatalogItem> {
        let mut episodes = Vec::new();
        let mut order_index = 0i64;
        // Search for /vod/play/id/ pattern in the body (one-line HTML)
        let mut search_start = 0;
        while let Some(pos) = body[search_start..].find("/vod/play/id/") {
            let absolute_pos = search_start + pos;
            let remaining = &body[absolute_pos..];
            // Extract URL
            if let Some(close_quote) = remaining.find('"') {
                let url_with_quote = &remaining[..close_quote];
                let play_url = format!("https://www.ypanso.com{}", url_with_quote);
                // Extract label - find text after the URL that looks like an episode label
                let after_url = &remaining[close_quote + 1..];
                if let Some(label) = self.extract_episode_label_from_detail(after_url) {
                    episodes.push(ScrapedCatalogEpisode {
                        source_name: "YpanSo".to_string(),
                        episode_label: label,
                        play_url,
                        order_index,
                    });
                    order_index += 1;
                }
                search_start = absolute_pos + 1;
            } else {
                break;
            }
        }
        if episodes.is_empty() { return None; }
        Some(ScrapedCatalogItem {
            source_item_key: ids.to_string(),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        })
    }

    fn extract_episode_label_from_detail(&self, text: &str) -> Option<String> {
        // Look for patterns like "立即播放" or episode text near play button
        // Try to find text between > and </a> near the play URL
        if let Some(gt_pos) = text.find('>') {
            let after_gt = &text[gt_pos + 1..];
            if let Some(lt_pos) = after_gt.find('<') {
                let label = &after_gt[..lt_pos];
                let label = label.trim();
                if !label.is_empty() && label.len() < 50 {
                    return Some(label.to_string());
                }
            }
        }
        None
    }
}

impl Default for YpansoScraper {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl VideoProvider for YpansoScraper {
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

    const TEST_KEYWORD: &str = "危险关系";

    #[tokio::test]
    async fn test_search_then_detail_then_play() {
        let scraper = YpansoScraper::new();
        test_scraper(&scraper, "YpanSo", TEST_KEYWORD).await
            .expect("YpanSo test failed");
    }
}
