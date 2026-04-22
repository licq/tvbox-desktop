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
    let modern_anchor_regex =
        Regex::new(r#"(?s)<a class="[^"]*card[^"]*" href="([^"]+)" title="([^"]+)".*?</a>"#)
            .unwrap();
    let anchor_regex = Regex::new(r#"(?s)<a class="public-list-exp"([^>]*)>(.*?)</a>"#).unwrap();
    let href_regex = Regex::new(r#"href="([^"]+)""#).unwrap();
    let title_regex = Regex::new(r#"title="([^"]+)""#).unwrap();
    let poster_regex = Regex::new(r#"(?:data-original|data-src|src)="([^"]+)""#).unwrap();
    let mut entries = Vec::new();
    let mut seen = HashSet::new();

    for capture in modern_anchor_regex.captures_iter(html) {
        let Some(href) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        if !href.contains("/vod/") {
            continue;
        }
        let Some(title) = capture
            .get(2)
            .map(|value| html_escape_decode(value.as_str()).trim().to_string())
        else {
            continue;
        };
        let body = capture.get(0).map(|value| value.as_str()).unwrap_or_default();
        let poster = poster_regex
            .captures(body)
            .and_then(|poster| poster.get(1))
            .map(|value| value.as_str().to_string());
        let detail_url = absolutize_url(page_url, href);
        if seen.insert(detail_url.clone()) {
            entries.push(JianpianListingEntry {
                title,
                detail_url,
                item_type: item_type.to_string(),
                poster,
            });
        }
    }

    for capture in anchor_regex.captures_iter(html) {
        let attrs = capture.get(1).map(|value| value.as_str()).unwrap_or_default();
        let body = capture.get(2).map(|value| value.as_str()).unwrap_or_default();

        let Some(href) = href_regex
            .captures(attrs)
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str())
        else {
            continue;
        };
        if !href.contains("/voddetail/") && !href.contains("/detail/") && !href.contains("/vod/") {
            continue;
        }

        let Some(title) = title_regex
            .captures(attrs)
            .and_then(|capture| capture.get(1))
            .map(|value| html_escape_decode(value.as_str()).trim().to_string())
        else {
            continue;
        };
        let poster = poster_regex
            .captures(body)
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str().to_string());
        let detail_url = absolutize_url(page_url, href);
        if seen.insert(detail_url.clone()) {
            entries.push(JianpianListingEntry {
                title,
                detail_url,
                item_type: item_type.to_string(),
                poster,
            });
        }
    }

    entries
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"(?s)<h[123][^>]*>(.*?)</h[123]>"#).unwrap();
    let summary_regex =
        Regex::new(r#"(?s)<section class="section-ryhd6l">\s*<div class="section-head-ryhd6l">\s*<h2 class="title">剧情介绍</h2>.*?<div class="section-content-ryhd6l">\s*(.*?)\s*</div>"#)
            .unwrap();
    let modern_section_regex =
        Regex::new(r#"(?s)<section class="[^"]*vod-play-list-box[^"]*"[^>]*>(.*?)</section>"#)
            .unwrap();
    let modern_source_regex = Regex::new(r#"<h2 class="title">([^<]+)</h2>"#).unwrap();
    let modern_anchor_regex =
        Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*title="[^"]*"[^>]*>(.*?)</a>"#).unwrap();
    let section_regex = Regex::new(
        r#"(?s)<div class="switch-box-item"[^>]*>(.*?)</div>\s*<div class="anthology-list-box"[^>]*>(.*?)</div>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
    let tab_regex =
        Regex::new(r##"(?s)<li[^>]*>\s*<a href="#([^"]+)"[^>]*>([^<]+)</a>\s*</li>"##).unwrap();
    let playlist_regex = Regex::new(
        r#"(?s)<div id="([^"]+)"[^>]*>\s*<ul class="stui-content__playlist[^"]*"[^>]*>(.*?)</ul>"#,
    )
    .unwrap();

    let score_regex = Regex::new(r#"(?s)<span[^>]*class="[^"]*score[^"]*"[^>]*>.*?</span>"#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| score_regex.replace_all(value.as_str(), "").to_string())
        .map(|value| strip_tags(&value))
        .map(|value| html_escape_decode(&value).trim().to_string())?;
    let summary = summary_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| strip_tags(value.as_str()))
        .filter(|value| !value.is_empty());

    let mut episodes = Vec::new();
    let mut seen = HashSet::new();
    for section in modern_section_regex.captures_iter(html) {
        let Some(source_name) = section
            .get(1)
            .and_then(|value| modern_source_regex.captures(value.as_str()))
            .and_then(|capture| capture.get(1))
            .map(|value| html_escape_decode(value.as_str()).trim().to_string())
        else {
            continue;
        };
        if is_external_source(&source_name) {
            continue;
        }
        let body = section.get(1).map(|value| value.as_str()).unwrap_or_default();
        for anchor in modern_anchor_regex.captures_iter(body) {
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
        let body = section
            .get(2)
            .map(|value| value.as_str())
            .unwrap_or_default();

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

    if episodes.is_empty() {
        let mut playlists = std::collections::HashMap::new();
        for playlist in playlist_regex.captures_iter(html) {
            let Some(playlist_id) = playlist.get(1).map(|value| value.as_str().to_string()) else {
                continue;
            };
            let Some(body) = playlist.get(2).map(|value| value.as_str().to_string()) else {
                continue;
            };
            playlists.insert(playlist_id, body);
        }

        for tab in tab_regex.captures_iter(html) {
            let Some(playlist_id) = tab.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(source_name) = tab
                .get(2)
                .map(|value| html_escape_decode(value.as_str()).trim().to_string())
            else {
                continue;
            };
            if is_external_source(&source_name) {
                continue;
            }
            let Some(body) = playlists.get(playlist_id) else {
                continue;
            };

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
    }

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type: infer_item_type(detail_url, html),
        poster: None,
        summary,
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
    play_url.contains("/play/") || play_url.contains("/vodplay/") || play_url.contains("/jpplay/")
}

fn infer_item_type(detail_url: &str, html: &str) -> String {
    let page_text = strip_tags(html);
    if ["连续剧", "欧美剧", "国产剧", "香港剧", "台湾剧", "日本剧", "韩国剧", "海外剧", "泰国剧", "短剧"]
        .iter()
        .any(|needle| page_text.contains(needle))
    {
        return "series".to_string();
    }
    if ["综艺", "真人秀", "脱口秀"]
        .iter()
        .any(|needle| page_text.contains(needle))
    {
        return "variety".to_string();
    }
    if ["动漫", "动画", "番剧"]
        .iter()
        .any(|needle| page_text.contains(needle))
    {
        return "anime".to_string();
    }

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
    use reqwest::Client;
    use std::time::Duration;

    const DEFAULT_JIANPIAN_DETAIL_URL: &str = "https://jpvod.com/vod/97910.html";

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
        assert_eq!(
            entries[0].detail_url,
            "https://www.jianpian.example/voddetail/123.html"
        );
        assert_eq!(
            entries[0].poster.as_deref(),
            Some("https://img.example.com/poster.jpg")
        );
        assert_eq!(
            entries[1].detail_url,
            "https://www.jianpian.example/voddetail/456.html"
        );
    }

    #[test]
    fn parses_jianpian_jpvod_listing_entries() {
        let html = r#"
            <li class="vod-item-default-ryhd6l">
              <a class="d-block card p-0" href="/vod/97910.html" title="帝王计划：怪兽遗产第二季">
                <div class="poster rounded-0">
                  <div class="lazyload" data-original="https://img.example.com/poster.webp"></div>
                </div>
              </a>
            </li>
        "#;

        let entries = parse_listing_page("https://jpvod.com/type/2.html", "series", html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "帝王计划：怪兽遗产第二季");
        assert_eq!(entries[0].detail_url, "https://jpvod.com/vod/97910.html");
        assert_eq!(entries[0].poster.as_deref(), Some("https://img.example.com/poster.webp"));
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
        assert_eq!(
            item.episodes[0].play_url,
            "https://www.jianpian.example/play/123-1-1.html"
        );
        assert_eq!(
            item.episodes[2].play_url,
            "https://www.jianpian.example/vodplay/123-2-1.html"
        );
    }

    #[test]
    fn parses_jianpian_vodjp_style_detail_page() {
        let html = r##"
            <h2 class="title fs-4 fw-bold">帝王计划：怪兽遗产第二季</h2>
            <section class="section-ryhd6l web-position p-0 bg-transparent border-0 shadow-none">
              <a href="/">首页</a>
              <a href="/type/2.html">连续剧</a>
              <a href="/type/17.html">欧美剧</a>
            </section>
            <section class="section-ryhd6l vod-play-list-box vod-play-list-1 active">
              <div class="section-head-ryhd6l justify-content-start">
                <h2 class="title">金牌资源</h2>
              </div>
              <div class="section-content-ryhd6l">
                <a class="w-100 btn btn-secondary active" href="/play/97910-1-1.html" title="播放帝王计划：怪兽遗产第二季第1集">第1集</a>
                <a class="w-100 btn btn-secondary" href="/play/97910-1-2.html" title="播放帝王计划：怪兽遗产第二季第2集">第2集</a>
              </div>
            </section>
            <section class="section-ryhd6l">
              <div class="section-head-ryhd6l"><h2 class="title">剧情介绍</h2></div>
              <div class="section-content-ryhd6l">剧情简介</div>
            </section>
        "##;

        let item = parse_detail_page("https://jpvod.com/vod/97910.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "帝王计划：怪兽遗产第二季");
        assert_eq!(item.item_type, "series");
        assert_eq!(item.summary.as_deref(), Some("剧情简介"));
        assert_eq!(item.episodes.len(), 2);
        assert_eq!(item.episodes[0].source_name, "金牌资源");
        assert_eq!(
            item.episodes[0].play_url,
            "https://jpvod.com/play/97910-1-1.html"
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

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn scrapes_real_jianpian_detail_page() {
        let url = std::env::var("JIANPIAN_DETAIL_URL")
            .unwrap_or_else(|_| DEFAULT_JIANPIAN_DETAIL_URL.to_string());
        let html = fetch_live_html(&url).await;
        let item = parse_detail_page(&url, &html).expect("jianpian detail should parse");
        println!("jianpian url={url} episodes={}", item.episodes.len());
        assert!(
            !item.episodes.is_empty(),
            "expected jianpian detail to produce episodes"
        );
    }

    async fn fetch_live_html(url: &str) -> String {
        Client::builder()
            .no_proxy()
            .http1_only()
            .timeout(Duration::from_secs(60))
            .user_agent("Mozilla/5.0")
            .build()
            .expect("client should build")
            .get(url)
            .send()
            .await
            .expect("request should succeed")
            .error_for_status()
            .expect("response should be successful")
            .text()
            .await
            .expect("body should decode")
    }
}
