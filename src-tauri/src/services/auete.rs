use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use regex::Regex;
use std::collections::HashSet;
use tokio::task::JoinSet;

const AUETE_ROOT: &str = "https://auete.top/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AueteListingEntry {
    title: String,
    detail_url: String,
    item_type: String,
    poster: Option<String>,
}

pub fn is_auete_site(site: &crate::services::tvbox::TvboxSiteRecord) -> bool {
    site.site_key.contains("奥特")
        || site.site_name.contains("Auete")
        || site.raw_json.contains("Auete")
        || site
            .ext
            .as_deref()
            .is_some_and(|ext| ext.contains("auete.") || ext.contains("au1080.com"))
}

pub async fn scrape_auete_catalog() -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let category_groups = [
        ("Movie", vec![
            ("", "movie"),
            ("xjp", "movie"),
            ("dzp", "movie"),
            ("aqp", "movie"),
            ("khp", "movie"),
            ("kbp", "movie"),
            ("jsp", "movie"),
            ("zzp", "movie"),
            ("jqp", "movie"),
        ]),
        ("Tv", vec![
            ("", "series"),
            ("oumei", "series"),
            ("hanju", "series"),
            ("riju", "series"),
            ("yataiju", "series"),
            ("wangju", "series"),
            ("taiju", "series"),
            ("neidi", "series"),
            ("tvbgj", "series"),
            ("yingju", "series"),
            ("waiju", "series"),
            ("duanju", "series"),
        ]),
        ("Zy", vec![("", "variety")]),
        ("Dm", vec![("", "anime")]),
    ];

    let mut page_jobs = Vec::new();
    for (group_name, subcats) in category_groups {
        for (subcat_slug, item_type) in subcats {
            let base_slug = if subcat_slug.is_empty() {
                group_name.to_string()
            } else {
                format!("{}/{}", group_name, subcat_slug)
            };
            let first_page_url = format!("{AUETE_ROOT}{}/index.html", base_slug);
            let first_page_html = match fetch_text(&client, &first_page_url).await {
                Ok(html) => html,
                Err(e) => {
                    log::warn!("抓取 {} 失败，跳过该分类: {}", first_page_url, e);
                    continue;
                }
            };
            let page_count = parse_page_count(&first_page_html).unwrap_or(1);

            page_jobs.push((first_page_url, item_type.to_string(), first_page_html));
            for page in 2..=page_count {
                page_jobs.push((
                    format!("{AUETE_ROOT}{}/index{}.html", base_slug, page),
                    item_type.to_string(),
                    String::new(),
                ));
            }
        }
    }

    let mut join_set = JoinSet::new();
    let mut queued = page_jobs.into_iter();
    for _ in 0..20 {
        let Some((page_url, item_type, maybe_html)) = queued.next() else {
            break;
        };
        let client = client.clone();
        join_set.spawn(async move {
            let html = if maybe_html.is_empty() {
                fetch_text(&client, &page_url).await?
            } else {
                maybe_html
            };
            Ok::<_, String>(parse_listing_page(&page_url, &item_type, &html))
        });
    }

    let mut items = Vec::new();
    let mut seen = HashSet::new();
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(Ok(entries)) => {
                for entry in entries {
                    if !seen.insert(entry.detail_url.clone()) {
                        continue;
                    }
                    let item_type = entry.item_type.clone();
                    items.push(ScrapedCatalogItem {
                        source_item_key: entry.detail_url.clone(),
                        title: entry.title,
                        item_type: item_type.clone(),
                        poster: entry.poster,
                        summary: None,
                        detail_json: Some(format!(
                            r#"{{"source":"auete","url":"{}","item_type":"{}"}}"#,
                            entry.detail_url, item_type
                        )),
                        episodes: Vec::new(),
                    });
                }
            }
            Ok(Err(error)) => {
                log::warn!("抓取 auete 列表失败: {}", error);
            }
            Err(error) => {
                log::warn!("auete 列表任务失败: {}", error);
            }
        }

        if let Some((page_url, item_type, maybe_html)) = queued.next() {
            let client = client.clone();
            join_set.spawn(async move {
                let html = if maybe_html.is_empty() {
                    fetch_text(&client, &page_url).await?
                } else {
                    maybe_html
                };
                Ok::<_, String>(parse_listing_page(&page_url, &item_type, &html))
            });
        }
    }

    Ok(items)
}

pub async fn scrape_auete_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    Ok(parse_detail_page(detail_url, &html))
}

pub fn extract_player_url(body: &str) -> Option<String> {
    let player_regex = Regex::new(r#"var\s+now\s*=\s*base64decode\("([^"]+)"\)"#).unwrap();
    let encoded = player_regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str())?;
    let decoded = STANDARD.decode(encoded).ok()?;
    String::from_utf8(decoded)
        .ok()
        .map(|value| value.trim().to_string())
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<AueteListingEntry> {
    let item_regex = Regex::new(
        r#"<li class="trans_3 "[^>]*>\s*<a href="([^"]+)" class="pic"[^>]*>\s*<img src="([^"]+)" alt="([^"]+)""#,
    )
    .unwrap();

    item_regex
        .captures_iter(html)
        .filter_map(|capture| {
            let href = capture.get(1)?.as_str();
            if href.contains("/play-") || !href.ends_with('/') {
                return None;
            }
            Some(AueteListingEntry {
                title: html_escape_decode(capture.get(3)?.as_str())
                    .trim()
                    .to_string(),
                detail_url: absolutize_url(page_url, href),
                item_type: item_type.to_string(),
                poster: capture.get(2).map(|value| value.as_str().to_string()),
            })
        })
        .collect()
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<meta property="og:title" content="([^"]+)""#).unwrap();
    let desc_regex = Regex::new(r#"<meta property="og:description" content="([^"]+)""#).unwrap();
    let poster_regex = Regex::new(r#"<meta property="og:image" content="([^"]+)""#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| html_escape_decode(value.as_str()).trim().to_string())?;
    let summary = desc_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| html_escape_decode(value.as_str()).trim().to_string())
        .filter(|value| !value.is_empty());
    let poster = poster_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string());
    let item_type = infer_item_type(detail_url);
    let episodes = parse_play_episodes(detail_url, html);

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type,
        poster,
        summary,
        detail_json: Some(format!(r#"{{"source":"auete","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

pub(crate) fn parse_page_count(html: &str) -> Option<usize> {
    let tail_regex =
        Regex::new(r#"href="/[A-Za-z]+/index(\d+)\.html" class="page-link">尾页"#).unwrap();
    tail_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|value| value.as_str().parse::<usize>().ok())
}

pub(crate) fn parse_play_episodes(detail_url: &str, html: &str) -> Vec<ScrapedCatalogEpisode> {
    let heading_regex = Regex::new(r#"』([^：<]+)："#).unwrap();
    let anchor_regex =
        Regex::new(r#"<a class="btn btn-orange" title="([^"]+)" href="([^"]+)""#).unwrap();
    let mut episodes = Vec::new();
    let mut seen = HashSet::new();

    for section in html.split(r#"<div id="player_list""#).skip(1) {
        let Some(source_name) = heading_regex
            .captures(section)
            .and_then(|capture| capture.get(1))
            .map(|value| html_escape_decode(value.as_str()).trim().to_string())
        else {
            continue;
        };
        let source_name = html_escape_decode(&source_name).trim().to_string();
        if is_external_source(&source_name) {
            continue;
        }
        for anchor in anchor_regex.captures_iter(section) {
            let Some(label) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(href) = anchor.get(2).map(|value| value.as_str()) else {
                continue;
            };
            let play_url = absolutize_url(detail_url, href);
            if !play_url.contains("/play-") || !seen.insert(play_url.clone()) {
                continue;
            }

            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label: html_escape_decode(label).trim().to_string(),
                play_url,
                order_index: episodes.len() as i64,
            });
        }
    }

    episodes
}

fn is_external_source(source_name: &str) -> bool {
    ["网盘", "夸克", "迅雷", "下载", "磁力"]
        .iter()
        .any(|needle| source_name.contains(needle))
}

fn infer_item_type(detail_url: &str) -> String {
    if detail_url.contains("/Movie/") {
        "movie".to_string()
    } else if detail_url.contains("/Tv/") {
        "series".to_string()
    } else if detail_url.contains("/Zy/") {
        "variety".to_string()
    } else if detail_url.contains("/Dm/") {
        "anime".to_string()
    } else {
        "series".to_string()
    }
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn fetch_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
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
    use super::{
        extract_player_url, parse_detail_page, parse_listing_page, parse_page_count,
        parse_play_episodes,
    };

    #[test]
    fn parses_auete_page_count_and_listing_entries() {
        let html = r#"
            <ul class="threadlist">
                <li class="trans_3 " data-href="/Movie/dzp/xunlongjuemizong/" data-tid="41618">
                    <a href="/Movie/dzp/xunlongjuemizong/" class="pic" target="_blank">
                        <img src="https://mtzy.me/upload/vod/demo.jpg" alt="寻龙诀·觅踪" title="寻龙诀·觅踪" class="lazy" />
                    </a>
                </li>
                <li class="trans_3 " data-href="/Movie/jqp/xiangyuzhijia/" data-tid="41619">
                    <a href="/Movie/jqp/xiangyuzhijia/" class="pic" target="_blank">
                        <img src="https://mtzy.me/upload/vod/demo2.jpg" alt="相遇之家" title="相遇之家" class="lazy" />
                    </a>
                </li>
            </ul>
            <a href="/Movie/index844.html" class="page-link">尾页</a>
        "#;
        let items = parse_listing_page("https://auete.top/Movie/index.html", "movie", html);
        assert_eq!(parse_page_count(html), Some(844));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "寻龙诀·觅踪");
        assert_eq!(items[0].item_type, "movie");
        assert_eq!(
            items[0].poster.as_deref(),
            Some("https://mtzy.me/upload/vod/demo.jpg")
        );
    }

    #[test]
    fn parses_auete_detail_and_playlists() {
        let html = r#"
            <meta property="og:title" content="寻龙诀·觅踪"/>
            <meta property="og:image" content="https://mtzy.me/upload/vod/demo.jpg"/>
            <meta property="og:description" content="故事发生在20世纪90年代"/>
            <div id="player_list" class="clearfix mt-3">
                <h2 class="title border-top border-bottom mb-3 pt-2 pb-2"><i class="icon-film"></i>『寻龙诀·觅踪』云播D线：<small>（M3u8超清播放）</small></h2>
                <ul><li id="00"><a class="btn btn-orange" title="HD国语" href="/Movie/dzp/xunlongjuemizong/play-0-0.html" target="_self">HD国语</a></li></ul>
            </div>
            <div id="player_list" class="clearfix mt-3">
                <h2 class="title border-top border-bottom mb-3 pt-2 pb-2"><i class="icon-film"></i>『寻龙诀·觅踪』百度网盘：<small>（外部）</small></h2>
                <ul><li id="10"><a class="btn btn-orange" title="资源" href="/Movie/dzp/xunlongjuemizong/play-9-0.html" target="_self">资源</a></li></ul>
            </div>
        "#;
        let item = parse_detail_page("https://auete.top/Movie/dzp/xunlongjuemizong/", html)
            .expect("detail should parse");
        assert_eq!(item.title, "寻龙诀·觅踪");
        assert_eq!(item.item_type, "movie");
        assert_eq!(
            item.poster.as_deref(),
            Some("https://mtzy.me/upload/vod/demo.jpg")
        );
        assert_eq!(item.summary.as_deref(), Some("故事发生在20世纪90年代"));
        assert_eq!(item.episodes.len(), 1);
        assert_eq!(item.episodes[0].source_name, "云播D线");
        assert_eq!(item.episodes[0].episode_label, "HD国语");
    }

    #[test]
    fn extracts_auete_player_url() {
        let body = r#"var vid="41618"; var now=base64decode("aHR0cHM6Ly92aXAuZHl0dC1rYW4uY29tLzIwMjYwNDE3LzEzNjMyX2I3ZTVkNTllL2luZGV4Lm0zdTg=");var pn="dyun";"#;
        assert_eq!(
            extract_player_url(body).as_deref(),
            Some("https://vip.dytt-kan.com/20260417/13632_b7e5d59e/index.m3u8")
        );
    }

    #[test]
    fn parses_auete_play_episodes() {
        let html = r#"
            <div id="player_list" class="clearfix mt-3">
                <h2 class="title border-top border-bottom mb-3 pt-2 pb-2"><i class="icon-film"></i>『寻龙诀·觅踪』云播M线：<small>（极速超清mp4格式播放）</small></h2>
                <ul><li id="10"><a class="btn btn-orange" title="正片国语" href="/Movie/dzp/xunlongjuemizong/play-1-0.html" target="_self">正片国语</a></li></ul>
            </div>
        "#;
        let episodes = parse_play_episodes("https://auete.top/Movie/dzp/xunlongjuemizong/", html);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].episode_label, "正片国语");
        assert_eq!(
            episodes[0].play_url,
            "https://auete.top/Movie/dzp/xunlongjuemizong/play-1-0.html"
        );
    }

    #[test]
    fn parses_subcategory_page_urls() {
        let test_cases = vec![
            ("https://auete.top/Movie/index.html", "movie"),
            ("https://auete.top/Movie/xjp/index.html", "movie"),
            ("https://auete.top/Tv/oumei/index.html", "series"),
            ("https://auete.top/Tv/neidi/index.html", "series"),
        ];

        let dummy_html = r#"
            <li class="trans_3 " data-href="/Movie/test/">
                <a href="/Movie/test/" class="pic">
                    <img src="https://example.com/poster.jpg" alt="测试影片"/>
                </a>
            </li>
        "#;

        for (url, expected_type) in test_cases {
            let results = parse_listing_page(url, expected_type, dummy_html);
            assert_eq!(results.len(), 1, "Should parse 1 entry from {}", url);
            assert_eq!(results[0].title, "测试影片");
            assert_eq!(results[0].item_type, expected_type);
        }
    }

    #[test]
    fn parses_page_count_correctly() {
        let html = r#"<a href="/Movie/index844.html" class="page-link">尾页</a>"#;
        assert_eq!(parse_page_count(html), Some(844));

        let html2 = r#"<a href="/Tv/index735.html" class="page-link">尾页</a>"#;
        assert_eq!(parse_page_count(html2), Some(735));
    }
}
