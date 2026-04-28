use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose;
use regex::Regex;

use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};
use crate::services::provider::native::NativeScraper;

/// Au1080 (formerly auete.com) native scraper.
/// The site redirects from auete.com to au1080.com as the main domain.
/// Search (/auete4so.php) requires CAPTCHA, so we use home + category browsing instead.
pub struct AueteScraper {
    base: NativeScraper,
}

impl AueteScraper {
    pub fn new() -> Self {
        Self {
            // Use au1080.com as the actual backend - auete.com redirects here
            base: NativeScraper::new("auete", "🏝奥特┃多线", "https://au1080.com"),
        }
    }

    /// Home page returns recently updated items with no CAPTCHA required.
    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let body = self.base.fetch_text(&self.base.base_url).await?;
        Ok(self.parse_listing_page(&body))
    }

    /// Search is blocked by CAPTCHA on this site. Return home() as fallback.
    pub async fn search(&self, _keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.home().await
    }

    /// Detail page contains episode list.
    /// ids format: "Movie/khp/yuzhouhuweidui_baibianliuxing" (path from URL)
    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/{}/", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        self.parse_detail_page(&body, ids)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        let body = self.base.fetch_text(play_url).await?;
        let video_url = extract_auete_player_url(&body)
            .unwrap_or_else(|| play_url.to_string());
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "auete".to_string(),
            target_url: video_url,
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }])
    }

    /// Parse listing page (home or category) for media items.
    /// Auete uses two link patterns:
    /// 1. Carousel links (banner): <a href="/Tv/wangju/slug/"> (no title on anchor, title is on img inside)
    /// 2. Detail links: <a href="/Tv/wangju/slug/" title="{Title}" ...> (title on anchor itself)
    /// We only match pattern 2 (with title on anchor) to get proper titles.
    fn parse_listing_page(&self, body: &str) -> Vec<ScrapedCatalogItem> {
        let mut items = Vec::new();
        let mut seen_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();

        // Pattern: <a href="/{Cat}/{SubCat}/{slug}/" title="{Title}"
        // The title attribute on the <a> tag is the definitive marker of a real detail link.
        // Carousel links don't have title= on the anchor itself.
        let detail_re = match Regex::new(r#"href="(/[A-Za-z]+/[^"/]+/[^"/]+/)"[^>]*\s+title="([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return items,
        };

        for line in body.lines() {
            let line = line.trim();
            if let Some(caps) = detail_re.captures(line) {
                let slug = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let title = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
                if !slug.is_empty() && !seen_slugs.contains(slug) {
                    // Filter out nav labels that appear as title values
                    let nav_labels = ["热映影视", "今日热门", "电影", "电视剧", "综艺", "动漫", "其他",
                        "Netflix影视", "连载剧集更新", "当天实时热播", "2024豆瓣年度榜单",
                        "豆瓣TOP250", "2025豆瓣年度榜单", "最新", "热映", "全集", "已完结"];
                    if nav_labels.contains(&title) {
                        continue;
                    }
                    seen_slugs.insert(slug.to_string());
                    let poster = self.extract_poster_from_anchor_line(line);
                    let category = if slug.starts_with("/Movie/") {
                        "movie"
                    } else if slug.starts_with("/Tv/") {
                        "series"
                    } else if slug.starts_with("/Dm/") {
                        "anime"
                    } else if slug.starts_with("/Zy/") {
                        "variety"
                    } else {
                        "movie"
                    };
                    let ids = slug.trim_start_matches('/').trim_end_matches('/');
                    items.push(ScrapedCatalogItem {
                        source_item_key: ids.to_string(),
                        title: title.to_string(),
                        item_type: category.to_string(),
                        poster,
                        summary: None,
                        detail_json: None,
                        episodes: vec![],
                    });
                }
            }
        }
        items
    }

    /// Extract poster image from an anchor line. The img is inside the <a>:
    /// <a href="..."><img src="..." alt="..." /></a>
    fn extract_poster_from_anchor_line(&self, line: &str) -> Option<String> {
        let img_re = match Regex::new(r#"img src="([^"]+)""#) {
            Ok(r) => r,
            Err(_) => return None,
        };
        if let Some(caps) = img_re.captures(line) {
            let src = caps.get(1)?.as_str();
            if src.starts_with("http") {
                return Some(src.to_string());
            }
        }
        None
    }

    fn parse_detail_page(&self, body: &str, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let mut episodes = Vec::new();
        let mut order_index = 0i64;

        let episode_re = match Regex::new(r#"href="(/[^"]+/play-\d+-\d+\.html)""#) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        for line in body.lines() {
            let line = line.trim();
            if let Some(caps) = episode_re.captures(line) {
                let play_path = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                if !play_path.is_empty() {
                    let base = self.base.base_url.trim_start_matches("https://");
                    let full_url = format!("https://{}{}", base, play_path);
                    let label = self.extract_episode_label_from_line(line, order_index);

                    episodes.push(ScrapedCatalogEpisode {
                        source_name: "auete".to_string(),
                        episode_label: label,
                        play_url: full_url,
                        order_index,
                    });
                    order_index += 1;
                }
            }
        }

        if episodes.is_empty() {
            return Ok(None);
        }

        Ok(Some(ScrapedCatalogItem {
            source_item_key: format!("auete-{}", ids),
            title: "Detail".to_string(),
            item_type: "movie".to_string(),
            poster: None,
            summary: None,
            detail_json: None,
            episodes,
        }))
    }

    fn extract_episode_label_from_line(&self, line: &str, fallback_idx: i64) -> String {
        // Try title attribute
        if let Some(start) = line.find("title=\"") {
            let remaining = &line[start + 7..];
            if let Some(end) = remaining.find('"') {
                let label = &remaining[..end];
                if !label.is_empty() && label.len() < 100 {
                    return label.to_string();
                }
            }
        }
        // Text between > and </a>
        if let Some(gt) = line.find('>') {
            let after = &line[gt + 1..];
            if let Some(lt) = after.find("</a>") {
                let text = after[..lt].trim();
                if !text.is_empty() && text.len() < 100 && !text.contains('<') {
                    return text.to_string();
                }
            }
        }
        // Button text (btn btn-orange)
        if let Some(btn_start) = line.find("btn btn-orange") {
            let remaining = &line[btn_start..];
            if let Some(gt) = remaining.find('>') {
                let after = &remaining[gt + 1..];
                if let Some(lt) = after.find('<') {
                    let text = after[..lt].trim();
                    if !text.is_empty() && text.len() < 100 {
                        return text.to_string();
                    }
                }
            }
        }
        format!("Episode {}", fallback_idx)
    }
}

impl Default for AueteScraper {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the actual m3u8 video URL from an auete play page.
/// The URL is base64-encoded in a `now=` JavaScript variable:
///   var now=base64decode("BASE64_ENCODED_URL");
/// The decoded URL contains the actual m3u8 source (e.g., https://vip.dytt-kan.com/...).
pub fn extract_auete_player_url(body: &str) -> Option<String> {
    // Pattern: var now=base64decode("...") or  var now=base64decode("...")
    // Note: there may be 1-2 spaces before "var" due to JS formatting
    if let Some(re) = Regex::new(r#"(?:var\s+now\s*=\s*base64decode\s*\()?\s*["']?([A-Za-z0-9+/=]{20,})["']?\s*\)"#).ok() {
        if let Some(caps) = re.captures(body) {
            if let Some(encoded) = caps.get(1) {
                let e = encoded.as_str();
                // Try standard base64
                if let Ok(decoded) = general_purpose::STANDARD.decode(e) {
                    let url = String::from_utf8_lossy(&decoded);
                    if url.starts_with("http") && (url.contains(".m3u8") || url.contains("dytt") || url.contains("mp4")) {
                        return Some(url.to_string());
                    }
                }
                // Try URL-safe base64
                if let Ok(decoded) = general_purpose::URL_SAFE_NO_PAD.decode(e) {
                    let url = String::from_utf8_lossy(&decoded);
                    if url.starts_with("http") && (url.contains(".m3u8") || url.contains("dytt") || url.contains("mp4")) {
                        return Some(url.to_string());
                    }
                }
            }
        }
    }

    // Pattern 2: direct iframe src with m3u8
    if let Some(re) = Regex::new(r#"<iframe[^>]+src="([^"]+\.m3u8[^"]*)""#).ok() {
        if let Some(caps) = re.captures(body) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }

    // Pattern 3: direct m3u8 URL in body
    if let Some(re) = Regex::new(r#"(https?://[^\s"']+\.m3u8[^\s"'<>]*)"#).ok() {
        if let Some(caps) = re.captures(body) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
    }

    None
}

#[async_trait]
impl VideoProvider for AueteScraper {
    fn source_key(&self) -> &str { &self.base.site_key }
    fn source_name(&self) -> &str { &self.base.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        self.home().await
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        Ok(vec![])
    }

    async fn category(&self, type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/{}/index.html", self.base.base_url, type_id);
        let body = self.base.fetch_text(&url).await?;
        Ok(self.parse_listing_page(&body))
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

    #[test]
    fn extracts_video_url_from_play_page() {
        let body = r#"<script>var vid="41757";var vfrom="0";var vpart="0"; var now=base64decode("aHR0cHM6Ly92aXAuZHl0dC1rYW4uY29tLzIwMjYwNDI3LzE0MDE5XzI1MWVhMzk5L2luZGV4Lm0zdTg=");</script>"#;
        let url = extract_auete_player_url(body);
        assert!(url.is_some(), "should extract m3u8 URL from base64, got: {:?}", url);
        let u = url.unwrap();
        assert!(u.contains("dytt") || u.contains(".m3u8"), "should be m3u8 URL: {}", u);
    }

    #[test]
    fn returns_none_when_no_player_data() {
        let body = r#"<html><body>No video here</body></html>"#;
        assert!(extract_auete_player_url(body).is_none());
    }

    #[test]
    fn extracts_video_url_multiline() {
        let body = r#"<script>
var vid="123";
var now=base64decode("aHR0cHM6Ly92aXAuZHl0dC1rYW4uY29tL3Rlc3QuaDUzdTg=");
</script>"#;
        let url = extract_auete_player_url(body);
        assert!(url.is_some(), "should extract from multiline HTML");
    }

    #[test]
    fn parse_listing_extracts_titles_and_slugs() {
        let scraper = AueteScraper::new();
        // Real HTML format: title= on the <a> tag
        let body = r#"<a href="/Tv/wangju/heiyegaobai/" title="重案解密" target="_blank"><img src="https://img.jpg" alt="重案解密" /></a>"#;
        let items = scraper.parse_listing_page(body);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "重案解密");
        assert_eq!(items[0].source_item_key, "Tv/wangju/heiyegaobai");
    }

    #[test]
    fn parse_listing_skips_nav_links() {
        let scraper = AueteScraper::new();
        // Nav links don't have title= on the anchor
        let body = r#"<a href="/Movie/index.html" title="电影">电影</a>
<a href="/Tv/index.html" title="电视剧">电视剧</a>
<a href="/topic/" title="2024豆瓣年度榜单">2024豆瓣年度榜单</a>"#;
        let items = scraper.parse_listing_page(body);
        assert_eq!(items.len(), 0, "nav links should be skipped");
    }

    #[test]
    fn parse_listing_skips_nav_title_values() {
        let scraper = AueteScraper::new();
        // These have title= but it's a nav label, not a movie title
        let body = r#"<a href="/Movie/index.html" title="电影">电影</a>"#;
        let items = scraper.parse_listing_page(body);
        assert_eq!(items.len(), 0);
    }
}
