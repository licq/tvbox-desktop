use async_trait::async_trait;
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
        // First try the POST to get the redirect, but with a short timeout
        let url = format!("{}/e/search/1index.php", self.base.base_url);

        // Use a timeout for the POST
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.base.post_form_follow_redirect(
                &url,
                &[
                    ("keyboard", keyword),
                    ("show", "title"),
                    ("tempid", "1"),
                    ("tbname", "article"),
                    ("mid", "1"),
                    ("dopost", "search"),
                ],
            )
        ).await;

        let body = match result {
            Ok(Ok(b)) => {
                b
            }
            Ok(Err(_e)) => {
                // Fallback: try to construct URL directly
                let fallback_url = format!("{}/e/search/result/?searchid=1", self.base.base_url);
                self.base.fetch_text(&fallback_url).await?
            }
            Err(_) => {
                // Fallback: try to construct URL directly
                let fallback_url = format!("{}/e/search/result/?searchid=1", self.base.base_url);
                self.base.fetch_text(&fallback_url).await?
            }
        };

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
    /// Only extracts items within the post_container section (actual search results),
    /// excluding sidebar items that contain unrelated category links.
    /// Returns empty vec if HTML structure can't be determined.
    fn parse_search_results(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let lines: Vec<&str> = body.lines().collect();

        let category_patterns = ["dongzuopian", "xijupian", "juqingpian", "aiqingpian",
            "donghuapian", "kongbupian", "kehuanpian", "zhanzhengpian",
            "jilupian", "dianshiju", "ZongYi"];

        let mut item_keys: Vec<(String, usize)> = Vec::new();
        // Only extract items within the post_container div (search results area)
        let mut in_post_container = false;

        for (i, line) in lines.iter().enumerate() {
            let line = line.trim();
            if line.contains("id=\"post_container\"") {
                in_post_container = true;
                continue;
            }
            if in_post_container && line == "</ul>" {
                in_post_container = false;
                continue;
            }
            if !in_post_container {
                continue;
            }
            let has_category = category_patterns.iter().any(|cat| line.contains(cat));
            if !has_category {
                continue;
            }
            if let Some(key) = self.extract_key_from_line(line) {
                item_keys.push((key, i));
            }
        }

        let mut items = Vec::new();
        for (key, href_line_idx) in item_keys {
            let title = self.find_title_at_or_after(&lines, href_line_idx);
            let poster = self.extract_poster_from_item(&lines, href_line_idx);
            items.push(ScrapedCatalogItem {
                source_item_key: key.clone(),
                title,
                item_type: "movie".to_string(),
                poster,
                summary: None,
                detail_json: None,
                episodes: vec![],
            });
        }

        Ok(items)
    }

    fn extract_key_from_line(&self, line: &str) -> Option<String> {
        // Try single-quoted href first (sidebar format): href='/category/id.html'
        if let Some(href_start) = line.find("href='/" ) {
            let remaining = &line[href_start + 7..];
            let html_pos = remaining.find(".html'")?;
            let before_html = &remaining[..html_pos];
            let last_slash = before_html.rfind('/')?;
            let category = &before_html[..last_slash];
            let id = &before_html[last_slash + 1..];
            return Some(format!("{}/{}", category, id));
        }
        // Try double-quoted href (search results format): href="/category/id.html"
        if let Some(href_start) = line.find("href=\"/" ) {
            let remaining = &line[href_start + 7..];
            let html_pos = remaining.find(".html\"")?;
            let before_html = &remaining[..html_pos];
            let last_slash = before_html.rfind('/')?;
            let category = &before_html[..last_slash];
            let id = &before_html[last_slash + 1..];
            return Some(format!("{}/{}", category, id));
        }
        None
    }

    fn find_title_at_or_after(&self, lines: &[&str], href_line_idx: usize) -> String {
        // First try extracting from the href line's title attribute (post_container format)
        let line = lines[href_line_idx];
        if let Some(title_attr) = self.extract_title_from_attr(line) {
            return title_attr;
        }
        // Fall back to text-based extraction (sidebar format)
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

    /// Extract title from the `title=""` attribute of an anchor tag.
    /// Handles format: `<a ... title="TITLE">`
    fn extract_title_from_attr(&self, line: &str) -> Option<String> {
        // Find title="..." - might have single or double quotes
        for prefix in &["title=\"", "title='"] {
            if let Some(start) = line.find(prefix) {
                let remaining = &line[start + prefix.len()..];
                let quote = if *prefix == "title=\"" { '"' } else { '\'' };
                if let Some(end) = remaining.find(quote) {
                    let title = &remaining[..end];
                    if !title.is_empty() && title.len() < 200 {
                        // Strip any HTML tags from title
                        let clean = title.replace("<font color='red'>", "").replace("</font>", "");
                        return Some(clean);
                    }
                }
            }
        }
        None
    }

    /// Extract poster URL from search result item.
    /// Looks for `<img src="POSTER_URL"` near the href line within post_container.
    fn extract_poster_from_item(&self, lines: &[&str], href_line_idx: usize) -> Option<String> {
        // Check lines around the href for an img tag with src attribute
        let start = if href_line_idx > 3 { href_line_idx - 3 } else { 0 };
        let end = std::cmp::min(href_line_idx + 5, lines.len());
        for i in start..end {
            let line = lines[i].trim();
            if let Some(src_start) = line.find("<img src=\"") {
                let remaining = &line[src_start + 10..];
                if let Some(src_end) = remaining.find('\"') {
                    let url = &remaining[..src_end];
                    if url.starts_with("http") {
                        return Some(url.to_string());
                    }
                }
            }
        }
        None
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
            // Fallback: if no '>' before </a>, the text before </a> IS the title
            // This happens when <a> tag starts on previous line:
            //   <a href='/xxx.html'>
            //   剑来[第二季全]            </a>
            let text = before_close.trim();
            if !text.is_empty() && text.len() < 200 && !text.contains('<') && !text.contains('>') && !text.contains("href") {
                return Some(text.to_string());
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
    async fn test_search_then_detail_then_play() {
        let scraper = Xb6vScraper::new();
        test_scraper(&scraper, "xb6v", TEST_KEYWORD).await
            .expect("xb6v test failed");
    }
}
