use crate::services::tvbox::TvboxSiteRecord;
use regex::Regex;
use std::collections::HashSet;
use tokio::task::JoinSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrapedCatalogEpisode {
    pub source_name: String,
    pub episode_label: String,
    pub play_url: String,
    pub order_index: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrapedCatalogItem {
    pub source_item_key: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
    pub detail_json: Option<String>,
    pub episodes: Vec<ScrapedCatalogEpisode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ListingEntry {
    title: String,
    detail_url: String,
    item_type: String,
}

pub async fn scrape_supported_tvbox_catalogs(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let mut items = Vec::new();
    if sites.iter().any(is_xb6v_site) {
        items.extend(scrape_xb6v_catalog().await?);
    }
    Ok(items)
}

fn is_xb6v_site(site: &TvboxSiteRecord) -> bool {
    site.site_key.contains("新6V")
        || site.site_name.contains("新6V")
        || site
            .ext
            .as_deref()
            .is_some_and(|ext| ext.contains("xb6v.com") || ext.contains("hao6v.com"))
}

async fn scrape_xb6v_catalog() -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let pages = [
        "https://www.xb6v.com/",
        "https://www.xb6v.com/qian50m.html",
        "https://www.xb6v.com/dianshiju/",
        "https://www.xb6v.com/ZongYi/",
        "https://www.xb6v.com/dongzuopian/",
        "https://www.xb6v.com/juqingpian/",
    ];

    let mut seen = HashSet::new();
    let mut listings = Vec::new();
    for page in pages {
        let html = fetch_text(&client, page).await?;
        for entry in parse_listing_page(page, &html) {
            if seen.insert(entry.detail_url.clone()) {
                listings.push(entry);
            }
        }
    }

    let detail_entries: Vec<_> = listings.into_iter().take(72).collect();
    let mut join_set = JoinSet::new();
    let mut queued = detail_entries.into_iter();
    for _ in 0..8 {
        let Some(entry) = queued.next() else {
            break;
        };
        let client = client.clone();
        join_set.spawn(async move { fetch_detail_item(client, entry).await });
    }

    let mut items = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        match joined {
            Ok(Ok(Some(item))) => items.push(item),
            Ok(Ok(None)) => {}
            Ok(Err(error)) => {
                log::warn!("抓取 xb6v 详情失败: {}", error);
            }
            Err(error) => {
                log::warn!("xb6v 详情任务失败: {}", error);
            }
        }

        if let Some(entry) = queued.next() {
            let client = client.clone();
            join_set.spawn(async move { fetch_detail_item(client, entry).await });
        }
    }

    Ok(items)
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
    let response = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        )
        .send()
        .await
        .map_err(|e| e.to_string())?;
    response.text().await.map_err(|e| e.to_string())
}

async fn fetch_detail_item(
    client: reqwest::Client,
    entry: ListingEntry,
) -> Result<Option<ScrapedCatalogItem>, String> {
    let detail_html = fetch_text(&client, &entry.detail_url).await?;
    Ok(parse_detail_page(&entry.detail_url, &detail_html, &entry))
}

fn parse_listing_page(page_url: &str, html: &str) -> Vec<ListingEntry> {
    let anchor_regex = Regex::new(r#"<a[^>]+href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
    let tag_regex = Regex::new(r"<[^>]+>").unwrap();
    let mut entries = Vec::new();

    for capture in anchor_regex.captures_iter(html) {
        let Some(href) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        if !href.ends_with(".html") || href.contains('#') {
            continue;
        }

        let Some(raw_label) = capture.get(2).map(|value| value.as_str()) else {
            continue;
        };
        let title = html_escape_decode(tag_regex.replace_all(raw_label, " ").trim());
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        if title.is_empty() || title == "评论" {
            continue;
        }

        let detail_url = absolutize_url(page_url, href);
        if !detail_url.contains("xb6v.com") {
            continue;
        }

        entries.push(ListingEntry {
            title,
            item_type: infer_item_type(&detail_url),
            detail_url,
        });
    }

    entries
}

fn parse_detail_page(
    detail_url: &str,
    html: &str,
    entry: &ListingEntry,
) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<title>([^<]+)</title>"#).unwrap();
    let meta_regex = Regex::new(r#"<meta\s+name="description"\s+content="([^"]+)""#).unwrap();
    let poster_regex = Regex::new(r#"https?://[^"'<> ]+\.(?:jpg|jpeg|png)"#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1).map(|value| value.as_str()))
        .map(|value| value.split('-').next().unwrap_or(value).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| entry.title.clone());

    let summary = meta_regex
        .captures(html)
        .and_then(|capture| capture.get(1).map(|value| html_escape_decode(value.as_str())))
        .filter(|value| !value.is_empty());

    let poster = poster_regex
        .find_iter(html)
        .map(|m| m.as_str().to_string())
        .find(|url| url.contains("66tutup.com") || url.contains("xb6v"));

    let episodes = parse_play_episodes(detail_url, html);

    Some(ScrapedCatalogItem {
        source_item_key: detail_url.to_string(),
        title,
        item_type: entry.item_type.clone(),
        poster,
        summary,
        detail_json: Some(format!(r#"{{"source":"xb6v","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

fn parse_play_episodes(detail_url: &str, html: &str) -> Vec<ScrapedCatalogEpisode> {
    let heading_regex = Regex::new(r#"<h3>([^<]+)</h3>"#).unwrap();
    let anchor_regex = Regex::new(r#"(?s)<a\b([^>]*)>(.*?)</a>"#).unwrap();
    let title_regex = Regex::new(r#"title=['"]([^'"]+)['"]"#).unwrap();
    let href_regex = Regex::new(r#"href\s*=\s*['"]([^'"]+)['"]"#).unwrap();
    let mut episodes = Vec::new();
    let mut seen_urls = HashSet::new();

    for section in html.split(r#"<div class="widget box row">"#).skip(1) {
        let heading = heading_regex
            .captures(section)
            .and_then(|captures| captures.get(1).map(|value| value.as_str()))
            .map(|value| html_escape_decode(value).trim().to_string())
            .unwrap_or_else(|| "在线播放".to_string());

        for anchor in anchor_regex.captures_iter(section) {
            let attrs = anchor.get(1).map(|value| value.as_str()).unwrap_or_default();
            let label = title_regex
                .captures(attrs)
                .and_then(|captures| captures.get(1).map(|value| value.as_str()))
                .map(|value| html_escape_decode(value).trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "播放".to_string());
            let href = href_regex
                .captures(attrs)
                .and_then(|captures| captures.get(1).map(|value| value.as_str()))
                .map(|value| absolutize_url(detail_url, value))
                .unwrap_or_default();
            if href.is_empty()
                || !href.contains("/e/DownSys/play/")
                || !seen_urls.insert(href.clone())
            {
                continue;
            }

            episodes.push(ScrapedCatalogEpisode {
                source_name: heading.clone(),
                episode_label: label,
                play_url: href,
                order_index: episodes.len() as i64,
            });
        }
    }

    if !episodes.is_empty() {
        return episodes;
    }

    parse_download_episodes(html)
}

fn parse_download_episodes(html: &str) -> Vec<ScrapedCatalogEpisode> {
    let link_regex = Regex::new(r#"(magnet:\?[^"'<> ]+|ed2k://[^"'<> ]+|thunder://[^"'<> ]+)"#).unwrap();
    let mut seen_urls = HashSet::new();

    link_regex
        .find_iter(html)
        .filter_map(|m| {
            let url = m.as_str().to_string();
            if !seen_urls.insert(url.clone()) {
                return None;
            }
            Some(url)
        })
        .enumerate()
        .map(|(index, url)| ScrapedCatalogEpisode {
            source_name: "下载地址".to_string(),
            episode_label: format!("资源 {}", index + 1),
            play_url: url,
            order_index: index as i64,
        })
        .collect()
}

fn infer_item_type(detail_url: &str) -> String {
    if detail_url.contains("/dianshiju/") {
        "series".to_string()
    } else if detail_url.contains("/ZongYi/") {
        "variety".to_string()
    } else if detail_url.contains("/donghuapian/") {
        "anime".to_string()
    } else {
        "movie".to_string()
    }
}

fn absolutize_url(page_url: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        href.to_string()
    } else if href.starts_with('/') {
        let base = reqwest::Url::parse(page_url).unwrap();
        format!("{}://{}{}", base.scheme(), base.host_str().unwrap_or_default(), href)
    } else {
        reqwest::Url::parse(page_url)
            .and_then(|base| base.join(href))
            .map(|url| url.to_string())
            .unwrap_or_else(|_| href.to_string())
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
        infer_item_type, parse_detail_page, parse_listing_page, parse_play_episodes, ListingEntry,
    };

    #[test]
    fn parses_xb6v_listing_entries() {
        let html = r#"
            <a href="/dianshiju/oumeiju/11308.html">亢奋[第三季]</a>
            <a href="/juqingpian/28598.html">我的阿米什人双重生活</a>
            <a href="/ZongYi/28518.html">乘风2026</a>
            <a href="/juqingpian/28598.html#respond">评论</a>
        "#;
        let entries = parse_listing_page("https://www.xb6v.com/", html);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].item_type, "series");
        assert_eq!(entries[1].item_type, "movie");
        assert_eq!(entries[2].item_type, "variety");
    }

    #[test]
    fn parses_xb6v_detail_page() {
        let html = r#"
            <title>我的阿米什人双重生活-新版6v电影（旧版66影视）- 免费电影下载</title>
            <meta name="description" content="◎简　　介　剧情简介测试" />
            <img src="https://www.66tutup.com/2026/0267.jpg" />
            <div class="widget box row"><h3>播放地址（无需安装插件）</h3>
              <a title='HD' href="/e/DownSys/play/?classid=17&id=28598&pathid2=0&bf=1" target="_blank" class="lBtn">HD</a>
            </div>
        "#;
        let entry = ListingEntry {
            title: "我的阿米什人双重生活".to_string(),
            detail_url: "https://www.xb6v.com/juqingpian/28598.html".to_string(),
            item_type: "movie".to_string(),
        };
        let item = parse_detail_page(&entry.detail_url, html, &entry).expect("detail should parse");
        assert_eq!(item.title, "我的阿米什人双重生活");
        assert_eq!(item.item_type, "movie");
        assert_eq!(item.poster.as_deref(), Some("https://www.66tutup.com/2026/0267.jpg"));
        assert_eq!(item.episodes.len(), 1);
        assert_eq!(item.episodes[0].source_name, "播放地址（无需安装插件）");
        assert_eq!(item.episodes[0].episode_label, "HD");
        assert!(item.episodes[0].play_url.contains("/e/DownSys/play/"));
    }

    #[test]
    fn parses_grouped_play_episodes() {
        let html = r#"
            <div class="widget box row"><h3>播放地址（无插件 极速播放）</h3>
              <a title='第01集' href="/e/DownSys/play/?classid=8&id=11308&pathid1=0&bf=0" target="_blank" class="lBtn">第01集</a>
              <a title='第02集' href="/e/DownSys/play/?classid=8&id=11308&pathid1=1&bf=0" target="_blank" class="lBtn">第02集</a>
            </div>
            <div class="widget box row"><h3>播放地址（无需安装插件）</h3>
              <a title='第01集' href="/e/DownSys/play/?classid=8&id=11308&pathid2=0&bf=1" target="_blank" class="lBtn">第01集</a>
            </div>
        "#;
        let episodes =
            parse_play_episodes("https://www.xb6v.com/dianshiju/oumeiju/11308.html", html);
        assert_eq!(episodes.len(), 3);
        assert_eq!(episodes[0].source_name, "播放地址（无插件 极速播放）");
        assert_eq!(episodes[1].episode_label, "第02集");
        assert_eq!(episodes[2].source_name, "播放地址（无需安装插件）");
    }

    #[test]
    fn infers_item_type_from_url() {
        assert_eq!(infer_item_type("https://www.xb6v.com/dianshiju/oumeiju/11308.html"), "series");
        assert_eq!(infer_item_type("https://www.xb6v.com/ZongYi/28518.html"), "variety");
        assert_eq!(infer_item_type("https://www.xb6v.com/donghuapian/28580.html"), "anime");
        assert_eq!(infer_item_type("https://www.xb6v.com/juqingpian/28598.html"), "movie");
    }
}
