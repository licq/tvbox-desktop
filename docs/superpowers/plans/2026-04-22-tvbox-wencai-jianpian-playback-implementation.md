# TVBox WenCai / JianPian Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `文采` and `荐片` as desktop-usable Fantaihard VOD sources, showing only lines that resolve to real streams and pass probe checks.

**Architecture:** Extend the existing TVBox VOD ingestion pipeline with two source-specific scraper modules, route those modules through the supported catalog/detail dispatcher, and add dedicated resolver branches that turn source play pages into fully probed stream candidates. Keep visibility policy centralized in backend filtering so the frontend continues to receive only desktop-usable lines.

**Tech Stack:** Rust, reqwest, regex, rusqlite, Tauri commands, Vue frontend build verification, cargo test

---

### Task 1: Add WenCai service skeleton with parser-first tests

**Files:**
- Create: `src-tauri/src/services/wencai.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/wencai.rs`

- [ ] **Step 1: Write failing parser tests in `src-tauri/src/services/wencai.rs`**

```rust
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
```

- [ ] **Step 2: Run the new WenCai tests to verify they fail**

Run:

```bash
cargo test -q parses_wencai_listing_entries --manifest-path src-tauri/Cargo.toml
cargo test -q parses_wencai_detail_page_and_filters_external_lines --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_wencai_player_url --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `src-tauri/src/services/wencai.rs` does not exist yet.

- [ ] **Step 3: Implement minimal WenCai parsing module**

```rust
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
    site.site_key.contains("文采")
        || site.site_name.contains("文采")
        || site.raw_json.contains("文采")
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<WencaiListingEntry> {
    let item_regex = Regex::new(
        r#"<a class="module-item-pic" href="([^"]+)" title="([^"]+)".*?<img[^>]+(?:data-original|src)="([^"]+)""#,
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
        let source_name = section.get(1)?.as_str().trim().to_string();
        if is_external_source(&source_name) {
            continue;
        }
        let body = section.get(2)?.as_str();
        for anchor in anchor_regex.captures_iter(body) {
            let play_url = absolutize_url(detail_url, anchor.get(1)?.as_str());
            if !play_url.contains("/play/") || !seen.insert(play_url.clone()) {
                continue;
            }
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label: anchor.get(2)?.as_str().trim().to_string(),
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
        .map(|value| value.as_str().replace(r#"\/"#, "/"))
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
```

- [ ] **Step 4: Export the WenCai module**

```rust
pub mod wencai;

pub use wencai::{extract_player_url as extract_wencai_player_url, is_wencai_site};
```

- [ ] **Step 5: Run the WenCai tests again**

Run:

```bash
cargo test -q parses_wencai_listing_entries --manifest-path src-tauri/Cargo.toml
cargo test -q parses_wencai_detail_page_and_filters_external_lines --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_wencai_player_url --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/wencai.rs src-tauri/src/services/mod.rs
git commit -m "feat: add wencai parsing service"
```

### Task 2: Add JianPian service skeleton with parser-first tests

**Files:**
- Create: `src-tauri/src/services/jianpian.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/jianpian.rs`

- [ ] **Step 1: Write failing parser tests in `src-tauri/src/services/jianpian.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::{extract_player_url, parse_detail_page, parse_listing_page};

    #[test]
    fn parses_jianpian_listing_entries() {
        let html = r#"
            <a class="public-list-exp" href="/voddetail/888.html" title="荐片示例">
              <img data-src="https://img.example.com/jianpian.jpg" />
            </a>
        "#;

        let entries = parse_listing_page("https://www.jianpian.example/type/1.html", "movie", html);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title, "荐片示例");
        assert_eq!(entries[0].detail_url, "https://www.jianpian.example/voddetail/888.html");
    }

    #[test]
    fn parses_jianpian_detail_page_and_keeps_play_pages_only() {
        let html = r#"
            <h1>荐片示例</h1>
            <div class="switch-box-item">荐片线路A</div>
            <div class="anthology-list-box">
              <a href="/vodplay/888-1-1.html">正片</a>
            </div>
            <div class="switch-box-item">百度网盘</div>
            <div class="anthology-list-box">
              <a href="https://pan.baidu.com/s/demo">合集</a>
            </div>
        "#;

        let item = parse_detail_page("https://www.jianpian.example/voddetail/888.html", html)
            .expect("detail should parse");
        assert_eq!(item.title, "荐片示例");
        assert_eq!(item.episodes.len(), 1);
        assert_eq!(item.episodes[0].source_name, "荐片线路A");
        assert_eq!(
            item.episodes[0].play_url,
            "https://www.jianpian.example/vodplay/888-1-1.html"
        );
    }

    #[test]
    fn extracts_jianpian_player_url() {
        let html = r#"player_data={"url":"https:\/\/video.example.com\/jianpian.mp4"}"#;
        assert_eq!(
            extract_player_url(html).as_deref(),
            Some("https://video.example.com/jianpian.mp4")
        );
    }
}
```

- [ ] **Step 2: Run the new JianPian tests to verify they fail**

Run:

```bash
cargo test -q parses_jianpian_listing_entries --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jianpian_detail_page_and_keeps_play_pages_only --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_jianpian_player_url --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `src-tauri/src/services/jianpian.rs` does not exist yet.

- [ ] **Step 3: Implement minimal JianPian parsing module**

```rust
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
}

pub(crate) fn parse_listing_page(
    page_url: &str,
    item_type: &str,
    html: &str,
) -> Vec<JianpianListingEntry> {
    let item_regex = Regex::new(
        r#"<a class="public-list-exp" href="([^"]+)" title="([^"]+)".*?<img[^>]+(?:data-src|src)="([^"]+)""#,
    )
    .unwrap();

    item_regex
        .captures_iter(html)
        .filter_map(|capture| {
            Some(JianpianListingEntry {
                title: capture.get(2)?.as_str().trim().to_string(),
                detail_url: absolutize_url(page_url, capture.get(1)?.as_str()),
                item_type: item_type.to_string(),
                poster: capture.get(3).map(|value| value.as_str().to_string()),
            })
        })
        .collect()
}

pub(crate) fn parse_detail_page(detail_url: &str, html: &str) -> Option<ScrapedCatalogItem> {
    let title_regex = Regex::new(r#"<h1>([^<]+)</h1>"#).unwrap();
    let section_regex = Regex::new(
        r#"(?s)<div class="switch-box-item">([^<]+)</div>\s*<div class="anthology-list-box">(.*?)</div>"#,
    )
    .unwrap();
    let anchor_regex = Regex::new(r#"<a href="([^"]+)">([^<]+)</a>"#).unwrap();

    let title = title_regex
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())?;
    let mut episodes = Vec::new();
    let mut seen = HashSet::new();

    for section in section_regex.captures_iter(html) {
        let source_name = section.get(1)?.as_str().trim().to_string();
        if is_external_source(&source_name) {
            continue;
        }
        for anchor in anchor_regex.captures_iter(section.get(2)?.as_str()) {
            let play_url = absolutize_url(detail_url, anchor.get(1)?.as_str());
            if !play_url.contains("/vodplay/") || !seen.insert(play_url.clone()) {
                continue;
            }
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label: anchor.get(2)?.as_str().trim().to_string(),
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
        summary: None,
        detail_json: Some(format!(r#"{{"source":"jianpian","url":"{}"}}"#, detail_url)),
        episodes,
    })
}

pub fn extract_player_url(body: &str) -> Option<String> {
    let regex = Regex::new(r#""url":"([^"]+)""#).unwrap();
    regex
        .captures(body)
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().replace(r#"\/"#, "/"))
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
```

- [ ] **Step 4: Export the JianPian module**

```rust
pub mod jianpian;

pub use jianpian::{extract_player_url as extract_jianpian_player_url, is_jianpian_site};
```

- [ ] **Step 5: Run the JianPian tests again**

Run:

```bash
cargo test -q parses_jianpian_listing_entries --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jianpian_detail_page_and_keeps_play_pages_only --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_jianpian_player_url --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/jianpian.rs src-tauri/src/services/mod.rs
git commit -m "feat: add jianpian parsing service"
```

### Task 3: Wire WenCai and JianPian into the supported TVBox catalog/detail dispatcher

**Files:**
- Modify: `src-tauri/src/services/xb6v.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/xb6v.rs`

- [ ] **Step 1: Add failing dispatcher tests**

```rust
#[test]
fn selects_wencai_source_from_tvbox_sites() {
    let site = crate::services::tvbox::TvboxSiteRecord {
        site_key: "文采".to_string(),
        site_name: "文采".to_string(),
        api: None,
        ext: None,
        searchable: true,
        quick_search: false,
        filterable: false,
        source_type: "custom".to_string(),
        raw_json: "{}".to_string(),
    };

    assert!(crate::services::is_wencai_site(&site));
}

#[test]
fn selects_jianpian_source_from_tvbox_sites() {
    let site = crate::services::tvbox::TvboxSiteRecord {
        site_key: "荐片".to_string(),
        site_name: "荐片".to_string(),
        api: None,
        ext: None,
        searchable: true,
        quick_search: false,
        filterable: false,
        source_type: "custom".to_string(),
        raw_json: "{}".to_string(),
    };

    assert!(crate::services::is_jianpian_site(&site));
}
```

- [ ] **Step 2: Run the dispatcher tests**

Run:

```bash
cargo test -q selects_wencai_source_from_tvbox_sites --manifest-path src-tauri/Cargo.toml
cargo test -q selects_jianpian_source_from_tvbox_sites --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL if the exports or detection wiring are incomplete.

- [ ] **Step 3: Wire source detection and dispatch in `src-tauri/src/services/xb6v.rs`**

```rust
use crate::services::jianpian::{is_jianpian_site, scrape_jianpian_catalog, scrape_jianpian_detail};
use crate::services::wencai::{is_wencai_site, scrape_wencai_catalog, scrape_wencai_detail};

if sites.iter().any(is_wencai_site) {
    items.extend(scrape_wencai_catalog().await?);
}
if sites.iter().any(is_jianpian_site) {
    items.extend(scrape_jianpian_catalog().await?);
}

match source.as_str() {
    "wencai" => {
        let mut item = scrape_wencai_detail(url).await?;
        Ok(item.take())
    }
    "jianpian" => {
        let mut item = scrape_jianpian_detail(url).await?;
        Ok(item.take())
    }
    _ => { /* existing branches */ }
}
```

- [ ] **Step 4: Run the dispatcher tests again**

Run:

```bash
cargo test -q selects_wencai_source_from_tvbox_sites --manifest-path src-tauri/Cargo.toml
cargo test -q selects_jianpian_source_from_tvbox_sites --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/xb6v.rs src-tauri/src/services/mod.rs
git commit -m "feat: wire wencai and jianpian source dispatch"
```

### Task 4: Add WenCai and JianPian resolver branches with candidate probing

**Files:**
- Modify: `src-tauri/src/services/resolver.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/resolver.rs`

- [ ] **Step 1: Add failing resolver tests**

```rust
#[test]
fn classifies_wencai_and_jianpian_play_pages_as_resolvable() {
    assert_eq!(
        classify_playback_target("https://www.wencai.example/play/123-1-1.html"),
        "resolvable"
    );
    assert_eq!(
        classify_playback_target("https://www.jianpian.example/vodplay/888-1-1.html"),
        "resolvable"
    );
}

#[test]
fn extracts_same_episode_candidates_from_wencai_page() {
    let body = r#"
        <script>var playp='正片';</script>
        <div class="line">文采A</div><a href="/play/1-1-1.html">正片</a>
        <div class="line">文采B</div><a href="/play/1-2-1.html">正片</a>
    "#;
    let candidates = extract_wencai_play_page_candidates(
        "https://www.wencai.example/play/1-1-1.html",
        body,
    );
    assert_eq!(candidates.len(), 2);
}

#[test]
fn extracts_same_episode_candidates_from_jianpian_page() {
    let body = r#"
        <script>var playp='正片';</script>
        <div class="from">荐片A</div><a href="/vodplay/2-1-1.html">正片</a>
        <div class="from">荐片B</div><a href="/vodplay/2-2-1.html">正片</a>
    "#;
    let candidates = extract_jianpian_play_page_candidates(
        "https://www.jianpian.example/vodplay/2-1-1.html",
        body,
    );
    assert_eq!(candidates.len(), 2);
}
```

- [ ] **Step 2: Run the new resolver tests**

Run:

```bash
cargo test -q classifies_wencai_and_jianpian_play_pages_as_resolvable --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_same_episode_candidates_from_wencai_page --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_same_episode_candidates_from_jianpian_page --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because resolver does not know these sources yet.

- [ ] **Step 3: Implement resolver branches and candidate extraction**

```rust
use crate::services::{extract_jianpian_player_url, extract_wencai_player_url};

if looks_like_wencai_play_page(input) {
    return resolve_wencai_play_page(input).await;
}
if looks_like_jianpian_play_page(input) {
    return resolve_jianpian_play_page(input).await;
}

fn looks_like_wencai_play_page(input: &str) -> bool {
    input.contains("/play/") && input.contains("wencai")
}

fn looks_like_jianpian_play_page(input: &str) -> bool {
    input.contains("/vodplay/") && input.contains("jianpian")
}

async fn resolve_wencai_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    resolve_multi_candidate_page(input, extract_wencai_play_page_candidates, extract_wencai_player_url).await
}

async fn resolve_jianpian_play_page(input: &str) -> Result<ResolvedPlayback, String> {
    resolve_multi_candidate_page(input, extract_jianpian_play_page_candidates, extract_jianpian_player_url).await
}
```

- [ ] **Step 4: Reuse the existing probe flow for resolved media candidates**

```rust
if probe_media_candidate(&client, &source_url).await.is_err() {
    continue;
}

candidates.push(PlaybackCandidate {
    url: source_url.clone(),
    label: play_page.label,
    kind: detect_kind(&source_url).to_string(),
    headers: None,
});
```

- [ ] **Step 5: Run the resolver tests again**

Run:

```bash
cargo test -q classifies_wencai_and_jianpian_play_pages_as_resolvable --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_same_episode_candidates_from_wencai_page --manifest-path src-tauri/Cargo.toml
cargo test -q extracts_same_episode_candidates_from_jianpian_page --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/resolver.rs src-tauri/src/services/mod.rs
git commit -m "feat: add wencai and jianpian playback resolver"
```

### Task 5: Add live-network coverage for WenCai and JianPian source scraping

**Files:**
- Modify: `src-tauri/src/services/wencai.rs`
- Modify: `src-tauri/src/services/jianpian.rs`
- Test: `src-tauri/src/services/wencai.rs`
- Test: `src-tauri/src/services/jianpian.rs`

- [ ] **Step 1: Add ignored real-network tests**

```rust
#[tokio::test]
#[ignore = "requires live network and DNS access"]
async fn scrapes_real_wencai_detail_page() {
    let url = std::env::var("WENCAI_DETAIL_URL").expect("WENCAI_DETAIL_URL should be set");
    let item = scrape_wencai_detail(&url)
        .await
        .expect("wencai detail should fetch")
        .expect("wencai detail should parse");
    assert!(!item.episodes.is_empty(), "expected wencai detail to produce episodes");
}

#[tokio::test]
#[ignore = "requires live network and DNS access"]
async fn scrapes_real_jianpian_detail_page() {
    let url = std::env::var("JIANPIAN_DETAIL_URL").expect("JIANPIAN_DETAIL_URL should be set");
    let item = scrape_jianpian_detail(&url)
        .await
        .expect("jianpian detail should fetch")
        .expect("jianpian detail should parse");
    assert!(!item.episodes.is_empty(), "expected jianpian detail to produce episodes");
}
```

- [ ] **Step 2: Discover one real WenCai URL and one real JianPian URL from local source data**

```bash
sqlite3 "$HOME/Library/Application Support/com.tvbox.app/tvbox.db" \
  "select json_extract(detail_json,'$.url') from catalog_items where json_extract(detail_json,'$.source')='wencai' limit 5;"

sqlite3 "$HOME/Library/Application Support/com.tvbox.app/tvbox.db" \
  "select json_extract(detail_json,'$.url') from catalog_items where json_extract(detail_json,'$.source')='jianpian' limit 5;"
```

- [ ] **Step 3: Run the ignored live-network tests with the discovered URLs**

Run:

```bash
WENCAI_DETAIL_URL="$(sqlite3 "$HOME/Library/Application Support/com.tvbox.app/tvbox.db" \
  "select json_extract(detail_json,'$.url') from catalog_items where json_extract(detail_json,'$.source')='wencai' limit 1;")" \
cargo test -q scrapes_real_wencai_detail_page --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture

JIANPIAN_DETAIL_URL="$(sqlite3 "$HOME/Library/Application Support/com.tvbox.app/tvbox.db" \
  "select json_extract(detail_json,'$.url') from catalog_items where json_extract(detail_json,'$.source')='jianpian' limit 1;")" \
cargo test -q scrapes_real_jianpian_detail_page --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
```

Expected: PASS with non-zero episode counts printed or asserted.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/wencai.rs src-tauri/src/services/jianpian.rs
git commit -m "test: add live network coverage for wencai and jianpian"
```

### Task 6: Verify catalog visibility contract and full regression suite

**Files:**
- Modify: `src-tauri/src/services/storage.rs`
- Test: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Add a storage-level visibility regression test**

```rust
#[test]
fn catalog_queries_keep_only_desktop_playable_wencai_and_jianpian_items() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("tvbox", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");

    seed_catalog_item_with_source(&storage, subscription.id, 201, "文采影片", "movie", "wencai");
    seed_catalog_item_with_source(&storage, subscription.id, 202, "荐片影片", "movie", "jianpian");
    seed_catalog_item_with_source(&storage, subscription.id, 203, "嵌页影片", "series", "zxzj");

    let items = storage
        .get_catalog_items(None, None)
        .expect("catalog items should query");

    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item.title == "文采影片"));
    assert!(items.iter().any(|item| item.title == "荐片影片"));
    assert!(items.iter().all(|item| item.title != "嵌页影片"));
}
```

- [ ] **Step 2: Run the storage visibility test**

Run:

```bash
cargo test -q catalog_queries_keep_only_desktop_playable_wencai_and_jianpian_items --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 3: Run the full backend test suite**

Run:

```bash
cargo test -q --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run the frontend build**

Run:

```bash
npm run build
```

Expected: PASS, with only the known `PlayerPage` chunk-size warning remaining.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "test: verify catalog visibility for wencai and jianpian"
```
