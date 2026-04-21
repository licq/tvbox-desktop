use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use regex::Regex;
use std::collections::HashSet;
use tokio::task::JoinSet;

const LIBVIO_ROOT: &str = "https://www.libvio.me/";
const LIBVIO_PAGE_LIMIT_PER_CATEGORY: usize = 15;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LibvioListingEntry {
    title: String,
    detail_url: String,
    item_type: String,
    poster: Option<String>,
}

pub fn is_libvio_site(site: &crate::services::tvbox::TvboxSiteRecord) -> bool {
    site.site_key.eq_ignore_ascii_case("Lib")
        || site.site_name.contains("立播")
        || site.api.as_deref().is_some_and(|api| api.contains("Libvio"))
        || site.raw_json.contains("Libvio")
}

pub async fn scrape_libvio_catalog() -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let categories = [
        ("movie", "type/1"),
        ("series", "type/2"),
        ("anime", "type/4"),
        ("series", "type/15"),
        ("series", "type/16"),
    ];

    let mut page_jobs = Vec::new();
    for (item_type, slug) in categories {
        let first_page_url = format!("{LIBVIO_ROOT}{slug}.html");
        let first_page_html = fetch_text(&client, &first_page_url).await?;
        let page_count = parse_page_count(&first_page_html).unwrap_or(1);
        let capped_count = page_count.min(LIBVIO_PAGE_LIMIT_PER_CATEGORY);

        page_jobs.push((first_page_url.clone(), item_type.to_string(), first_page_html));
        for page in 2..=capped_count {
            page_jobs.push((
                format!("{LIBVIO_ROOT}{slug}-{page}.html"),
                item_type.to_string(),
                String::new(),
            ));
        }
    }

    let mut join_set = JoinSet::new();
    let mut queued = page_jobs.into_iter();
    for _ in 0..10 {
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
                            r#"{{"source":"libvio","url":"{}","item_type":"{}"}}"#,
                            entry.detail_url, item_type
                        )),
                        episodes: Vec::new(),
                    });
                }
            }
            Ok(Err(error)) => {
                log::warn!("抓取 libvio 列表失败: {}", error);
            }
            Err(error) => {
                log::warn!("libvio 列表任务失败: {}", error);
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

pub async fn scrape_libvio_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    Ok(parse_detail_page(detail_url, &html))
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

fn parse_listing_page(page_url: &str, item_type: &str, html: &str) -> Vec<LibvioListingEntry> {
    let anchor_regex = Regex::new(
        r#"<a class="stui-vodlist__thumb[^"]*" href="([^"]+)" title="([^"]+)"(?:[^>]*data-original="([^"]+)")?"#,
    )
    .unwrap();

    anchor_regex
        .captures_iter(html)
        .filter_map(|capture| {
            let href = capture.get(1)?.as_str();
            if !href.contains("/detail/") {
                return None;
            }
            Some(LibvioListingEntry {
                title: html_escape_decode(capture.get(2)?.as_str()).trim().to_string(),
                detail_url: absolutize_url(page_url, href),
                item_type: item_type.to_string(),
                poster: capture.get(3).map(|value| value.as_str().to_string()),
            })
        })
        .collect()
}

fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<h1 class="title">([^<]+)</h1>"#).unwrap();
    let desc_regex = Regex::new(r#"<span class="detail-content"[^>]*>(.*?)</span>"#).unwrap();
    let meta_regex = Regex::new(r#"<meta name="description" content="([^"]+)""#).unwrap();
    let poster_regex = Regex::new(r#"data-original="([^"]+)""#).unwrap();
    let item_type = infer_item_type(detail_url);

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| html_escape_decode(value.as_str()).trim().to_string())?;
    let summary = desc_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| strip_tags(value.as_str()))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            meta_regex
                .captures(html)
                .and_then(|capture| capture.get(1))
                .map(|value| html_escape_decode(value.as_str()).trim().to_string())
                .filter(|value| !value.is_empty())
        });
    let poster = poster_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string());
    let episodes = parse_play_episodes(detail_url, html);

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type,
        poster,
        summary,
        detail_json: Some(format!(r#"{{"source":"libvio","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

fn parse_page_count(html: &str) -> Option<usize> {
    let page_regex = Regex::new(r#"<li class="active num"><a>\d+/(\d+)</a></li>"#).unwrap();
    page_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|value| value.as_str().parse::<usize>().ok())
}

fn parse_play_episodes(detail_url: &str, html: &str) -> Vec<ScrapedCatalogEpisode> {
    let section_regex = Regex::new(
        r#"(?s)<div class="playlist-panel(?: netdisk-panel)?">.*?<h3>([^<]+)</h3>.*?<div class="(?:playlist-list|netdisk-list)">(.*?)</div>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>\s*<span class="(?:episode-name|netdisk-name)">?([^<]+)"#).unwrap();
    let plain_anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>([^<]+)</a>"#).unwrap();
    let mut episodes = Vec::new();
    let mut seen = HashSet::new();

    for section in section_regex.captures_iter(html) {
        let source_name = html_escape_decode(section.get(1).unwrap().as_str())
            .trim()
            .to_string();
        if is_external_source(&source_name) {
            continue;
        }
        let body = section.get(2).map(|value| value.as_str()).unwrap_or_default();

        for anchor in plain_anchor_regex.captures_iter(body) {
            let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(label) = anchor.get(2).map(|value| value.as_str()) else {
                continue;
            };
            if href.starts_with("http://") || href.starts_with("https://") && !href.contains("/play/") {
                continue;
            }
            let play_url = absolutize_url(detail_url, href);
            if !play_url.contains("/play/") || !seen.insert(play_url.clone()) {
                continue;
            }

            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label: html_escape_decode(label).trim().to_string(),
                play_url,
                order_index: episodes.len() as i64,
            });
        }

        for anchor in anchor_regex.captures_iter(body) {
            let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(label) = anchor.get(2).map(|value| value.as_str()) else {
                continue;
            };
            let play_url = absolutize_url(detail_url, href);
            if !play_url.contains("/play/") || !seen.insert(play_url.clone()) {
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

fn infer_item_type(detail_url: &str) -> String {
    if detail_url.contains("/type/4") {
        "anime".to_string()
    } else {
        "series".to_string()
    }
}

fn is_external_source(source_name: &str) -> bool {
    ["夸克", "下载", "网盘", "迅雷", "UC"]
        .iter()
        .any(|needle| source_name.contains(needle))
}

fn strip_tags(input: &str) -> String {
    let tag_regex = Regex::new(r"<[^>]+>").unwrap();
    html_escape_decode(tag_regex.replace_all(input, " ").trim())
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
    use super::{extract_player_url, parse_detail_page, parse_listing_page, parse_page_count};

    #[test]
    fn parses_libvio_listing_entries() {
        let html = r#"
            <li class="active num"><a>1/251</a></li>
            <a class="stui-vodlist__thumb lazyload" href="/detail/714893377.html" title="寻龙诀·觅踪" data-original="https://img.example.com/1.jpg"></a>
            <a class="stui-vodlist__thumb lazyload" href="/detail/714893370.html" title="准备好了没" data-original="https://img.example.com/2.jpg"></a>
        "#;
        let items = parse_listing_page("https://www.libvio.me/type/1.html", "movie", html);
        assert_eq!(parse_page_count(html), Some(251));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "寻龙诀·觅踪");
        assert_eq!(items[0].item_type, "movie");
    }

    #[test]
    fn parses_libvio_detail_page_and_filters_downloads() {
        let html = r#"
            <h1 class="title">菜鸟老警 第六季</h1>
            <img class="lazyload" data-original="https://img.example.com/poster.jpg" />
            <span class="detail-content" style="display:none;">内详</span>
            <div class="playlist-panel">
              <div class="panel-head"><h3>BD5播放</h3></div>
              <div class="playlist-list">
                <a href="/play/714891197-1-1.html">第01集</a>
                <a href="/play/714891197-1-2.html">第02集</a>
              </div>
            </div>
            <div class="playlist-panel netdisk-panel">
              <div class="panel-head netdisk-head"><h3>视频下载 (夸克)</h3></div>
              <div class="netdisk-list">
                <a class="netdisk-item" href="https://pan.quark.cn/s/a6cc8f8a7dce"><span class="netdisk-name">合集</span></a>
              </div>
            </div>
        "#;
        let item = parse_detail_page("https://www.libvio.me/detail/714891197.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "菜鸟老警 第六季");
        assert_eq!(item.episodes.len(), 2);
        assert_eq!(item.episodes[0].source_name, "BD5播放");
        assert!(item
            .episodes
            .iter()
            .all(|episode| episode.play_url.contains("/play/714891197-1-")));
    }

    #[test]
    fn extracts_libvio_player_url() {
        let body = r#"var player_aaaa={"url":"https:\/\/v.vbing.me\/t3\/The_Rookie\/The_RookieS06\/The_RookieS06E01.mp4","from":"vr2"}"#;
        assert_eq!(
            extract_player_url(body).as_deref(),
            Some("https://v.vbing.me/t3/The_Rookie/The_RookieS06/The_RookieS06E01.mp4")
        );
    }
}
