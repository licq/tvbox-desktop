# Native Rust TVBox Scraper Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace JavaScript spider mode (QuickJS) with native Rust scrapers for all 54 TVBox type-3 sources, enabling search and catalog browsing without depending on external JS spider files.

**Architecture:**

The architecture uses a `NativeScraper` provider that implements the `VideoProvider` trait. Each source gets its own Rust module with `is_<source>_site()`, `scrape_<source>_catalog()`, `scrape_<source>_detail()`, and `extract_<source>_player_url()` functions. The `ProviderRegistry` routes requests to the correct scraper based on source key. No JavaScript runtime is used.

**Tech Stack:** Rust, reqwest, scraper (CSS selectors), serde_json, tokio

---

## Phase 1: Scraper Infrastructure

### Task 1: Create native provider module scaffold

**Files:**
- Create: `src-tauri/src/services/provider/native.rs`
- Modify: `src-tauri/src/services/provider/mod.rs:34`
- Modify: `src-tauri/src/services/provider/registry.rs` (add `register_native_source`)

- [ ] **Step 1: Create `src-tauri/src/services/provider/native.rs` with base infrastructure**

```rust
// src-tauri/src/services/provider/native.rs

use async_trait::async_trait;
use reqwest::Client;
use crate::services::playback_types::PlaybackTarget;
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use super::{VideoProvider, ProviderError};

/// Base struct for all native scrapers. Each source embeds its own Client.
pub struct NativeScraper {
    pub site_key: String,
    pub site_name: String,
    pub base_url: String,
    pub client: Client,
}

impl NativeScraper {
    pub fn new(site_key: &str, site_name: &str, base_url: &str) -> Self {
        Self {
            site_key: site_key.to_string(),
            site_name: site_name.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch_text(&self, url: &str) -> Result<String, ProviderError> {
        self.client.get(url).send().await?
            .text().await.map_err(|e| ProviderError::Http(e))
    }

    pub async fn fetch_json(&self, url: &str) -> Result<serde_json::Value, ProviderError> {
        let text = self.fetch_text(url).await?;
        serde_json::from_str(&text).map_err(|e| ProviderError::Parse(e.to_string()))
    }
}

/// Extract source key from a playback URL by pattern matching.
pub fn extract_source_key_from_url(url: &str) -> Option<String> {
    if url.contains("xb6v.com") { Some("xb6v".to_string()) }
    else if url.contains("auete.com") { Some("auete".to_string()) }
    else if url.contains("zxzjhd.com") { Some("zxzj".to_string()) }
    else if url.contains("jianpian") { Some("jianpian".to_string()) }
    else if url.contains("wencai") { Some("wencai".to_string()) }
    else if url.contains("libvio") { Some("libvio".to_string()) }
    else { None }
}
```

- [ ] **Step 2: Update `src-tauri/src/services/provider/mod.rs`**

Add to line 34 (after spider_provider line):
```rust
pub mod native;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build --lib`
Expected: SUCCESS (warning about unused code only)

- [ ] **Step 4: Commit**

```bash
git add src/services/provider/native.rs src/services/provider/mod.rs
git commit -m "feat(provider): add native scraper base infrastructure"
```

---

### Task 2: Integrate native scrapers into ProviderRegistry

**Files:**
- Modify: `src-tauri/src/services/provider/registry.rs:1-10` (add `use super::native::*;`)
- Modify: `src-tauri/src/services/provider/registry.rs` (add `register_all_native_sources()`)

- [ ] **Step 1: Add `use super::native::*;` to imports in registry.rs**

```rust
use super::{VideoProvider, CmsProvider, SpiderProvider, NativeScraper};
```

- [ ] **Step 2: Add `register_all_native_sources()` method to ProviderRegistry**

Add after `pub fn register_from_config_with_spider`:

```rust
    /// Register all known native Rust scrapers.
    pub fn register_all_native_sources(&mut self) {
        // Add each native source here as they're implemented
        // Example: self.register_site(Box::new(auete::create_provider()));
    }
```

- [ ] **Step 3: Verify compilation**

Run: `cargo build --lib`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git add src/services/provider/registry.rs
git commit -m "feat(provider): add native source registration to registry"
```

---

## Phase 2: Per-Source Native Scrapers

Each task below implements ONE TVBox source as a native Rust scraper. Start with the highest-priority searchable sources.

### Task 3: Implement xb6v native scraper

**Files:**
- Create: `src-tauri/src/services/provider/xb6v_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs` (add `pub mod xb6v_scraper;`)
- Modify: `src-tauri/src/services/provider/registry.rs` (register in `register_all_native_sources()`)

The `xb6v` source (key: `新6V`, ext: `https://www.xb6v.com/`) is already partially handled by `resolver.rs` which can resolve `xb6v.com/e/DownSys/play/` URLs. Create a full native scraper for its catalog and search.

- [ ] **Step 1: Write the failing test** (in `src-tauri/src/services/provider/xb6v_scraper.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn xb6v_search_returns_results() {
        let scraper = NativeScraper::new("xb6v", "新6V", "https://www.xb6v.com");
        let results = scraper.search("功夫").await.unwrap();
        assert!(!results.is_empty(), "xb6v search should return results for '功夫'");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib xb6v_search_returns_results`
Expected: FAIL - method not found

- [ ] **Step 3: Implement the xb6v scraper with `home()`, `search()`, `detail()`, `play()` methods**

```rust
pub struct Xb6vScraper {
    base: NativeScraper,
}

impl Xb6vScraper {
    pub fn new() -> Self {
        Self { base: NativeScraper::new("xb6v", "新6V", "https://www.xb6v.com") }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/e/search/index.php?keyword={}", self.base.base_url, keyword);
        let body = self.base.fetch_text(&url).await?;
        // Parse HTML using scraper crate - find .movie-item elements
        // Return Vec<ScrapedCatalogItem>
        Ok(vec![])
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        // Parse homepage
        Ok(vec![])
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // ids format: "classid=8&id=11308"
        let url = format!("{}/e/DownSys/play/?{}", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        // Parse detail page, extract episodes
        Ok(None)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        // xb6v play pages are already handled by resolver.rs
        // This just returns the URL as-is
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "xb6v".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }])
    }
}

#[async_trait]
impl VideoProvider for Xb6vScraper {
    fn source_key(&self) -> &str { &self.base.site_key }
    fn source_name(&self) -> &str { &self.base.site_name }
    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { self.home().await }
    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> { Ok(vec![]) }
    async fn category(&self, _type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { Ok(vec![]) }
    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> { self.search(keyword).await }
    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> { self.detail(ids).await }
    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> { self.play(flag, play_url).await }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib xb6v_search_returns_results`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/services/provider/xb6v_scraper.rs src/services/provider/native.rs src/services/provider/registry.rs
git commit -m "feat(scraper): add native xb6v scraper"
```

---

### Task 4: Implement auete native scraper

**Files:**
- Create: `src-tauri/src/services/provider/auete_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs` (add `pub mod auete_scraper;`)
- Modify: `src-tauri/src/services/provider/registry.rs` (register in `register_all_native_sources()`)

The `auete` source (key: `奥特`, ext: `https://auete.com/`) is ranked highest priority (rank 0) in playback ranking.

- [ ] **Step 1: Write the failing test**

```rust
#[tokio::test]
async fn auete_search_returns_results() {
    let scraper = NativeScraper::new("auete", "奥特┃多线", "https://auete.com");
    let results = scraper.search("功夫").await.unwrap();
    assert!(!results.is_empty(), "auete search should return results");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib auete_search_returns_results`
Expected: FAIL

- [ ] **Step 3: Implement auete scraper**

Study `auete.com` to find its search and catalog HTML structure. Use `scraper` crate for CSS selectors.

```rust
pub struct AueteScraper {
    base: NativeScraper,
}

impl AueteScraper {
    pub fn new() -> Self {
        Self { base: NativeScraper::new("auete", "奥特┃多线", "https://auete.com") }
    }

    pub async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/search?wd={}", self.base.base_url, keyword);
        let body = self.base.fetch_text(&url).await?;
        // Parse HTML: find video list items
        // Extract: title, poster (thumb), detail_url
        Ok(vec![])
    }

    pub async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/", self.base.base_url);
        let body = self.base.fetch_text(&url).await?;
        Ok(vec![])
    }

    pub async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = format!("{}/video/{}", self.base.base_url, ids);
        let body = self.base.fetch_text(&url).await?;
        // Parse episodes from player page
        Ok(None)
    }

    pub async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: "auete".to_string(),
            target_url: play_url.to_string(),
            target_kind: PlaybackTargetKind::Direct,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        }])
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib auete_search_returns_results`
Expected: PASS (or skip if network unavailable)

- [ ] **Step 5: Commit**

```bash
git add src/services/provider/auete_scraper.rs src/services/provider/native.rs src/services/provider/registry.rs
git commit -m "feat(scraper): add native auete scraper"
```

---

### Task 5: Implement zxzj native scraper

**Files:**
- Create: `src-tauri/src/services/provider/zxzj_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs` (add `pub mod zxzj_scraper;`)
- Modify: `src-tauri/src/services/provider/registry.rs` (register)

`zxzj` source (key: `zxzj`, ext: `https://www.zxzjhd.com/`), searchable=1, rank=4.

- [ ] **Step 1-5: Same pattern as Task 3 and 4**

Study `zxzjhd.com` HTML structure, implement scraper, test, commit.

---

### Task 6: Implement libvio native scraper

**Files:**
- Create: `src-tauri/src/services/provider/libvio_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs`
- Modify: `src-tauri/src/services/provider/registry.rs`

`libvio` source (key: `Lib`, ext contains `Cloud-drive`), searchable=1, rank=1 (second highest).

- [ ] **Step 1-5: Same pattern**

Study the actual libvio site URL (from ext.Cloud-drive path resolution), implement scraper.

---

### Task 7: Implement jianpian native scraper

**Files:**
- Create: `src-tauri/src/services/provider/jianpian_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs`
- Modify: `src-tauri/src/services/provider/registry.rs`

`jianpian` source (key: `贱贱`, api: `csp_JPJGuard`), searchable=1, rank=0 (highest).

- [ ] **Step 1-5: Same pattern**

---

### Task 8: Implement wencai/jpys native scraper

**Files:**
- Create: `src-tauri/src/services/provider/wencai_scraper.rs`
- Modify: `src-tauri/src/services/provider/native.rs`
- Modify: `src-tauri/src/services/provider/registry.rs`

`wencai` source (key: `文采`, api: `csp_JpysGuard`), searchable=1, rank=0.

- [ ] **Step 1-5: Same pattern**

---

### Task 9: Implement remaining searchable sources (batch)

After the top 6-8 sources are done, batch implement the remaining `searchable=1` sources:

**Sources with searchable=1** (from fixture):
- YGP, 抠搜, UC, 原创, 苹果, 糯米, 白白, 文采 (done above), Lib (done above), zxzj (done above), 厂长, 溢彩, 比特, 低端, 萌米, 兄弟, 热播, 欢视, 奥特 (done above), 荐片 (done above), 新6V (done above), Dm84, Ysj, Anime1, YpanSo, xzso, 米搜, 夸搜, Aliso, 易搜, Bili, Biliych, fan, cc

Each of these needs a native scraper module following the same pattern as Tasks 3-8.

- [ ] **Step 1: Create scraper for each remaining searchable source** (use subagent per source)

Each scraper follows the same pattern: `new()`, `search()`, `home()`, `detail()`, `play()` + `VideoProvider` trait impl.

- [ ] **Step 2: Register all in `register_all_native_sources()`**

- [ ] **Step 3: Test with `cargo test --lib <source>_search_returns_results`**

- [ ] **Step 4: Commit each source separately**

---

## Phase 3: Disable JS Spider Mode

### Task 10: Remove SpiderProvider dependency for TVBox sources

**Files:**
- Modify: `src-tauri/src/services/provider/registry.rs` (remove SpiderProvider fallback for csp_* sources)
- Modify: `src-tauri/src/main.rs` (remove `fetch_spider_script` calls)
- Modify: `src-tauri/src/commands/subscription.rs` (remove spider script fetch)

- [ ] **Step 1: Remove `fetch_spider_script` from registry**

Remove `fetch_spider_script`, `resolve_spider_urls`, `extract_spider_path`, and `looks_like_javascript` from registry.rs.

- [ ] **Step 2: Remove `SpiderProvider` from TVBox source registration**

In `register_site_with_spider`, change class-based (csp_*) handling to use native scraper instead of `SpiderProvider::new()`.

- [ ] **Step 3: Verify compilation**

Run: `cargo build --lib`
Expected: SUCCESS

- [ ] **Step 4: Commit**

```bash
git commit -m "refactor(provider): remove JS spider fallback, use native scrapers only"
```

---

## Phase 4: Integration Test

### Task 11: Write integration test for native scrapers

**Files:**
- Modify: `src-tauri/src/services/provider/registry.rs` (add test)

- [ ] **Step 1: Write integration test**

```rust
#[tokio::test]
async fn native_scrapers_search_returns_results() {
    let mut registry = ProviderRegistry::new();
    registry.register_all_native_sources();

    let keyword = "功夫";
    let results = registry.search_all(keyword).await;

    let total_items: usize = results.iter().map(|r| r.items.len()).sum();
    let providers_searched = results.len();

    println!("Search '{}': {} providers returned results, {} total items",
        keyword, providers_searched, total_items);

    // Assert at least some results from the native scrapers
    assert!(total_items > 0, "At least one native scraper should return results for '{}'", keyword);
}
```

- [ ] **Step 2: Run test**

Run: `cargo test --lib native_scrapers_search_returns_results -- --nocapture`
Expected: PASS (with actual search results from working scrapers)

- [ ] **Step 3: Commit**

---

## Phase 5: Source Prioritization Reference

The 54 TVBox sources from the fixture break down as follows:

### Priority 1 - Implement First (searchable=1, high rank)
| Key | Name | Site URL | Notes |
|-----|------|---------|-------|
| 文采 | 💮文采┃秒播 | (csp_JpysGuard) | Rank 0 |
| 奥特 | 🏝奥特┃多线 | auete.com | Rank 0 |
| 贱贱 | 🐭荐片┃P2P | (csp_JPJGuard) | Rank 0 |
| 新6V | 🧲新6V┃磁力 | xb6v.com | Rank 2 |
| Lib | 🌟立播┃秒播 | (csp_LibvioGuard) | Rank 1 |
| zxzj | 🍊在线┃秒播 | zxzjhd.com | Rank 4 |
| 厂长 | 📔厂长┃不卡 | (csp_NewCzGuard) | |
| 溢彩 | 💡流光┃秒播 | (csp_AppSxGuard) | |
| 比特 | 🍄比特┃手机 | (csp_BttwooGuard) | |
| 低端 | ⏮️低端┃外剧 | (csp_DdrkGuard) | |
| 萌米 | 👀萌米┃多线 | (csp_AppTTGuard) | |

### Priority 2 - Implement Second (searchable=1, standard priority)
| Key | Name |
|-----|------|
| 兄弟 | 🍊水星┃多线 |
| 热播 | 📺热播┃多线 |
| 欢视 | 👓欢视┃多线 |
| 玩偶 | 👽玩偶哥哥┃4K弹幕 |
| YGP | 🚀叨观荐影┃预告片 |
| 抠搜 | 🍄抠抠┃搜搜 |
| UC | 🌈优汐┃搜搜 |
| 原创 | ☀原创┃不卡 |
| 苹果 | 🍎苹果┃不卡 |
| 糯米 | 🍓糯米┃秒播 |
| 白白 | 🐟白白┃秒播 |
| Dm84 | 🚌巴士┃动漫 |
| Ysj | 🎀异界┃动漫 |
| Anime1 | 🐾日本┃动漫 |
| YpanSo | 🐟盘她┃三盘 |
| xzso | 👻盘它┃三盘 |
| 米搜 | 🦋米搜┃夸父 |
| 夸搜 | 😻夸搜┃夸父 |
| Aliso | 🙀盘搜┃阿狸 |
| 易搜 | 😹易搜┃阿狸 |
| Bili | 🅱哔哔合集┃弹幕 |
| Biliych | 🅱哔哔演唱会┃弹幕 |
| fan | 导航www.饭太硬.com |
| cc | 请勿相信视频中广告 |

### Priority 3 - Not Searchable (searchable=0)
All remaining sources (豆, alllive, YGP, Aid, MTV, 926, 88, 看球, Jrsjs, 酷奇, 虎牙直播js, 斗鱼js, 有声小说js, dr_兔小贝, 少儿教育, 小学课堂, 初中课堂, 高中教育, push_agent) have searchable=0 and can be implemented later or not at all (they won't be searched anyway).

---

## File Structure After Implementation

```
src/services/provider/
├── mod.rs           # + pub mod native; + pub mod xb6v_scraper; etc.
├── traits.rs        # VideoProvider trait (unchanged)
├── cms_provider.rs  # CMS provider (unchanged)
├── spider_provider.rs  # Kept but no longer used for TVBox csp_* sources
├── registry.rs      # + register_all_native_sources(), remove JS spider
├── native.rs        # NativeScraper base struct + helper fns
├── xb6v_scraper.rs  # NEW: xb6v native scraper
├── auete_scraper.rs  # NEW: auete native scraper
├── zxzj_scraper.rs  # NEW: zxzj native scraper
├── libvio_scraper.rs # NEW: libvio native scraper
├── jianpian_scraper.rs # NEW: jianpian native scraper
├── wencai_scraper.rs # NEW: wencai/jpys native scraper
├── yuanchuang_scraper.rs # NEW: 原创 source
├── changzhang_scraper.rs # NEW: 厂长 source
└── ... (one file per source)
```

## Verification Commands

```bash
# Build
cargo build --lib

# Run all tests
cargo test --lib

# Run native scraper integration test
cargo test --lib native_scrapers_search_returns_results -- --nocapture

# Run single scraper test
cargo test --lib xb6v_search_returns_results -- --nocapture
```

## Notes

- **Scraper approach**: Use `reqwest` for HTTP + `scraper` crate for CSS selectors on HTML responses
- **No JS runtime**: Completely remove QuickJS/rquickjs dependency for TVBox sources
- **Error handling**: Each scraper method should return `Result<T, ProviderError>` with clear error messages
- **Fallback**: If a scraper method fails (network error, parsing error), return `Err(ProviderError::...)` so the registry logs it and continues with other sources
- **Testing**: Each scraper has its own test that verifies network access and parsing. Tests should be skipped if network unavailable.
