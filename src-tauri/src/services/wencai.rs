use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use regex::Regex;
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WencaiListingEntry {
    pub title: String,
    pub detail_url: String,
    pub item_type: String,
    pub poster: Option<String>,
}

pub fn is_wencai_site(site: &crate::services::tvbox::TvboxSiteRecord) -> bool {
    site.site_key.contains("文采") || site.site_name.contains("文采") || site.raw_json.contains("文采")
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<WencaiListingEntry> {
    let item_regex = Regex::new(
        r#"(?s)<a class="module-item-pic" href="([^"]+)" title="([^"]+)".*?<img[^>]+(?:data-original|src)="([^"]+)""#,
    )
    .unwrap();

    item_regex
        .captures_iter(html)
        .filter_map(|capture| {
            Some(WencaiListingEntry {
                title: capture.get(2)?.as_str().trim().to_string(),
                detail_url: absolutize_url(page_url, capture.get(1)?.as_str()),
                item_type: item_type.to_string(),
                poster: capture.get(3).map(|value| value.as_str().to_string()),
            })
        })
        .collect()
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<h1 class="title">([^<]+)</h1>"#).unwrap();
    let summary_regex = Regex::new(r#"<div class="vod_content">([^<]+)</div>"#).unwrap();
    let section_regex = Regex::new(
        r#"(?s)<div class="module-tab-item"><span>([^<]+)</span></div>\s*<div class="module-play-list">(.*?)</div>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a href="([^"]+)">([^<]+)</a>"#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())?;
    let summary = summary_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string());

    let mut episodes = Vec::new();
    let mut seen = HashSet::new();
    for section in section_regex.captures_iter(html) {
        let Some(source_name) = section.get(1).map(|value| value.as_str().trim().to_string()) else {
            continue;
        };
        if is_external_source(&source_name) {
            continue;
        }
        let Some(body) = section.get(2).map(|value| value.as_str()) else {
            continue;
        };

        for anchor in anchor_regex.captures_iter(body) {
            let Some(play_url) = anchor.get(1).map(|value| absolutize_url(detail_url, value.as_str())) else {
                continue;
            };
            if !play_url.contains("/play/") || !seen.insert(play_url.clone()) {
                continue;
            }
            let Some(episode_label) = anchor.get(2).map(|value| value.as_str().trim().to_string()) else {
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
        item_type: "movie".to_string(),
        poster: None,
        summary,
        detail_json: Some(format!(r#"{{"source":"wencai","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

pub fn extract_player_url(body: &str) -> Option<String> {
    let regex = Regex::new(r#""url":"([^"]+)""#).unwrap();
    regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().replace(r#"\/"#, "/").replace(r#"\u0026"#, "&"))
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

#[cfg(test)]
mod tests {
    use super::{extract_player_url, parse_detail_page, parse_listing_page};

    #[test]
    fn parses_wencai_listing_entries() {
        let html = r#"
            <a class="module-item-pic" href="/detail/123.html" title="示例电影">
              <img data-original="https://img.example.com/poster.jpg" />
            </a>
        "#;

        let entries = parse_listing_page("https://www.wencai.example/list/1.html", "movie", html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "示例电影");
        assert_eq!(entries[0].item_type, "movie");
        assert_eq!(entries[0].detail_url, "https://www.wencai.example/detail/123.html");
    }

    #[test]
    fn parses_wencai_detail_page_and_filters_external_lines() {
        let html = r#"
            <h1 class="title">示例电影</h1>
            <div class="vod_content">剧情简介</div>
            <div class="module-tab-item"><span>文采线路A</span></div>
            <div class="module-play-list">
              <a href="/play/123-1-1.html">正片</a>
            </div>
            <div class="module-tab-item"><span>夸克网盘</span></div>
            <div class="module-play-list">
              <a href="https://pan.quark.cn/s/demo">合集</a>
            </div>
        "#;

        let item = parse_detail_page("https://www.wencai.example/detail/123.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "示例电影");
        assert_eq!(item.episodes.len(), 1);
        assert_eq!(item.episodes[0].source_name, "文采线路A");
        assert_eq!(item.episodes[0].play_url, "https://www.wencai.example/play/123-1-1.html");
    }

    #[test]
    fn extracts_wencai_player_url() {
        let html = r#"player_aaaa={"url":"https:\/\/media.example.com\/demo\/index.m3u8"}"#;
        assert_eq!(
            extract_player_url(html).as_deref(),
            Some("https://media.example.com/demo/index.m3u8")
        );
    }
}
