use async_trait::async_trait;
use std::sync::Arc;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

/// xb6v native scraper for https://www.xb6v.com/
pub struct Xb6vScraper {
    base: NativeScraper,
}

impl Xb6vScraper {
    pub fn new() -> Self {
        Self {
            base: NativeScraper::new("xb6v", "🧲新6V┃磁力", "https://www.xb6v.com"),
        }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // xb6v uses POST form that redirects to results page
        let body = self.base.post_form_follow_redirect(
            &format!("{}/e/search/1index.php", self.base.base_url),
            &[
                ("keyboard", keyword),
                ("show", "title"),
                ("tempid", "1"),
                ("tbname", "article"),
                ("mid", "1"),
                ("dopost", "search"),
            ],
        ).await?;
        self.parse_search_results(&body)
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        let items = self.parse_home_results(&body)?;
        Ok(items)
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // ids is now like "donghuapian/24219" - convert to URL path
        let url = format!("{}/{}.html", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        let item = self.parse_detail_page(&body, ids)?;
        Ok(item)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        // xb6v play pages are resolved by resolver.rs
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "xb6v".to_string(),
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
        let lines: Vec<&str> = body.lines().collect();
        let mut items = Vec::new();

        let category_patterns = ["dongzuopian", "xijupian", "juqingpian", "aiqingpian",
            "donghuapian", "kongbupian", "kehuanpian", "zhanzhengpian",
            "jilupian", "dianshiju", "ZongYi"];

        let mut item_keys: Vec<(String, usize)> = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            let has_category = category_patterns.iter().any(|cat| line.contains(cat));
            if !has_category {
                continue;
            }
            if let Some(key) = self.extract_key_from_line(line) {
                item_keys.push((key, i));
            }
        }

        for (key, href_line_idx) in item_keys {
            let title = self.find_title_at_or_after(&lines, href_line_idx);
            items.push(ScrapedCatalogItem {
                source_item_key: key.clone(),
                title,
                item_type: "movie".to_string(),
                poster: None,
                summary: None,
                detail_json: None,
                episodes: vec![],
            });
        }

        Ok(items)
    }

    fn line_matches_category(&self, line: &str) -> bool {
        line.contains("/donghuapian/") || line.contains("/dongzuopian/") ||
        line.contains("/xijupian/") || line.contains("/juqingpian/") ||
        line.contains("/aiqingpian/") || line.contains("/kongbupian/") ||
        line.contains("/kehuanpian/") || line.contains("/zhanzhengpian/") ||
        line.contains("/jilupian/") || line.contains("/dianshiju/") ||
        line.contains("/ZongYi/")
    }

    fn extract_key_from_line(&self, line: &str) -> Option<String> {
        let href_start = line.find("href='/")?;
        let remaining = &line[href_start + 7..];
        let html_pos = remaining.find(".html'")?;
        let before_html = &remaining[..html_pos];
        let last_slash = before_html.rfind('/')?;
        let category = &before_html[..last_slash];
        let id = &before_html[last_slash + 1..];
        Some(format!("{}/{}", category, id))
    }

    fn find_title_at_or_after(&self, lines: &[&str], href_line_idx: usize) -> String {
        if let Some(title) = self.extract_title_from_full_line(lines[href_line_idx]) {
            return title;
        }
        if href_line_idx + 1 < lines.len() {
            if let Some(title) = self.extract_title_from_full_line(lines[href_line_idx + 1]) {
                return title;
            }
        }
        if href_line_idx + 2 < lines.len() {
            if let Some(title) = self.extract_title_from_full_line(lines[href_line_idx + 2]) {
                return title;
            }
        }
        "Unknown".to_string()
    }

    fn extract_title_from_full_line(&self, line: &str) -> Option<String> {
        let line = line.trim();
        if !line.contains("</a>") {
            return None;
        }
        if let Some(close_bracket) = line.rfind("</a>") {
            let before_close = &line[..close_bracket];
            if let Some(open_bracket) = before_close.rfind('>') {
                let text = &before_close[open_bracket + 1..];
                let text = text.trim();
                if !text.is_empty() && text.len() < 200 && !text.contains('<') && !text.contains("href") {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn parse_home_results(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.parse_search_results(body)
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // Extract episode list from detail page
        let mut episodes = Vec::new();
        let mut order_index = 0i64;

        for line in body.lines() {
            let line = line.trim();
            if line.contains("/e/DownSys/play/") {
                if let Some(label) = self.extract_episode_label(line) {
                    let play_url = self.extract_play_url(line);
                    if !play_url.is_empty() {
                        episodes.push(ScrapedCatalogEpisode {
                            source_name: "xb6v".to_string(),
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
            source_item_key: format!("xb6v-{}", ids),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        }))
    }

    fn extract_title_from_line(&self, line: &str) -> Option<String> {
        // For lines with href containing .html, the title comes after the closing > of the href
        // e.g. <li><a href='/donghuapian/24219.html'>剑来[第二季全]</a>
        // Look for .html'> pattern and extract title after that >
        if let Some(html_end) = line.find(".html'") {
            let after_html = &line[html_end + 5..]; // skip ".html'"
            if let Some(gt_pos) = after_html.find('>') {
                let text_start = &after_html[gt_pos + 1..];
                if let Some(lt_pos) = text_start.find('<') {
                    let text = &text_start[..lt_pos];
                    let text = text.trim();
                    if !text.is_empty() && text.len() < 200 && !text.contains("href") {
                        return Some(text.to_string());
                    }
                }
            }
        }
        // Fallback for other lines (e.g., play links)
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
        // Extract episode label from play URL line like:
        // <a title='S01E01' href= "/e/DownSys/play/?classid=20&id=24219&pathid1=0&bf=0" ... >S01E01</a>
        // The label is the text between the last > and </a>
        if let Some(close_a) = line.find("</a>") {
            let before_close = &line[..close_a];
            if let Some(last_gt) = before_close.rfind('>') {
                let text = &before_close[last_gt + 1..];
                let text = text.trim();
                if !text.is_empty() && text.len() < 50 && !text.contains('<') {
                    return Some(text.to_string());
                }
            }
        }
        None
    }

    fn extract_play_url(&self, line: &str) -> String {
        // Extract the href URL from an anchor tag
        // Pattern: href= "URL" (note space after =)
        if let Some(href_start) = line.find("href=\"") {
            let remaining = &line[href_start + 6..];
            if let Some(href_end) = remaining.find('"') {
                let url = &remaining[..href_end];
                if url.starts_with('/') {
                    return format!("https://www.xb6v.com{}", url);
                }
                return url.to_string();
            }
        }
        // Try with space: href= "..."
        if let Some(href_start) = line.find("href= \"") {
            let remaining = &line[href_start + 7..];
            if let Some(href_end) = remaining.find('"') {
                let url = &remaining[..href_end];
                if url.starts_with('/') {
                    return format!("https://www.xb6v.com{}", url);
                }
                return url.to_string();
            }
        }
        String::new()
    }
}

impl Default for Xb6vScraper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VideoProvider for Xb6vScraper {
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
    #[ignore]
    async fn test_search_then_detail_then_play() {
        let scraper = Xb6vScraper::new();
        test_scraper(&scraper, "xb6v", TEST_KEYWORD).await
            .expect("xb6v test failed");
    }
}
