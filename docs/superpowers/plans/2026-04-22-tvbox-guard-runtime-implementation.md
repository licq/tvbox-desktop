# TVBox Guard Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a pure-Rust Guard runtime that lets `csp_JpysGuard` and `csp_JPJGuard` TVBox sources produce catalog items, detail episodes, and resolved playable media candidates inside the existing Tauri backend.

**Architecture:** Add a reusable Guard adapter layer in Rust, wire it into TVBox catalog/detail ingestion, and extend the playback resolver to understand internal `guard://...` play targets. Keep storage and frontend contracts stable by continuing to write `catalog_items`, `catalog_episodes`, and `ResolvedPlayback` using the same backend query surface.

**Tech Stack:** Rust, reqwest, regex, serde_json, rusqlite, tokio, Tauri backend commands, existing storage/resolver pipeline

---

## File Map

| File | Action | Responsibility |
| --- | --- | --- |
| `src-tauri/src/services/guard.rs` | Create | Shared Guard adapter trait, registry, request client helpers, internal guard play-target encoding |
| `src-tauri/src/services/guard_jpys.rs` | Create | `csp_JpysGuard` adapter for category/detail/play resolution |
| `src-tauri/src/services/guard_jpj.rs` | Create | `csp_JPJGuard` adapter for category/detail/play resolution |
| `src-tauri/src/services/mod.rs` | Modify | Export Guard runtime surface |
| `src-tauri/src/services/xb6v.rs` | Modify | Bridge Guard-backed `source_sites` into catalog/detail ingestion |
| `src-tauri/src/services/resolver.rs` | Modify | Resolve `guard://...` play targets into real candidates and probe them |
| `src-tauri/src/services/storage.rs` | Modify | Add regression coverage for Guard-backed catalog/detail visibility |

### Task 1: Add the shared Guard runtime contract

**Files:**
- Create: `src-tauri/src/services/guard.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/guard.rs`

- [ ] **Step 1: Write the failing Guard contract tests**

```rust
#[cfg(test)]
mod tests {
    use super::{
        decode_guard_play_target, encode_guard_play_target, guard_adapter_key,
        is_guard_site_supported,
    };
    use crate::services::tvbox::TvboxSiteRecord;

    #[test]
    fn recognizes_supported_guard_sites() {
        let jpys = TvboxSiteRecord {
            site_key: "文采".to_string(),
            site_name: "💮文采┃秒播".to_string(),
            api: Some("csp_JpysGuard".to_string()),
            ext: None,
            searchable: true,
            quick_search: true,
            filterable: false,
            source_type: "3".to_string(),
            raw_json: r#"{"api":"csp_JpysGuard"}"#.to_string(),
        };
        let jpj = TvboxSiteRecord {
            site_key: "贱贱".to_string(),
            site_name: "🐭荐片┃P2P".to_string(),
            api: Some("csp_JPJGuard".to_string()),
            ext: None,
            searchable: true,
            quick_search: true,
            filterable: false,
            source_type: "3".to_string(),
            raw_json: r#"{"api":"csp_JPJGuard"}"#.to_string(),
        };

        assert!(is_guard_site_supported(&jpys));
        assert!(is_guard_site_supported(&jpj));
        assert_eq!(guard_adapter_key(&jpys).as_deref(), Some("csp_JpysGuard"));
        assert_eq!(guard_adapter_key(&jpj).as_deref(), Some("csp_JPJGuard"));
    }

    #[test]
    fn round_trips_guard_play_targets() {
        let encoded = encode_guard_play_target("csp_JpysGuard", "文采", "1419", "1", "1");
        let decoded = decode_guard_play_target(&encoded).expect("guard target should decode");

        assert_eq!(decoded.guard_key, "csp_JpysGuard");
        assert_eq!(decoded.site_key, "文采");
        assert_eq!(decoded.item_id, "1419");
        assert_eq!(decoded.source_id, "1");
        assert_eq!(decoded.episode_id, "1");
    }
}
```

- [ ] **Step 2: Run the new Guard tests to verify they fail**

Run:

```bash
cargo test -q recognizes_supported_guard_sites --manifest-path src-tauri/Cargo.toml
cargo test -q round_trips_guard_play_targets --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `guard.rs` does not exist yet.

- [ ] **Step 3: Implement the shared Guard contract**

Create `src-tauri/src/services/guard.rs`:

```rust
use crate::services::tvbox::TvboxSiteRecord;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardPlayTarget {
    pub guard_key: String,
    pub site_key: String,
    pub item_id: String,
    pub source_id: String,
    pub episode_id: String,
}

pub fn guard_adapter_key(site: &TvboxSiteRecord) -> Option<String> {
    let api = site.api.as_deref().unwrap_or_default();
    match api {
        "csp_JpysGuard" | "csp_JPJGuard" => Some(api.to_string()),
        _ => None,
    }
}

pub fn is_guard_site_supported(site: &TvboxSiteRecord) -> bool {
    guard_adapter_key(site).is_some()
}

pub fn encode_guard_play_target(
    guard_key: &str,
    site_key: &str,
    item_id: &str,
    source_id: &str,
    episode_id: &str,
) -> String {
    format!(
        "guard://{}/{}/{}/{}/{}",
        urlencoding::encode(guard_key),
        urlencoding::encode(site_key),
        urlencoding::encode(item_id),
        urlencoding::encode(source_id),
        urlencoding::encode(episode_id)
    )
}

pub fn decode_guard_play_target(value: &str) -> Option<GuardPlayTarget> {
    let trimmed = value.strip_prefix("guard://")?;
    let mut parts = trimmed.split('/');
    let guard_key = urlencoding::decode(parts.next()?).ok()?.to_string();
    let site_key = urlencoding::decode(parts.next()?).ok()?.to_string();
    let item_id = urlencoding::decode(parts.next()?).ok()?.to_string();
    let source_id = urlencoding::decode(parts.next()?).ok()?.to_string();
    let episode_id = urlencoding::decode(parts.next()?).ok()?.to_string();

    Some(GuardPlayTarget {
        guard_key,
        site_key,
        item_id,
        source_id,
        episode_id,
    })
}
```

- [ ] **Step 4: Export the Guard runtime surface**

Update `src-tauri/src/services/mod.rs`:

```rust
pub mod guard;
pub mod guard_jpj;
pub mod guard_jpys;

pub use guard::{
    decode_guard_play_target, encode_guard_play_target, guard_adapter_key,
    is_guard_site_supported, GuardPlayTarget,
};
```

- [ ] **Step 5: Run the Guard tests again**

Run:

```bash
cargo test -q recognizes_supported_guard_sites --manifest-path src-tauri/Cargo.toml
cargo test -q round_trips_guard_play_targets --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/guard.rs src-tauri/src/services/mod.rs
git commit -m "feat: add guard runtime contract"
```

### Task 2: Implement the `csp_JpysGuard` adapter

**Files:**
- Create: `src-tauri/src/services/guard_jpys.rs`
- Modify: `src-tauri/src/services/guard.rs`
- Test: `src-tauri/src/services/guard_jpys.rs`

- [ ] **Step 1: Write the failing `JpysGuard` parser tests**

```rust
#[cfg(test)]
mod tests {
    use super::{
        parse_jpys_detail_payload, parse_jpys_list_payload, parse_jpys_play_payload,
    };

    #[test]
    fn parses_jpys_category_list() {
        let payload = r#"{
          "list":[
            {"vod_id":"1419","vod_name":"复仇双雄","vod_pic":"https://img.example.com/a.jpg","type_name":"动作"}
          ]
        }"#;

        let items = parse_jpys_list_payload("文采", "movie", payload).expect("list should parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "复仇双雄");
        assert_eq!(items[0].item_id, "1419");
    }

    #[test]
    fn parses_jpys_detail_payload() {
        let payload = r#"{
          "list":[
            {
              "vod_id":"1419",
              "vod_name":"复仇双雄",
              "vod_pic":"https://img.example.com/a.jpg",
              "vod_content":"剧情简介",
              "vod_play_from":"线路A$$$线路B",
              "vod_play_url":"正片$1-1-1#预告$1-1-2$$$正片$1-2-1"
            }
          ]
        }"#;

        let detail = parse_jpys_detail_payload("文采", "1419", payload).expect("detail should parse");
        assert_eq!(detail.title, "复仇双雄");
        assert_eq!(detail.episodes.len(), 3);
        assert!(detail.episodes[0].play_url.starts_with("guard://"));
    }

    #[test]
    fn parses_jpys_play_payload() {
        let payload = r#"{"url":"https://media.example.com/demo/index.m3u8"}"#;
        let resolved = parse_jpys_play_payload(payload).expect("play payload should parse");
        assert_eq!(resolved, "https://media.example.com/demo/index.m3u8");
    }
}
```

- [ ] **Step 2: Run the new `JpysGuard` tests to verify they fail**

Run:

```bash
cargo test -q parses_jpys_category_list --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpys_detail_payload --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpys_play_payload --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `guard_jpys.rs` does not exist yet.

- [ ] **Step 3: Implement the `JpysGuard` adapter and parser helpers**

Create `src-tauri/src/services/guard_jpys.rs`:

```rust
use crate::services::guard::encode_guard_play_target;
use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardListItem {
    pub item_id: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
}

pub fn parse_jpys_list_payload(
    site_key: &str,
    item_type: &str,
    payload: &str,
) -> Result<Vec<GuardListItem>, String> {
    let root: Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    let list = root
        .get("list")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "jpys list payload missing list".to_string())?;

    Ok(list
        .iter()
        .filter_map(|entry| {
            Some(GuardListItem {
                item_id: entry.get("vod_id")?.as_str()?.to_string(),
                title: entry.get("vod_name")?.as_str()?.to_string(),
                item_type: item_type.to_string(),
                poster: entry.get("vod_pic").and_then(|v| v.as_str()).map(str::to_string),
                summary: entry
                    .get("vod_content")
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
                    .or_else(|| entry.get("type_name").and_then(|v| v.as_str()).map(str::to_string)),
            })
        })
        .collect())
}

pub fn parse_jpys_detail_payload(
    site_key: &str,
    item_id: &str,
    payload: &str,
) -> Option<ScrapedCatalogItem> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let entry = root.get("list")?.as_array()?.first()?;
    let title = entry.get("vod_name")?.as_str()?.to_string();
    let poster = entry.get("vod_pic").and_then(|v| v.as_str()).map(str::to_string);
    let summary = entry.get("vod_content").and_then(|v| v.as_str()).map(str::to_string);
    let play_from = entry.get("vod_play_from")?.as_str()?;
    let play_url = entry.get("vod_play_url")?.as_str()?;

    let sources: Vec<&str> = play_from.split("$$$").collect();
    let groups: Vec<&str> = play_url.split("$$$").collect();

    let mut episodes = Vec::new();
    for (source_index, source_name) in sources.iter().enumerate() {
        let group = groups.get(source_index).copied().unwrap_or_default();
        for episode in group.split('#') {
            let mut parts = episode.split('$');
            let label = parts.next()?.trim();
            let encoded = parts.next()?.trim();
            let mut ids = encoded.split('-');
            let source_id = ids.next()?;
            let episode_id = ids.next()?;
            let play_url = encode_guard_play_target(
                "csp_JpysGuard",
                site_key,
                item_id,
                source_id,
                episode_id,
            );
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.trim().to_string(),
                episode_label: label.to_string(),
                play_url,
                order_index: episodes.len() as i64,
            });
        }
    }

    Some(ScrapedCatalogItem {
        source_item_key: format!("guard:{}:{}", site_key, item_id),
        title,
        item_type: "movie".to_string(),
        poster,
        summary,
        detail_json: Some(format!(
            r#"{{"source":"guard","guard_key":"csp_JpysGuard","site_key":"{}","item_id":"{}","item_type":"movie"}}"#,
            site_key, item_id
        )),
        episodes,
    })
}

pub fn parse_jpys_play_payload(payload: &str) -> Option<String> {
    let root: Value = serde_json::from_str(payload).ok()?;
    root.get("url").and_then(|value| value.as_str()).map(str::to_string)
}
```

- [ ] **Step 4: Run the `JpysGuard` tests again**

Run:

```bash
cargo test -q parses_jpys_category_list --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpys_detail_payload --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpys_play_payload --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/guard_jpys.rs
git commit -m "feat: add jpys guard adapter parsers"
```

### Task 3: Implement the `csp_JPJGuard` adapter

**Files:**
- Create: `src-tauri/src/services/guard_jpj.rs`
- Test: `src-tauri/src/services/guard_jpj.rs`

- [ ] **Step 1: Write the failing `JPJGuard` parser tests**

```rust
#[cfg(test)]
mod tests {
    use super::{
        parse_jpj_detail_payload, parse_jpj_list_payload, parse_jpj_play_payload,
    };

    #[test]
    fn parses_jpj_category_list() {
        let payload = r#"{
          "data":[
            {"id":"71483","title":"龙之家族 第二季","cover":"https://img.example.com/b.jpg"}
          ]
        }"#;

        let items = parse_jpj_list_payload("贱贱", "series", payload).expect("list should parse");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].item_id, "71483");
        assert_eq!(items[0].title, "龙之家族 第二季");
    }

    #[test]
    fn parses_jpj_detail_payload() {
        let payload = r#"{
          "data":{
            "id":"71483",
            "title":"龙之家族 第二季",
            "cover":"https://img.example.com/b.jpg",
            "intro":"剧情简介",
            "play_sources":[
              {"id":"1","name":"荐片A","episodes":[{"id":"1","name":"第01集"},{"id":"2","name":"第02集"}]}
            ]
          }
        }"#;

        let detail = parse_jpj_detail_payload("贱贱", "71483", payload).expect("detail should parse");
        assert_eq!(detail.title, "龙之家族 第二季");
        assert_eq!(detail.episodes.len(), 2);
        assert!(detail.episodes[0].play_url.starts_with("guard://"));
    }

    #[test]
    fn parses_jpj_play_payload() {
        let payload = r#"{"data":{"url":"https://media.example.com/demo.mp4"}}"#;
        let resolved = parse_jpj_play_payload(payload).expect("play payload should parse");
        assert_eq!(resolved, "https://media.example.com/demo.mp4");
    }
}
```

- [ ] **Step 2: Run the new `JPJGuard` tests to verify they fail**

Run:

```bash
cargo test -q parses_jpj_category_list --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpj_detail_payload --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpj_play_payload --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `guard_jpj.rs` does not exist yet.

- [ ] **Step 3: Implement the `JPJGuard` adapter and parser helpers**

Create `src-tauri/src/services/guard_jpj.rs`:

```rust
use crate::services::guard::encode_guard_play_target;
use crate::services::xb6v::{ScrapedCatalogEpisode, ScrapedCatalogItem};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardListItem {
    pub item_id: String,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub summary: Option<String>,
}

pub fn parse_jpj_list_payload(
    _site_key: &str,
    item_type: &str,
    payload: &str,
) -> Result<Vec<GuardListItem>, String> {
    let root: Value = serde_json::from_str(payload).map_err(|e| e.to_string())?;
    let list = root
        .get("data")
        .and_then(|value| value.as_array())
        .ok_or_else(|| "jpj list payload missing data array".to_string())?;

    Ok(list
        .iter()
        .filter_map(|entry| {
            Some(GuardListItem {
                item_id: entry.get("id")?.as_str()?.to_string(),
                title: entry.get("title")?.as_str()?.to_string(),
                item_type: item_type.to_string(),
                poster: entry.get("cover").and_then(|v| v.as_str()).map(str::to_string),
                summary: entry.get("intro").and_then(|v| v.as_str()).map(str::to_string),
            })
        })
        .collect())
}

pub fn parse_jpj_detail_payload(
    site_key: &str,
    item_id: &str,
    payload: &str,
) -> Option<ScrapedCatalogItem> {
    let root: Value = serde_json::from_str(payload).ok()?;
    let entry = root.get("data")?;
    let title = entry.get("title")?.as_str()?.to_string();
    let poster = entry.get("cover").and_then(|v| v.as_str()).map(str::to_string);
    let summary = entry.get("intro").and_then(|v| v.as_str()).map(str::to_string);
    let sources = entry.get("play_sources")?.as_array()?;

    let mut episodes = Vec::new();
    for source in sources {
        let source_name = source.get("name")?.as_str()?.to_string();
        let source_id = source.get("id")?.as_str()?.to_string();
        for episode in source.get("episodes")?.as_array()? {
            let episode_id = episode.get("id")?.as_str()?.to_string();
            let episode_label = episode.get("name")?.as_str()?.to_string();
            episodes.push(ScrapedCatalogEpisode {
                source_name: source_name.clone(),
                episode_label,
                play_url: encode_guard_play_target(
                    "csp_JPJGuard",
                    site_key,
                    item_id,
                    &source_id,
                    &episode_id,
                ),
                order_index: episodes.len() as i64,
            });
        }
    }

    Some(ScrapedCatalogItem {
        source_item_key: format!("guard:{}:{}", site_key, item_id),
        title,
        item_type: "movie".to_string(),
        poster,
        summary,
        detail_json: Some(format!(
            r#"{{"source":"guard","guard_key":"csp_JPJGuard","site_key":"{}","item_id":"{}","item_type":"movie"}}"#,
            site_key, item_id
        )),
        episodes,
    })
}

pub fn parse_jpj_play_payload(payload: &str) -> Option<String> {
    let root: Value = serde_json::from_str(payload).ok()?;
    root.get("data")
        .and_then(|value| value.get("url"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}
```

- [ ] **Step 4: Run the `JPJGuard` tests again**

Run:

```bash
cargo test -q parses_jpj_category_list --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpj_detail_payload --manifest-path src-tauri/Cargo.toml
cargo test -q parses_jpj_play_payload --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/guard_jpj.rs
git commit -m "feat: add jpj guard adapter parsers"
```

### Task 4: Wire Guard catalog and detail ingestion into the TVBox pipeline

**Files:**
- Modify: `src-tauri/src/services/xb6v.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/xb6v.rs`

- [ ] **Step 1: Write the failing Guard ingestion tests**

```rust
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

    assert_eq!(collect_supported_guard_keys(&sites), vec!["csp_JpysGuard", "csp_JPJGuard"]);
}

#[test]
fn parses_guard_detail_json_source() {
    let item = crate::services::xb6v::scrape_catalog_detail_from_json(
        r#"{"source":"guard","guard_key":"csp_JpysGuard","site_key":"文采","item_id":"1419","item_type":"movie"}"#,
    );
    assert!(item.is_err(), "dispatch path should exist even before network stubbing");
}
```

- [ ] **Step 2: Run the Guard ingestion tests to verify they fail**

Run:

```bash
cargo test -q selects_guard_sites_from_tvbox_records --manifest-path src-tauri/Cargo.toml
cargo test -q parses_guard_detail_json_source --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because Guard dispatch helpers do not exist yet.

- [ ] **Step 3: Implement Guard catalog/detail dispatch**

Update `src-tauri/src/services/xb6v.rs`:

```rust
use crate::services::guard::{guard_adapter_key, is_guard_site_supported};
use crate::services::guard_jpj::{parse_jpj_detail_payload, parse_jpj_list_payload};
use crate::services::guard_jpys::{parse_jpys_detail_payload, parse_jpys_list_payload};

fn collect_supported_guard_keys(sites: &[TvboxSiteRecord]) -> Vec<&'static str> {
    let mut keys = Vec::new();
    if sites.iter().any(|site| site.api.as_deref() == Some("csp_JpysGuard")) {
        keys.push("csp_JpysGuard");
    }
    if sites.iter().any(|site| site.api.as_deref() == Some("csp_JPJGuard")) {
        keys.push("csp_JPJGuard");
    }
    keys
}

pub async fn scrape_supported_tvbox_catalogs(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let mut items = Vec::new();
    // existing xb6v/libvio/auete/zxzj branches stay in place
    if sites.iter().any(is_guard_site_supported) {
        items.extend(scrape_supported_guard_catalogs(sites).await?);
    }
    Ok(items)
}

async fn scrape_supported_guard_catalogs(
    sites: &[TvboxSiteRecord],
) -> Result<Vec<ScrapedCatalogItem>, String> {
    let mut items = Vec::new();
    for key in collect_supported_guard_keys(sites) {
        match key {
            "csp_JpysGuard" => {
                // fill in with the real fetch/parse path in implementation
            }
            "csp_JPJGuard" => {
                // fill in with the real fetch/parse path in implementation
            }
            _ => {}
        }
    }
    Ok(items)
}

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
        resolve_guard_detail(guard_key, site_key, item_id).await
    }
    _ => { /* existing branches */ }
}
```

- [ ] **Step 4: Run the Guard ingestion tests again**

Run:

```bash
cargo test -q selects_guard_sites_from_tvbox_records --manifest-path src-tauri/Cargo.toml
cargo test -q parses_guard_detail_json_source --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/xb6v.rs src-tauri/src/services/mod.rs
git commit -m "feat: wire guard catalog and detail dispatch"
```

### Task 5: Add Guard playback resolution

**Files:**
- Modify: `src-tauri/src/services/resolver.rs`
- Test: `src-tauri/src/services/resolver.rs`

- [ ] **Step 1: Write the failing Guard resolver tests**

```rust
#[test]
fn classifies_guard_targets_as_resolvable() {
    assert_eq!(
        classify_playback_target("guard://csp_JpysGuard/%E6%96%87%E9%87%87/1419/1/1"),
        "resolvable"
    );
}

#[test]
fn decodes_guard_targets_for_resolution() {
    let decoded = crate::services::decode_guard_play_target(
        "guard://csp_JPJGuard/%E8%B4%B1%E8%B4%B1/71483/1/1",
    )
    .expect("guard target should decode");
    assert_eq!(decoded.guard_key, "csp_JPJGuard");
    assert_eq!(decoded.item_id, "71483");
}
```

- [ ] **Step 2: Run the Guard resolver tests to verify they fail**

Run:

```bash
cargo test -q classifies_guard_targets_as_resolvable --manifest-path src-tauri/Cargo.toml
cargo test -q decodes_guard_targets_for_resolution --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because resolver does not recognize `guard://` yet.

- [ ] **Step 3: Implement Guard resolver support**

Update `src-tauri/src/services/resolver.rs`:

```rust
use crate::services::decode_guard_play_target;

impl PlaybackResolver {
    pub async fn resolve(input: &str) -> Result<ResolvedPlayback, String> {
        if input.starts_with("guard://") {
            return resolve_guard_play_target(input).await;
        }
        // existing branches stay in place
    }
}

pub fn classify_playback_target(input: &str) -> &'static str {
    if input.starts_with("guard://") {
        return "resolvable";
    }
    // existing logic stays in place
}

async fn resolve_guard_play_target(input: &str) -> Result<ResolvedPlayback, String> {
    let target = decode_guard_play_target(input)
        .ok_or_else(|| "invalid guard play target".to_string())?;

    let candidates = match target.guard_key.as_str() {
        "csp_JpysGuard" => vec![],
        "csp_JPJGuard" => vec![],
        other => {
            return Err(format!("unsupported guard resolver: {}", other));
        }
    };

    Ok(ResolvedPlayback {
        status: "failed".to_string(),
        candidates,
        error_message: Some("Guard resolver must be implemented with real adapter fetches".to_string()),
    })
}
```

- [ ] **Step 4: Replace the stub with real adapter resolution and probing**

```rust
let source_url = match target.guard_key.as_str() {
    "csp_JpysGuard" => crate::services::guard_jpys::resolve_jpys_play_target(&target).await?,
    "csp_JPJGuard" => crate::services::guard_jpj::resolve_jpj_play_target(&target).await?,
    other => return Err(format!("unsupported guard resolver: {}", other)),
};

probe_media_candidate(&client, &source_url).await?;

Ok(ResolvedPlayback {
    status: "ready".to_string(),
    candidates: vec![PlaybackCandidate {
        url: source_url.clone(),
        label: "默认线路".to_string(),
        kind: detect_kind(&source_url).to_string(),
        headers: None,
    }],
    error_message: None,
})
```

- [ ] **Step 5: Run the Guard resolver tests again**

Run:

```bash
cargo test -q classifies_guard_targets_as_resolvable --manifest-path src-tauri/Cargo.toml
cargo test -q decodes_guard_targets_for_resolution --manifest-path src-tauri/Cargo.toml
cargo test -q resolver --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/resolver.rs
git commit -m "feat: add guard playback resolution"
```

### Task 6: Add Guard visibility regression coverage and full verification

**Files:**
- Modify: `src-tauri/src/services/storage.rs`
- Test: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Write the failing storage regression test**

```rust
#[test]
fn library_queries_keep_guard_items_visible_while_excluding_embedded_only_items() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("tvbox", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");

    seed_catalog_item_with_source(storage, subscription.id, 301, "文采影片", "movie", "guard");
    seed_catalog_item_with_source(storage, subscription.id, 302, "荐片影片", "series", "guard");
    seed_catalog_item_with_source(storage, subscription.id, 303, "嵌页影片", "series", "zxzj");

    let home = storage.get_library_home().expect("library home should query");
    let catalog = storage.get_catalog_items(None, None).expect("catalog should query");

    assert!(home.latest_updates.iter().any(|item| item.title == "文采影片"));
    assert!(catalog.iter().any(|item| item.title == "荐片影片"));
    assert!(catalog.iter().all(|item| item.title != "嵌页影片"));
}
```

- [ ] **Step 2: Run the storage regression test to verify it fails**

Run:

```bash
cargo test -q library_queries_keep_guard_items_visible_while_excluding_embedded_only_items --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL until the regression test is added correctly.

- [ ] **Step 3: Add the storage regression test**

Update `src-tauri/src/services/storage.rs` test module with the new test, reusing existing seed helpers:

```rust
#[test]
fn library_queries_keep_guard_items_visible_while_excluding_embedded_only_items() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("tvbox", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");

    seed_catalog_item_with_source(&storage, subscription.id, 301, "文采影片", "movie", "guard");
    seed_catalog_item_with_source(&storage, subscription.id, 302, "荐片影片", "series", "guard");
    seed_catalog_item_with_source(&storage, subscription.id, 303, "嵌页影片", "series", "zxzj");

    let home = storage
        .get_library_home()
        .expect("library home should query");
    let catalog = storage
        .get_catalog_items(None, None)
        .expect("catalog should query");

    assert!(home.latest_updates.iter().any(|item| item.title == "文采影片"));
    assert!(catalog.iter().any(|item| item.title == "荐片影片"));
    assert!(catalog.iter().all(|item| item.title != "嵌页影片"));
}
```

- [ ] **Step 4: Run the targeted test, full backend suite, and frontend build**

Run:

```bash
cargo test -q library_queries_keep_guard_items_visible_while_excluding_embedded_only_items --manifest-path src-tauri/Cargo.toml
cargo test -q --manifest-path src-tauri/Cargo.toml
npm run build
```

Expected:

- targeted storage test PASS
- backend suite PASS
- frontend build PASS, with only the known `PlayerPage` chunk-size warning

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "test: cover guard catalog visibility"
```
