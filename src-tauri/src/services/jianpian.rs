use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct JianpianListingEntry {
    pub title: String,
    pub detail_url: String,
    pub item_type: String,
    pub poster: Option<String>,
}

pub fn is_jianpian_site(site: &crate::services::tvbox::TvboxSiteRecord) -> bool {
    site.site_key.contains("荐片")
        || site.site_name.contains("荐片")
        || site.raw_json.contains("荐片")
        || site
            .ext
            .as_deref()
            .is_some_and(|ext| ext.contains("jianpian") || ext.contains("jpys"))
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<JianpianListingEntry> {
    let anchor_regex = Regex::new(r#"(?s)<a class="public-list-exp"([^>]*)>(.*?)</a>"#).unwrap();
    let href_regex = Regex::new(r#"href="([^"]+)""#).unwrap();
    let title_regex = Regex::new(r#"title="([^"]+)""#).unwrap();
    let poster_regex = Regex::new(r#"(?:data-original|data-src|src)="([^"]+)""#).unwrap();

    anchor_regex
        .captures_iter(html)
        .filter_map(|capture| {
            let attrs = capture.get(1)?.as_str();
            let body = capture.get(2)?.as_str();

            let Some(href) = href_regex
                .captures(attrs)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str())
            else {
                return None;
            };
            if !href.contains("/voddetail/") && !href.contains("/detail/") {
                return None;
            }

            let Some(title) = title_regex
                .captures(attrs)
                .and_then(|capture| capture.get(1))
                .map(|value| html_escape_decode(value.as_str()).trim().to_string())
            else {
                return None;
            };
            let poster = poster_regex
                .captures(body)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str().to_string());

            Some(JianpianListingEntry {
                title,
                detail_url: absolutize_url(page_url, href),
                item_type: item_type.to_string(),
                poster,
            })
        })
        .collect()
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<h1[^>]*>([^<]+)</h1>"#).unwrap();
    let section_regex = Regex::new(
        r#"(?s)<div class="switch-box-item"[^>]*>(.*?)</div>\s*<div class="anthology-list-box"[^>]*>(.*?)</div>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| html_escape_decode(value.as_str()).trim().to_string())?;

    let mut episodes = Vec::new();
    let mut seen = HashSet::new();
    for section in section_regex.captures_iter(html) {
        let Some(source_name) = section
            .get(1)
            .map(|value| strip_tags(value.as_str()))
            .map(|value| html_escape_decode(&value).trim().to_string())
        else {
            continue;
        };
        if is_external_source(&source_name) {
            continue;
        }
        let body = section.get(2).map(|value| value.as_str()).unwrap_or_default();

        for anchor in anchor_regex.captures_iter(body) {
            let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let play_url = absolutize_url(detail_url, href);
            if !is_play_url(&play_url) || !seen.insert(play_url.clone()) {
                continue;
            }
            let Some(episode_label) = anchor
                .get(2)
                .map(|value| html_escape_decode(value.as_str()).trim().to_string())
            else {
                continue;
            };

            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label,
                play_url,
                order_index: episodes.len() as i64,
            });
        }
    }

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type: infer_item_type(detail_url),
        poster: None,
        summary: None,
        detail_json: Some(format!(r#"{{"source":"jianpian","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

pub fn extract_player_url(body: &str) -> Option<String> {
    let player_regex = Regex::new(r#""url":"([^"]+)""#).unwrap();
    player_regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| {
            value
                .as_str()
                .replace(r#"\/"#, "/")
                .replace(r#"\u0026"#, "&")
        })
}

fn is_external_source(source_name: &str) -> bool {
    ["网盘", "夸克", "迅雷", "下载", "磁力"]
        .iter()
        .any(|needle| source_name.contains(needle))
}

fn absolutize_url(base_url: &str, candidate: &str) -> String {
    if candidate.starts_with("http://") || candidate.starts_with("https://") {
        candidate.to_string()
    } else {
        reqwest::Url::parse(base_url)
            .and_then(|base| base.join(candidate))
            .map(|url| url.to_string())
            .unwrap_or_else(|_| candidate.to_string())
    }
}

fn is_play_url(play_url: &str) -> bool {
    play_url.contains("/play/") || play_url.contains("/vodplay/")
}

fn infer_item_type(detail_url: &str) -> String {
    let normalized = detail_url.to_lowercase();

    if normalized.contains("/tv/")
        || normalized.contains("/dianshiju/")
        || normalized.contains("/series/")
    {
        "series".to_string()
    } else if normalized.contains("/zongyi/") || normalized.contains("/variety/") {
        "variety".to_string()
    } else if normalized.contains("/donghua/")
        || normalized.contains("/anime/")
        || normalized.contains("/dongman/")
    {
        "anime".to_string()
    } else {
        "movie".to_string()
    }
}

fn strip_tags(input: &str) -> String {
    let tag_regex = Regex::new(r"<[^>]+>").unwrap();
    html_escape_decode(tag_regex.replace_all(input, " ").trim())
}

fn html_escape_decode(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[cfg(test)]
mod tests {
    use super::{extract_player_url, parse_detail_page, parse_listing_page};

    #[test]
    fn parses_jianpian_listing_entries() {
        let html = r#"
            <a class="public-list-exp" href="/voddetail/123.html" title="示例电影">
              <img data-src="https://img.example.com/poster.jpg" />
            </a>
            <a class="public-list-exp" href="https://www.jianpian.example/voddetail/456.html" title="第二部">
              <img src="https://img.example.com/poster-2.jpg" />
            </a>
        "#;

        let entries = parse_listing_page("https://www.jianpian.example/list/1.html", "movie", html);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "示例电影");
        assert_eq!(entries[0].item_type, "movie");
        assert_eq!(entries[0].detail_url, "https://www.jianpian.example/voddetail/123.html");
        assert_eq!(
            entries[0].poster.as_deref(),
            Some("https://img.example.com/poster.jpg")
        );
        assert_eq!(entries[1].detail_url, "https://www.jianpian.example/voddetail/456.html");
    }

    #[test]
    fn parses_jianpian_detail_page_and_keeps_play_pages_only() {
        let html = r#"
            <h1>示例电影</h1>
            <div class="switch-box-item">荐片线路A</div>
            <div class="anthology-list-box">
              <a href="/play/123-1-1.html">正片</a>
              <a href="/play/123-1-2.html">正片2</a>
            </div>
            <div class="switch-box-item">荐片线路B</div>
            <div class="anthology-list-box">
              <a href="/vodplay/123-2-1.html">正片3</a>
              <a href="broken-link">坏链接</a>
            </div>
            <div class="switch-box-item">夸克网盘</div>
            <div class="anthology-list-box">
              <a href="https://pan.quark.cn/s/demo">合集</a>
            </div>
        "#;

        let item = parse_detail_page("https://www.jianpian.example/voddetail/123.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "示例电影");
        assert_eq!(item.item_type, "movie");
        assert_eq!(item.episodes.len(), 3);
        assert_eq!(item.episodes[0].source_name, "荐片线路A");
        assert_eq!(item.episodes[0].play_url, "https://www.jianpian.example/play/123-1-1.html");
        assert_eq!(
            item.episodes[2].play_url,
            "https://www.jianpian.example/vodplay/123-2-1.html"
        );
    }

    #[test]
    fn extracts_jianpian_player_url() {
        let html = r#"player_aaaa={"url":"https:\/\/media.example.com\/demo\/index.m3u8"}"#;
        assert_eq!(
            extract_player_url(html).as_deref(),
            Some("https://media.example.com/demo/index.m3u8")
        );
    }
}
