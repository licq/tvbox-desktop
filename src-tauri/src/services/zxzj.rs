use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use regex::Regex;
use std::collections::HashSet;
use tokio::task::JoinSet;

const ZXZJ_ROOT: &str = "https://www.zxzjhd.com/";
const ZXZJ_PAGE_LIMIT_PER_CATEGORY: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ZxzjListingEntry {
    title: String,
    detail_url: String,
    item_type: String,
    poster: Option<String>,
}

pub fn is_zxzj_site(site: &crate::services::tvbox::TvboxSiteRecord) -> bool {
    site.site_key.eq_ignore_ascii_case("zxzj")
        || site.site_name.contains("在线")
        || site
            .ext
            .as_deref()
            .is_some_and(|ext| ext.contains("zxzjhd.com") || ext.contains("zxzjys.com"))
}

pub async fn scrape_zxzj_catalog() -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let categories = [
        ("movie", 1_i32),
        ("series", 2_i32),
        ("series", 3_i32),
        ("series", 4_i32),
        ("series", 5_i32),
        ("anime", 6_i32),
    ];

    let mut page_jobs = Vec::new();
    for (item_type, category_id) in categories {
        let first_page_url = format!("{ZXZJ_ROOT}vodshow/{category_id}--------1---.html");
        let first_page_html = fetch_text(&client, &first_page_url).await?;
        let page_count = parse_page_count(&first_page_html).unwrap_or(1);
        let capped_count = page_count.min(ZXZJ_PAGE_LIMIT_PER_CATEGORY);

        page_jobs.push((first_page_url.clone(), item_type.to_string(), first_page_html));
        for page in 2..=capped_count {
            page_jobs.push((
                format!("{ZXZJ_ROOT}vodshow/{category_id}--------{page}---.html"),
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
                            r#"{{"source":"zxzj","url":"{}","item_type":"{}"}}"#,
                            entry.detail_url,
                            item_type
                        )),
                        episodes: Vec::new(),
                    });
                }
            }
            Ok(Err(error)) => {
                log::warn!("抓取 zxzj 列表失败: {}", error);
            }
            Err(error) => {
                log::warn!("zxzj 列表任务失败: {}", error);
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

pub async fn scrape_zxzj_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    Ok(parse_detail_page(detail_url, &html))
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<ZxzjListingEntry> {
    let anchor_regex = Regex::new(
        r#"<a class="stui-vodlist__thumb[^"]*" href="([^"]+)" title="([^"]+)"(?:[^>]*data-original="([^"]+)")?"#,
    )
    .unwrap();

    anchor_regex
        .captures_iter(html)
        .filter_map(|capture| {
            let href = capture.get(1)?.as_str();
            if !href.contains("/voddetail/") {
                return None;
            }
            Some(ZxzjListingEntry {
                title: html_escape_decode(capture.get(2)?.as_str()).trim().to_string(),
                detail_url: absolutize_url(page_url, href),
                item_type: item_type.to_string(),
                poster: capture.get(3).map(|value| value.as_str().to_string()),
            })
        })
        .collect()
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<h1 class="title">([^<]+)</h1>"#).unwrap();
    let desc_regex =
        Regex::new(r#"<span class="detail-content"[^>]*>(.*?)</span>"#).unwrap();
    let meta_regex = Regex::new(r#"<meta name="description" content="([^"]+)""#).unwrap();
    let poster_regex =
        Regex::new(r#"<img class="lazyload"[^>]+data-original="([^"]+)""#).unwrap();

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
    let item_type = infer_item_type(detail_url);
    let episodes = parse_play_episodes(detail_url, html);

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type,
        poster,
        summary,
        detail_json: Some(format!(r#"{{"source":"zxzj","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

pub(crate) fn parse_page_count(html: &str) -> Option<usize> {
    let page_regex = Regex::new(r#"<li class="active num"><a>\d+/(\d+)</a></li>"#).unwrap();
    page_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|value| value.as_str().parse::<usize>().ok())
}

pub(crate) fn parse_play_episodes(detail_url: &str, html: &str) -> Vec<ScrapedCatalogEpisode> {
    let section_regex = Regex::new(
        r#"(?s)<div class="stui-vodlist__head">.*?<h3>([^<]+)</h3>.*?</div>\s*<ul class="stui-content__playlist clearfix">(.*?)</ul>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a href="([^"]+)">([^<]+)</a>"#).unwrap();
    let mut episodes = Vec::new();

    for section in section_regex.captures_iter(html) {
        let source_name = html_escape_decode(section.get(1).unwrap().as_str())
            .trim()
            .to_string();
        if is_external_source(&source_name) {
            continue;
        }
        let Some(body) = section.get(2).map(|value| value.as_str()) else {
            continue;
        };

        for anchor in anchor_regex.captures_iter(body) {
            let Some(href) = anchor.get(1).map(|value| value.as_str()) else {
                continue;
            };
            let Some(label) = anchor.get(2).map(|value| value.as_str()) else {
                continue;
            };
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label: html_escape_decode(label).trim().to_string(),
                play_url: absolutize_url(detail_url, href),
                order_index: episodes.len() as i64,
            });
        }
    }

    episodes
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

fn infer_item_type(_detail_url: &str) -> String {
    "series".to_string()
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
    use super::{
        extract_player_url, parse_detail_page, parse_listing_page, parse_page_count,
        parse_play_episodes,
    };

    #[test]
    fn parses_zxzj_page_count_and_listing_entries() {
        let html = r#"
            <span class="all">，共检索到“1310”条结果</span>
            <ul class="stui-vodlist clearfix">
                <li><a class="stui-vodlist__thumb lazyload" href="/voddetail/4601.html" title="铁血战士：杀戮之地" data-original="https://img1.example.com/4601.jpg"></a></li>
                <li><a class="stui-vodlist__thumb lazyload" href="/voddetail/4596.html" title="惊天魔盗团3" data-original="https://img1.example.com/4596.jpg"></a></li>
            </ul>
            <li class="active num"><a>1/110</a></li>
        "#;
        let items = parse_listing_page("https://www.zxzjhd.com/vodshow/1--------1---.html", "movie", html);
        assert_eq!(parse_page_count(html), Some(110));
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "铁血战士：杀戮之地");
        assert_eq!(items[0].item_type, "movie");
        assert_eq!(
            items[0].poster.as_deref(),
            Some("https://img1.example.com/4601.jpg")
        );
    }

    #[test]
    fn parses_zxzj_detail_and_filters_external_lines() {
        let html = r#"
            <h1 class="title">为全人类第五季</h1>
            <img class="lazyload" data-original="https://img1.example.com/4627.jpg" />
            <span class="detail-content" style="display: none;">第五季剧情简介</span>
            <div class="stui-vodlist__head"><h3>播放线路5</h3></div>
            <ul class="stui-content__playlist clearfix">
                <li><a href="/vodplay/4627-1-1.html">第01集</a></li>
                <li><a href="/vodplay/4627-1-2.html">第02集</a></li>
            </ul>
            <div class="stui-vodlist__head"><h3>百度网盘</h3></div>
            <ul class="stui-content__playlist clearfix">
                <li><a href="/vodplay/4627-2-1.html">合集</a></li>
            </ul>
        "#;
        let item = parse_detail_page("https://www.zxzjhd.com/voddetail/4627.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "为全人类第五季");
        assert_eq!(item.poster.as_deref(), Some("https://img1.example.com/4627.jpg"));
        assert_eq!(item.summary.as_deref(), Some("第五季剧情简介"));
        assert_eq!(item.episodes.len(), 2);
        assert_eq!(item.episodes[0].source_name, "播放线路5");
        assert!(item
            .episodes
            .iter()
            .all(|episode| !episode.source_name.contains("百度网盘")));
    }

    #[test]
    fn extracts_zxzj_player_url() {
        let body = r#"var player_aaaa={"url":"https:\/\/jx.zxzjys.com:9876\/player-v2\/common\/demo","from":"line5"}"#;
        assert_eq!(
            extract_player_url(body).as_deref(),
            Some("https://jx.zxzjys.com:9876/player-v2/common/demo")
        );
    }

    #[test]
    fn parses_zxzj_play_episodes() {
        let html = r#"
            <div class="stui-vodlist__head"><h3>播放线路5</h3></div>
            <ul class="stui-content__playlist clearfix">
                <li><a href="/vodplay/4627-1-1.html">第01集</a></li>
            </ul>
        "#;
        let episodes = parse_play_episodes("https://www.zxzjhd.com/voddetail/4627.html", html);
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].episode_label, "第01集");
        assert_eq!(
            episodes[0].play_url,
            "https://www.zxzjhd.com/vodplay/4627-1-1.html"
        );
    }
}
