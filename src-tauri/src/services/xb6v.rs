use crate::services::auete::{is_auete_site, scrape_auete_catalog, scrape_auete_detail};
use crate::services::guard::{guard_adapter_key, is_guard_site_supported};
use crate::services::jianpian::{
    is_jianpian_site, parse_detail_page as parse_jianpian_detail_page,
    parse_listing_page as parse_jianpian_listing_page, JianpianListingEntry,
};
use crate::services::libvio::{is_libvio_site, scrape_libvio_catalog, scrape_libvio_detail};
use crate::services::playback_runtime::build_runtime_target;
use crate::services::PlaybackTarget;
use crate::services::tvbox::TvboxSiteRecord;
use crate::services::wencai::{
    is_wencai_site, parse_detail_page as parse_wencai_detail_page,
    parse_listing_page as parse_wencai_listing_page, WencaiListingEntry,
};
use crate::services::zxzj::{is_zxzj_site, scrape_zxzj_catalog, scrape_zxzj_detail};
use regex::Regex;
use serde_json::Value;
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

pub fn runtime_targets_for_item(
    item: &ScrapedCatalogItem,
    source_key: &str,
) -> Vec<PlaybackTarget> {
    item.episodes
        .iter()
        .enumerate()
        .map(|(index, episode)| {
            let mut target =
                build_runtime_target(&episode.play_url, source_key, Some((index + 1) as i64));
            target.sort_hint = episode.order_index as i32;
            target.meta = Some(format!("{}:{}", episode.source_name, episode.episode_label));
            target
        })
        .collect()
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
    if sites.iter().any(is_libvio_site) {
        items.extend(scrape_libvio_catalog().await?);
    }
    if sites.iter().any(is_auete_site) {
        items.extend(scrape_auete_catalog().await?);
    }
    if sites.iter().any(is_zxzj_site) {
        items.extend(scrape_zxzj_catalog().await?);
    }
    if sites.iter().any(is_guard_site_supported) {
        items.extend(scrape_supported_guard_catalogs(sites).await?);
    }
    if sites.iter().any(is_wencai_site) {
        items.extend(scrape_wencai_catalog(sites).await?);
    }
    if sites.iter().any(is_jianpian_site) {
        items.extend(scrape_jianpian_catalog(sites).await?);
    }
    Ok(items)
}

pub async fn scrape_catalog_detail_from_json(
    detail_json: &str,
) -> Result<Option<ScrapedCatalogItem>, String> {
    let detail: Value = serde_json::from_str(detail_json).map_err(|e| e.to_string())?;
    let source = detail
        .get("source")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "catalog detail source is missing".to_string())?;

    match source {
        "guard" => {
            let guard_key = detail
                .get("guard_key")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "guard detail missing guard_key".to_string())?;
            let site_key = detail
                .get("site_key")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "guard detail missing site_key".to_string())?;
            let item_id = detail
                .get("item_id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "guard detail missing item_id".to_string())?;
            let expected_item_type = detail
                .get("item_type")
                .and_then(|value| value.as_str())
                .unwrap_or("movie");
            resolve_guard_detail(guard_key, site_key, item_id, expected_item_type).await
        }
        "xb6v" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            scrape_xb6v_detail(url).await
        }
        "libvio" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            let mut item = scrape_libvio_detail(url).await?;
            if let Some(expected_type) = detail.get("item_type").and_then(|value| value.as_str()) {
                if let Some(item) = item.as_mut() {
                    item.item_type = expected_type.to_string();
                }
            }
            Ok(item)
        }
        "auete" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            let mut item = scrape_auete_detail(url).await?;
            if let Some(expected_type) = detail.get("item_type").and_then(|value| value.as_str()) {
                if let Some(item) = item.as_mut() {
                    item.item_type = expected_type.to_string();
                }
            }
            Ok(item)
        }
        "zxzj" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            let mut item = scrape_zxzj_detail(url).await?;
            if let Some(expected_type) = detail.get("item_type").and_then(|value| value.as_str()) {
                if let Some(item) = item.as_mut() {
                    item.item_type = expected_type.to_string();
                }
            }
            Ok(item)
        }
        "wencai" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            let mut item = scrape_wencai_detail(url).await?;
            if let Some(expected_type) = detail.get("item_type").and_then(|value| value.as_str()) {
                if let Some(item) = item.as_mut() {
                    item.item_type = expected_type.to_string();
                }
            }
            Ok(item)
        }
        "jianpian" => {
            let url = detail
                .get("url")
                .and_then(|value| value.as_str())
                .ok_or_else(|| "catalog detail url is missing".to_string())?;
            let mut item = scrape_jianpian_detail(url).await?;
            if let Some(expected_type) = detail.get("item_type").and_then(|value| value.as_str()) {
                if let Some(item) = item.as_mut() {
                    item.item_type = expected_type.to_string();
                }
            }
            Ok(item)
        }
        other => Err(format!("unsupported catalog detail source: {other}")),
    }
}

fn collect_supported_guard_keys(sites: &[TvboxSiteRecord]) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if sites
        .iter()
        .any(|site| guard_adapter_key(site).as_deref() == Some("csp_JpysGuard"))
    {
        keys.push("csp_JpysGuard");
    }
    if sites
        .iter()
        .any(|site| guard_adapter_key(site).as_deref() == Some("csp_JPJGuard"))
    {
        keys.push("csp_JPJGuard");
    }
    keys
}

async fn scrape_supported_guard_catalogs(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    for key in collect_supported_guard_keys(sites) {
        let Some(site_key) = first_site_key_for_guard(sites, key) else {
            continue;
        };
        for (item_type, page_url) in guard_catalog_pages(key) {
            let html = match fetch_text(&client, &page_url).await {
                Ok(value) => value,
                Err(error) => {
                    log::warn!("抓取 Guard 目录失败 {} {}: {}", key, page_url, error);
                    continue;
                }
            };
            match key {
                "csp_JpysGuard" => {
                    for entry in parse_wencai_listing_page(&page_url, &item_type, &html) {
                        let Some(item) = shallow_item_from_wencai_guard_entry(key, site_key, entry)
                        else {
                            continue;
                        };
                        if seen.insert(item.source_item_key.clone()) {
                            items.push(item);
                        }
                    }
                }
                "csp_JPJGuard" => {
                    for entry in parse_jianpian_listing_page(&page_url, &item_type, &html) {
                        let Some(item) =
                            shallow_item_from_jianpian_guard_entry(key, site_key, entry)
                        else {
                            continue;
                        };
                        if seen.insert(item.source_item_key.clone()) {
                            items.push(item);
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(items)
}

async fn resolve_guard_detail(
    guard_key: &str,
    site_key: &str,
    item_id: &str,
    expected_item_type: &str,
) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let Some(detail_url) = guard_detail_url(guard_key, item_id) else {
        return Ok(None);
    };
    let html = fetch_text(&client, &detail_url).await?;
    match guard_key {
        "csp_JpysGuard" => Ok(parse_wencai_detail_page(&detail_url, &html).map(|item| {
            retag_guard_detail(item, guard_key, site_key, item_id, expected_item_type)
        })),
        "csp_JPJGuard" => Ok(parse_jianpian_detail_page(&detail_url, &html).map(|item| {
            retag_guard_detail(item, guard_key, site_key, item_id, expected_item_type)
        })),
        other => Err(format!("unsupported guard detail dispatch: {other}")),
    }
}

fn first_site_key_for_guard<'a>(sites: &'a [TvboxSiteRecord], guard_key: &str) -> Option<&'a str> {
    sites.iter()
        .find(|site| guard_adapter_key(site).as_deref() == Some(guard_key))
        .map(|site| site.site_key.as_str())
}

fn shallow_item_from_wencai_guard_entry(
    guard_key: &str,
    site_key: &str,
    entry: WencaiListingEntry,
) -> Option<ScrapedCatalogItem> {
    let item_id = extract_guard_item_id(guard_key, &entry.detail_url)?;
    Some(guard_shallow_item(
        guard_key,
        site_key,
        item_id,
        entry.title,
        entry.item_type,
        entry.poster,
        None,
    ))
}

fn shallow_item_from_jianpian_guard_entry(
    guard_key: &str,
    site_key: &str,
    entry: JianpianListingEntry,
) -> Option<ScrapedCatalogItem> {
    let item_id = extract_guard_item_id(guard_key, &entry.detail_url)?;
    Some(guard_shallow_item(
        guard_key,
        site_key,
        item_id,
        entry.title,
        entry.item_type,
        entry.poster,
        None,
    ))
}

fn guard_shallow_item(
    guard_key: &str,
    site_key: &str,
    item_id: String,
    title: String,
    item_type: String,
    poster: Option<String>,
    summary: Option<String>,
) -> ScrapedCatalogItem {
    ScrapedCatalogItem {
        source_item_key: format!("guard:{site_key}:{item_id}"),
        title,
        item_type: item_type.clone(),
        poster,
        summary,
        detail_json: Some(format!(
            r#"{{"source":"guard","guard_key":"{}","site_key":"{}","item_id":"{}","item_type":"{}"}}"#,
            guard_key, site_key, item_id, item_type
        )),
        episodes: Vec::new(),
    }
}

fn guard_catalog_pages(guard_key: &str) -> Vec<(String, String)> {
    match guard_key {
        "csp_JpysGuard" => vec![
            ("movie".to_string(), "https://www.deeyy.com/vod/type/id/1.html".to_string()),
            ("series".to_string(), "https://www.deeyy.com/vod/type/id/2.html".to_string()),
            ("series".to_string(), "https://www.deeyy.com/vod/type/id/3.html".to_string()),
            ("anime".to_string(), "https://www.deeyy.com/vod/type/id/4.html".to_string()),
        ],
        "csp_JPJGuard" => vec![
            ("movie".to_string(), "https://jpvod.com/type/1.html".to_string()),
            ("series".to_string(), "https://jpvod.com/type/2.html".to_string()),
            ("variety".to_string(), "https://jpvod.com/type/3.html".to_string()),
            ("anime".to_string(), "https://jpvod.com/type/4.html".to_string()),
            ("series".to_string(), "https://jpvod.com/type/27.html".to_string()),
        ],
        _ => Vec::new(),
    }
}

fn guard_detail_url(guard_key: &str, item_id: &str) -> Option<String> {
    match guard_key {
        "csp_JpysGuard" => Some(format!("https://www.deeyy.com/vod/detail/id/{item_id}.html")),
        "csp_JPJGuard" => Some(format!("https://jpvod.com/vod/{item_id}.html")),
        _ => None,
    }
}

fn extract_guard_item_id(guard_key: &str, detail_url: &str) -> Option<String> {
    let regex = match guard_key {
        "csp_JpysGuard" => Regex::new(r#"/vod/detail/id/(\d+)\.html"#).unwrap(),
        "csp_JPJGuard" => Regex::new(r#"/vod/(\d+)\.html"#).unwrap(),
        _ => return None,
    };
    regex
        .captures(detail_url)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string())
}

fn extract_guard_play_parts(
    guard_key: &str,
    play_url: &str,
    item_id: &str,
) -> Option<(String, String)> {
    match guard_key {
        "csp_JpysGuard" => {
            let regex = Regex::new(r#"/vod/play/id/(\d+)/sid/(\d+)/nid/(\d+)\.html"#).unwrap();
            let captures = regex.captures(play_url)?;
            if captures.get(1)?.as_str() != item_id {
                return None;
            }
            Some((
                captures.get(2)?.as_str().to_string(),
                captures.get(3)?.as_str().to_string(),
            ))
        }
        "csp_JPJGuard" => {
            let regex = Regex::new(r#"/play/(\d+)-(\d+)-(\d+)\.html"#).unwrap();
            let captures = regex.captures(play_url)?;
            if captures.get(1)?.as_str() != item_id {
                return None;
            }
            Some((
                captures.get(2)?.as_str().to_string(),
                captures.get(3)?.as_str().to_string(),
            ))
        }
        _ => None,
    }
}

fn retag_guard_detail(
    mut item: ScrapedCatalogItem,
    guard_key: &str,
    site_key: &str,
    item_id: &str,
    expected_item_type: &str,
) -> ScrapedCatalogItem {
    item.episodes = item
        .episodes
        .into_iter()
        .filter_map(|episode| {
            let (source_id, episode_id) =
                extract_guard_play_parts(guard_key, &episode.play_url, item_id)?;
            Some(ScrapedCatalogEpisode {
                play_url: crate::services::encode_guard_play_target(
                    guard_key,
                    site_key,
                    item_id,
                    &source_id,
                    &episode_id,
                ),
                ..episode
            })
        })
        .collect();
    item.source_item_key = format!("guard:{site_key}:{item_id}");
    item.item_type = expected_item_type.to_string();
    item.detail_json = Some(format!(
        r#"{{"source":"guard","guard_key":"{}","site_key":"{}","item_id":"{}","item_type":"{}"}}"#,
        guard_key, site_key, item_id, expected_item_type
    ));
    item
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
            Ok(Ok(Some(item))) => {
                if item
                    .episodes
                    .iter()
                    .any(|episode| episode.play_url.contains("/e/DownSys/play/"))
                {
                    items.push(item);
                }
            }
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

async fn scrape_xb6v_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    let entry = ListingEntry {
        title: String::new(),
        detail_url: detail_url.to_string(),
        item_type: infer_item_type(detail_url),
    };
    Ok(parse_detail_page(detail_url, &html, &entry))
}

async fn scrape_wencai_catalog(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let pages = collect_site_roots(sites, is_wencai_site)
        .into_iter()
        .flat_map(|root| {
            [
                ("movie".to_string(), root.clone()),
                ("series".to_string(), format!("{root}dianshiju/")),
                ("variety".to_string(), format!("{root}zongyi/")),
                ("anime".to_string(), format!("{root}dongman/")),
            ]
        })
        .collect::<Vec<_>>();

    scrape_shallow_catalog_pages(
        &client,
        pages,
        parse_wencai_listing_page,
        shallow_item_from_wencai_entry,
    )
    .await
}

async fn scrape_wencai_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    Ok(parse_wencai_detail_page(detail_url, &html))
}

async fn scrape_jianpian_catalog(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let pages = collect_site_roots(sites, is_jianpian_site)
        .into_iter()
        .flat_map(|root| {
            [
                ("movie".to_string(), format!("{root}type/1.html")),
                ("series".to_string(), format!("{root}type/2.html")),
                ("variety".to_string(), format!("{root}type/3.html")),
                ("anime".to_string(), format!("{root}type/4.html")),
            ]
        })
        .collect::<Vec<_>>();

    scrape_shallow_catalog_pages(
        &client,
        pages,
        parse_jianpian_listing_page,
        shallow_item_from_jianpian_entry,
    )
    .await
}

async fn scrape_jianpian_detail(detail_url: &str) -> Result<Option<ScrapedCatalogItem>, String> {
    let client = build_client()?;
    let html = fetch_text(&client, detail_url).await?;
    Ok(parse_jianpian_detail_page(detail_url, &html))
}

fn build_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .no_proxy()
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())
}

async fn scrape_shallow_catalog_pages<Entry, ParseListing, BuildItem>(
    client: &reqwest::Client,
    pages: Vec<(String, String)>,
    parse_listing: ParseListing,
    build_item: BuildItem,
) -> Result<Vec<ScrapedCatalogItem>, String>
where
    Entry: Clone,
    ParseListing: Fn(&str, &str, &str) -> Vec<Entry>,
    BuildItem: Fn(Entry) -> ScrapedCatalogItem,
{
    let mut seen = HashSet::new();
    let mut items = Vec::new();

    for (item_type, page_url) in pages {
        let html = fetch_text(client, &page_url).await?;
        for entry in parse_listing(&page_url, &item_type, &html) {
            let item = build_item(entry);
            if seen.insert(item.source_item_key.clone()) {
                items.push(item);
            }
        }
    }

    Ok(items)
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

fn collect_site_roots(
    sites: &[TvboxSiteRecord],
    matcher: fn(&TvboxSiteRecord) -> bool,
) -> Vec<String> {
    let mut roots = Vec::new();
    let mut seen = HashSet::new();

    for site in sites.iter().filter(|site| matcher(site)) {
        for candidate in [
            site.ext.as_deref(),
            site.api.as_deref(),
            extract_first_url(&site.raw_json).as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(root) = derive_browse_root(candidate) {
                if seen.insert(root.clone()) {
                    roots.push(root);
                }
            }
        }
    }

    roots
}

fn derive_browse_root(candidate: &str) -> Option<String> {
    let url = reqwest::Url::parse(candidate.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    let mut kept_segments = Vec::new();
    let path_segments = url
        .path_segments()
        .into_iter()
        .flatten()
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_string())
        .collect::<Vec<_>>();

    for segment in path_segments {
        let normalized = segment.to_ascii_lowercase();
        if is_catalog_boundary_segment(&normalized) || is_file_like_segment(&normalized) {
            break;
        }
        kept_segments.push(segment);
    }

    let mut root = url;
    root.set_query(None);
    root.set_fragment(None);
    let new_path = if kept_segments.is_empty() {
        "/".to_string()
    } else {
        format!("/{}/", kept_segments.join("/"))
    };
    root.set_path(&new_path);
    Some(root.to_string())
}

fn is_catalog_boundary_segment(segment: &str) -> bool {
    matches!(
        segment,
        "detail"
            | "voddetail"
            | "play"
            | "vodplay"
            | "type"
            | "list"
            | "movie"
            | "tv"
            | "series"
            | "variety"
            | "anime"
            | "show"
            | "dianshiju"
            | "zongyi"
            | "dongman"
            | "donghua"
    )
}

fn is_file_like_segment(segment: &str) -> bool {
    segment.ends_with(".html")
        || segment.ends_with(".php")
        || segment.ends_with(".json")
        || segment.ends_with(".js")
        || segment.ends_with(".txt")
        || segment.ends_with(".xml")
        || segment.ends_with(".m3u")
        || segment.ends_with(".m3u8")
}

fn extract_first_url(raw_json: &str) -> Option<String> {
    let regex = Regex::new(r#"https?://[^"'\s,}]+""#).unwrap();
    regex
        .find(raw_json)
        .map(|matched| matched.as_str().trim_end_matches('"').to_string())
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
        if detail_url.contains("/index_") || detail_url.ends_with("/qian50m.html") {
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
        .and_then(|capture| {
            capture
                .get(1)
                .map(|value| html_escape_decode(value.as_str()))
        })
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
            let attrs = anchor
                .get(1)
                .map(|value| value.as_str())
                .unwrap_or_default();
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

    Vec::new()
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
        format!(
            "{}://{}{}",
            base.scheme(),
            base.host_str().unwrap_or_default(),
            href
        )
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

fn shallow_item_from_wencai_entry(entry: WencaiListingEntry) -> ScrapedCatalogItem {
    let item_type = entry.item_type.clone();
    ScrapedCatalogItem {
        source_item_key: entry.detail_url.clone(),
        title: entry.title,
        item_type: item_type.clone(),
        poster: entry.poster,
        summary: None,
        detail_json: Some(format!(
            r#"{{"source":"wencai","url":"{}","item_type":"{}"}}"#,
            entry.detail_url, item_type
        )),
        episodes: Vec::new(),
    }
}

fn shallow_item_from_jianpian_entry(entry: JianpianListingEntry) -> ScrapedCatalogItem {
    let item_type = entry.item_type.clone();
    ScrapedCatalogItem {
        source_item_key: entry.detail_url.clone(),
        title: entry.title,
        item_type: item_type.clone(),
        poster: entry.poster,
        summary: None,
        detail_json: Some(format!(
            r#"{{"source":"jianpian","url":"{}","item_type":"{}"}}"#,
            entry.detail_url, item_type
        )),
        episodes: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        collect_site_roots, collect_supported_guard_keys, derive_browse_root,
        extract_guard_item_id, extract_guard_play_parts, guard_catalog_pages, guard_detail_url,
        infer_item_type,
        parse_detail_page, parse_listing_page, parse_play_episodes, runtime_targets_for_item,
        scrape_catalog_detail_from_json, scrape_supported_tvbox_catalogs, ListingEntry,
        ScrapedCatalogEpisode, ScrapedCatalogItem,
    };
    use crate::services::{decode_guard_play_target, PlaybackTargetKind};

    #[test]
    fn parses_xb6v_listing_entries() {
        let html = r#"
            <a href="/dianshiju/oumeiju/11308.html">亢奋[第三季]</a>
            <a href="/juqingpian/28598.html">我的阿米什人双重生活</a>
            <a href="/ZongYi/28518.html">乘风2026</a>
            <a href="/dianshiju/index_2.html">电视剧</a>
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
        assert_eq!(
            item.poster.as_deref(),
            Some("https://www.66tutup.com/2026/0267.jpg")
        );
        assert_eq!(item.episodes.len(), 1);
        assert_eq!(item.episodes[0].source_name, "播放地址（无需安装插件）");
        assert_eq!(item.episodes[0].episode_label, "HD");
        assert!(item.episodes[0].play_url.contains("/e/DownSys/play/"));
    }

    #[test]
    fn returns_empty_when_detail_has_only_download_links() {
        let html = r#"
            <title>丹凤眼-新版6v电影（旧版66影视）- 免费电影下载</title>
            <a href="magnet:?xt=urn:btih:demo">下载1</a>
        "#;
        let entry = ListingEntry {
            title: "丹凤眼".to_string(),
            detail_url: "https://www.xb6v.com/juqingpian/28574.html".to_string(),
            item_type: "movie".to_string(),
        };
        let item = parse_detail_page(&entry.detail_url, html, &entry).expect("detail should parse");
        assert!(item.episodes.is_empty());
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
        assert_eq!(
            infer_item_type("https://www.xb6v.com/dianshiju/oumeiju/11308.html"),
            "series"
        );
        assert_eq!(
            infer_item_type("https://www.xb6v.com/ZongYi/28518.html"),
            "variety"
        );
        assert_eq!(
            infer_item_type("https://www.xb6v.com/donghuapian/28580.html"),
            "anime"
        );
        assert_eq!(
            infer_item_type("https://www.xb6v.com/juqingpian/28598.html"),
            "movie"
        );
    }

    #[test]
    fn selects_wencai_source_from_tvbox_sites() {
        let site = crate::services::tvbox::TvboxSiteRecord {
            site_key: "文采".to_string(),
            site_name: "💮文采┃秒播".to_string(),
            api: None,
            ext: Some("https://www.wencai.example/".to_string()),
            searchable: true,
            quick_search: false,
            filterable: false,
            source_type: "custom".to_string(),
            raw_json: "{}".to_string(),
        };

        assert!(crate::services::is_wencai_site(&site));
        assert_eq!(
            collect_site_roots(&[site], crate::services::is_wencai_site),
            vec!["https://www.wencai.example/".to_string()]
        );
    }

    #[test]
    fn selects_jianpian_source_from_tvbox_sites() {
        let site = crate::services::tvbox::TvboxSiteRecord {
            site_key: "荐片".to_string(),
            site_name: "⚔️荐片┃手机".to_string(),
            api: None,
            ext: Some("https://www.jianpian.example".to_string()),
            searchable: true,
            quick_search: false,
            filterable: false,
            source_type: "custom".to_string(),
            raw_json: "{}".to_string(),
        };

        assert!(crate::services::is_jianpian_site(&site));
        assert_eq!(
            collect_site_roots(&[site], crate::services::is_jianpian_site),
            vec!["https://www.jianpian.example/".to_string()]
        );
    }

    #[test]
    fn derives_browse_root_from_config_and_api_urls() {
        assert_eq!(
            derive_browse_root("https://www.wencai.example/config.json").as_deref(),
            Some("https://www.wencai.example/")
        );
        assert_eq!(
            derive_browse_root("https://www.jianpian.example/api.php?ac=list").as_deref(),
            Some("https://www.jianpian.example/")
        );
        assert_eq!(
            derive_browse_root("https://cdn.example.com/tvbox/wencai/config.js").as_deref(),
            Some("https://cdn.example.com/tvbox/wencai/")
        );
    }

    #[test]
    fn derives_browse_root_from_catalog_and_detail_urls() {
        assert_eq!(
            derive_browse_root("https://www.jianpian.example/type/1.html").as_deref(),
            Some("https://www.jianpian.example/")
        );
        assert_eq!(
            derive_browse_root("https://www.jianpian.example/voddetail/888.html").as_deref(),
            Some("https://www.jianpian.example/")
        );
        assert_eq!(
            derive_browse_root("https://www.wencai.example/dianshiju/123.html").as_deref(),
            Some("https://www.wencai.example/")
        );
    }

    #[test]
    fn selects_guard_sites_from_tvbox_records() {
        let sites = vec![
            crate::services::tvbox::TvboxSiteRecord {
                site_key: "文采".to_string(),
                site_name: "💮文采┃秒播".to_string(),
                api: Some("csp_JpysGuard".to_string()),
                ext: None,
                searchable: true,
                quick_search: true,
                filterable: false,
                source_type: "3".to_string(),
                raw_json: "{}".to_string(),
            },
            crate::services::tvbox::TvboxSiteRecord {
                site_key: "贱贱".to_string(),
                site_name: "🐭荐片┃P2P".to_string(),
                api: Some("csp_JPJGuard".to_string()),
                ext: None,
                searchable: true,
                quick_search: true,
                filterable: false,
                source_type: "3".to_string(),
                raw_json: "{}".to_string(),
            },
        ];

        assert_eq!(
            collect_supported_guard_keys(&sites),
            vec!["csp_JpysGuard", "csp_JPJGuard"]
        );
    }

    #[test]
    fn builds_default_guard_catalog_pages() {
        assert_eq!(
            guard_catalog_pages("csp_JpysGuard"),
            vec![
                ("movie".to_string(), "https://www.deeyy.com/vod/type/id/1.html".to_string()),
                ("series".to_string(), "https://www.deeyy.com/vod/type/id/2.html".to_string()),
                ("series".to_string(), "https://www.deeyy.com/vod/type/id/3.html".to_string()),
                ("anime".to_string(), "https://www.deeyy.com/vod/type/id/4.html".to_string()),
            ]
        );
        assert_eq!(
            guard_catalog_pages("csp_JPJGuard"),
            vec![
                ("movie".to_string(), "https://jpvod.com/type/1.html".to_string()),
                ("series".to_string(), "https://jpvod.com/type/2.html".to_string()),
                ("variety".to_string(), "https://jpvod.com/type/3.html".to_string()),
                ("anime".to_string(), "https://jpvod.com/type/4.html".to_string()),
                ("series".to_string(), "https://jpvod.com/type/27.html".to_string()),
            ]
        );
    }

    #[test]
    fn derives_guard_detail_urls_and_item_ids() {
        assert_eq!(
            guard_detail_url("csp_JpysGuard", "1419").as_deref(),
            Some("https://www.deeyy.com/vod/detail/id/1419.html")
        );
        assert_eq!(
            guard_detail_url("csp_JPJGuard", "97910").as_deref(),
            Some("https://jpvod.com/vod/97910.html")
        );
        assert_eq!(
            extract_guard_item_id("csp_JpysGuard", "https://www.deeyy.com/vod/detail/id/1419.html")
                .as_deref(),
            Some("1419")
        );
        assert_eq!(
            extract_guard_item_id("csp_JPJGuard", "https://jpvod.com/vod/97910.html")
                .as_deref(),
            Some("97910")
        );
        assert_eq!(
            extract_guard_play_parts(
                "csp_JpysGuard",
                "https://www.deeyy.com/vod/play/id/1419/sid/1/nid/1.html",
                "1419"
            ),
            Some(("1".to_string(), "1".to_string()))
        );
        assert_eq!(
            extract_guard_play_parts("csp_JPJGuard", "https://jpvod.com/play/97910-2-1.html", "97910"),
            Some(("2".to_string(), "1".to_string()))
        );
    }

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn scrapes_live_guard_catalogs_from_default_roots() {
        let sites = vec![
            crate::services::tvbox::TvboxSiteRecord {
                site_key: "文采".to_string(),
                site_name: "💮文采┃秒播".to_string(),
                api: Some("csp_JpysGuard".to_string()),
                ext: None,
                searchable: true,
                quick_search: true,
                filterable: false,
                source_type: "3".to_string(),
                raw_json: "{}".to_string(),
            },
            crate::services::tvbox::TvboxSiteRecord {
                site_key: "贱贱".to_string(),
                site_name: "🐭荐片┃P2P".to_string(),
                api: Some("csp_JPJGuard".to_string()),
                ext: None,
                searchable: true,
                quick_search: true,
                filterable: false,
                source_type: "3".to_string(),
                raw_json: "{}".to_string(),
            },
        ];

        let items = scrape_supported_tvbox_catalogs(&sites)
            .await
            .expect("live guard catalogs should parse");
        let guard_items = items
            .iter()
            .filter(|item| item.source_item_key.starts_with("guard:"))
            .count();
        println!("live_guard_catalog_items={guard_items}");
        assert!(guard_items >= 20, "expected live guard catalogs to yield many items");
    }

    #[tokio::test]
    #[ignore = "requires live network and DNS access"]
    async fn resolves_live_guard_detail_json_sources() {
        let wencai = scrape_catalog_detail_from_json(
            r#"{"source":"guard","guard_key":"csp_JpysGuard","site_key":"文采","item_id":"1419","item_type":"movie"}"#,
        )
        .await
        .expect("wencai guard detail should dispatch")
        .expect("wencai guard detail should exist");
        assert!(wencai.episodes.iter().all(|episode| episode.play_url.starts_with("guard://")));
        let wencai_target =
            decode_guard_play_target(&wencai.episodes[0].play_url).expect("guard play target");
        assert_eq!(wencai_target.guard_key, "csp_JpysGuard");
        println!("wencai episodes={}", wencai.episodes.len());
        assert!(!wencai.episodes.is_empty());

        let jianpian = scrape_catalog_detail_from_json(
            r#"{"source":"guard","guard_key":"csp_JPJGuard","site_key":"贱贱","item_id":"97910","item_type":"series"}"#,
        )
        .await
        .expect("jpj guard detail should dispatch")
        .expect("jpj guard detail should exist");
        assert!(jianpian
            .episodes
            .iter()
            .all(|episode| episode.play_url.starts_with("guard://")));
        let jianpian_target =
            decode_guard_play_target(&jianpian.episodes[0].play_url).expect("guard play target");
        assert_eq!(jianpian_target.guard_key, "csp_JPJGuard");
        println!("jianpian episodes={}", jianpian.episodes.len());
        assert!(!jianpian.episodes.is_empty());
    }

    #[test]
    fn keeps_guard_targets_resolvable_and_zxzj_targets_embedded() {
        let guard_item = ScrapedCatalogItem {
            source_item_key: "guard:荐片:97910".to_string(),
            title: "Guard Demo".to_string(),
            item_type: "series".to_string(),
            poster: None,
            summary: None,
            detail_json: Some(
                r#"{"source":"guard","guard_key":"csp_JPJGuard","site_key":"贱贱","item_id":"97910","item_type":"series"}"#
                    .to_string(),
            ),
            episodes: vec![ScrapedCatalogEpisode {
                source_name: "荐片".to_string(),
                episode_label: "第1集".to_string(),
                play_url: "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/97910/1/1".to_string(),
                order_index: 1,
            }],
        };
        let zxzj_item = ScrapedCatalogItem {
            source_item_key: "zxzj:4627".to_string(),
            title: "ZXZJ Demo".to_string(),
            item_type: "series".to_string(),
            poster: None,
            summary: None,
            detail_json: Some(
                r#"{"source":"zxzj","url":"https://www.zxzjhd.com/voddetail/4627.html"}"#
                    .to_string(),
            ),
            episodes: vec![ScrapedCatalogEpisode {
                source_name: "播放线路5".to_string(),
                episode_label: "第1集".to_string(),
                play_url: "https://www.zxzjhd.com/vodplay/4627-1-1.html".to_string(),
                order_index: 1,
            }],
        };

        let guard_targets = runtime_targets_for_item(&guard_item, "guard");
        let zxzj_targets = runtime_targets_for_item(&zxzj_item, "zxzj");

        assert_eq!(guard_targets.len(), 1);
        assert_eq!(guard_targets[0].target_kind, PlaybackTargetKind::Resolvable);
        assert_eq!(zxzj_targets.len(), 1);
        assert_eq!(zxzj_targets[0].target_kind, PlaybackTargetKind::Embedded);
    }
}
