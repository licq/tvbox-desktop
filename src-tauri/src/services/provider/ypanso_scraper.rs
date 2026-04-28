use async_trait::async_trait;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;
use regex::Regex;

/// Infer item type from the pic-text badge shown on search-result thumbnails.
/// Patterns like "更新第22集", "全37集" are strong series signals.
fn infer_type_from_pic_text(pic_text: &str) -> String {
    if pic_text.contains("集") {
        return "series".to_string();
    }
    // Default to movie when no strong series signal is present.
    // ("HD", "HD中字", "完结", "已完结" appear on both movies and series.)
    "movie".to_string()
}

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
        // Fetch the play page and extract the real video source from the player_aaaa JSON
        let body = self.base.fetch_text(play_url).await?;
        let video_url = extract_ypanso_player_url(&body)
            .unwrap_or_else(|| play_url.to_string());
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "YpanSo".to_string(),
            target_url: video_url,
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
                                // Extract pic-text from the <li> block that contains this item.
                                // Look backwards from current position for the nearest <li start,
                                // then forwards for pic-text within that block.
                                let pic_text = Self::extract_pic_text_around(
                                    section_content, abs_pos
                                );
                                let item_type = infer_type_from_pic_text(&pic_text);
                                items.push(ScrapedCatalogItem {
                                    source_item_key: id,
                                    title: title.to_string(),
                                    item_type,
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

    /// Extract the pic-text label from the <li> block that contains `pos`.
    /// Searches backwards to the nearest `<li` start, then forwards for
    /// `<span class="pic-text text-right">TEXT</span>`.
    fn extract_pic_text_around(section_content: &str, pos: usize) -> String {
        let block_start = section_content[..pos].rfind("<li").unwrap_or(0);
        let block = &section_content[block_start..];
        if let Some(start) = block.find("pic-text text-right\"") {
            let after = &block[start + 20..];
            if let Some(gt) = after.find('>') {
                let after_gt = &after[gt + 1..];
                if let Some(lt) = after_gt.find('<') {
                    return after_gt[..lt].trim().to_string();
                }
            }
        }
        String::new()
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

/// Extract the actual video URL from a ypanso play page.
/// Ypanso uses maccms (Apple CMS) which embeds a `player_aaaa` JSON object
/// with the real video source URL in a `url` field.
fn extract_ypanso_player_url(body: &str) -> Option<String> {
    // (?s) enables dotall mode so `.` matches newlines – the player_aaaa JSON may span multiple lines.
    // Use the same relaxed pattern as resolver::extract_maccms_player_url to handle:
    // - spaces around the equals sign (player_aaaa = {...})
    // - other player variable names like player_bbbb used by some maccms themes
    let player_regex = Regex::new(r"(?s)player_[a-z]{4}\s*=\s*(\{.*?\})</script>").ok()?;
    player_regex.captures(body).and_then(|captures| {
        let json_str = captures.get(1).map(|m| m.as_str())?;
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        parsed.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())
    })
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

    #[test]
    fn extracts_player_url_from_multiline_json() {
        // player_aaaa JSON spanning multiple lines (common in formatted maccms pages)
        let html = r#"<script>var player_aaaa={
            "url": "https://cdn.example.com/video.m3u8",
            "name": "高清线路",
            "encrypt": 0
        }</script>"#;
        assert_eq!(
            extract_ypanso_player_url(html).as_deref(),
            Some("https://cdn.example.com/video.m3u8")
        );
    }

    #[test]
    fn extracts_player_url_from_single_line_json() {
        let html = r#"<script>var player_aaaa={"url":"https://cdn.example.com/video.mp4","name":"标清"}</script>"#;
        assert_eq!(
            extract_ypanso_player_url(html).as_deref(),
            Some("https://cdn.example.com/video.mp4")
        );
    }

    #[test]
    fn returns_none_when_player_aaaa_missing() {
        let html = r#"<script>var other_player={"url":"https://cdn.example.com/video.m3u8"}</script>"#;
        assert!(extract_ypanso_player_url(html).is_none());
    }

    #[test]
    fn returns_none_when_url_field_missing() {
        let html = r#"<script>var player_aaaa={"name":"高清线路","encrypt":0}</script>"#;
        assert!(extract_ypanso_player_url(html).is_none());
    }

    #[test]
    fn extracts_player_url_with_spaces_around_equals() {
        // Some maccms themes format the assignment with spaces
        let html = r#"<script>var player_aaaa = {"url":"https://cdn.example.com/video.m3u8","name":"高清"}</script>"#;
        assert_eq!(
            extract_ypanso_player_url(html).as_deref(),
            Some("https://cdn.example.com/video.m3u8")
        );
    }

    #[test]
    fn extracts_player_url_from_alternate_player_variable() {
        // Some maccms themes use player_bbbb instead of player_aaaa
        let html = r#"<script>var player_bbbb={"url":"https://cdn.example.com/video.mp4","name":"标清"}</script>"#;
        assert_eq!(
            extract_ypanso_player_url(html).as_deref(),
            Some("https://cdn.example.com/video.mp4")
        );
    }
}
