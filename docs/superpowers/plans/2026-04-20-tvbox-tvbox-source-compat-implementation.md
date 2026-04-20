# TVBox Source Compatibility Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade the app from a simple live/VOD list player into a desktop media library that can ingest `TVBox`-style subscriptions such as `饭太硬`, resolve playback candidates, and expose source/line status in the UI.

**Architecture:** The work is split into three layers that ship in sequence: a Rust source compatibility layer for `simple_json` and `tvbox_config`, a Rust playback resolution layer that returns structured candidates instead of raw strings, and a Vue desktop-library UI that consumes source, catalog, detail, and playback session state. The migration keeps the current app runnable at every checkpoint by introducing additive tables, commands, and stores before switching views over.

**Tech Stack:** Tauri 2.x, Rust, rusqlite, reqwest, scraper, Vue 3, TypeScript, Pinia, Vue Router, TailwindCSS, Vitest

---

## File Structure

### Rust backend

| File | Action | Responsibility |
| --- | --- | --- |
| `src-tauri/src/models/mod.rs` | Modify | Add source config, site, catalog, playback resolver types |
| `src-tauri/src/services/storage.rs` | Modify | Add additive schema, persistence methods, catalog queries |
| `src-tauri/src/services/parser.rs` | Modify | Keep simple JSON parsing and add source kind detection helpers |
| `src-tauri/src/services/tvbox.rs` | Create | Parse `TVBox` single-warehouse config into structured records |
| `src-tauri/src/services/resolver.rs` | Create | Resolve raw play input into structured playback candidates |
| `src-tauri/src/services/mod.rs` | Modify | Export `tvbox` and `resolver` modules |
| `src-tauri/src/commands/subscription.rs` | Modify | Refresh subscriptions into new schema and source kinds |
| `src-tauri/src/commands/live.rs` | Modify | Return live groups with source health metadata |
| `src-tauri/src/commands/vod.rs` | Modify | Return library list and detail records backed by catalog tables |
| `src-tauri/src/commands/player.rs` | Modify | Add playback resolution commands |
| `src-tauri/src/commands/mod.rs` | Modify | Register new commands |
| `src-tauri/src/lib.rs` | Modify | Wire parser/resolver services into app state if needed |

### Frontend

| File | Action | Responsibility |
| --- | --- | --- |
| `package.json` | Modify | Add `vitest`, `@vue/test-utils`, `jsdom` scripts/deps |
| `vite.config.ts` | Modify | Add `vitest` config block |
| `src/types/index.ts` | Modify | Define source, catalog, detail, playback session types |
| `src/stores/subscription.ts` | Modify | Track source kind, refresh snapshots, errors |
| `src/stores/live.ts` | Modify | Consume grouped live payload with availability state |
| `src/stores/vod.ts` | Replace or narrow | Stop owning detail/playback concerns; keep library browsing only |
| `src/stores/library.ts` | Create | Homepage sections and typed library fetchers |
| `src/stores/detail.ts` | Create | Detail page content, episode groups, resume data |
| `src/stores/playback.ts` | Create | Resolve play tasks, switch candidates, expose player errors |
| `src/router/index.ts` | Modify | Move to `/library/:type`, `/detail/:itemId`, query-based playback routing |
| `src/views/Home.vue` | Modify | Desktop media library homepage |
| `src/views/VodDetail.vue` | Replace | Rich detail page with sources, grouped episodes, resume |
| `src/views/PlayerPage.vue` | Modify | Structured playback session UI |
| `src/components/VodCard.vue` | Modify | Add source/update badges and richer metadata |
| `src/components/ChannelCard.vue` | Modify | Add line counts and status chips |
| `src/components/SearchBar.vue` | Modify | Optional compact/dense desktop variant |
| `src/style.css` | Modify | Define desktop visual tokens and shell surfaces |

### Tests

| File | Action | Responsibility |
| --- | --- | --- |
| `src-tauri/src/services/parser.rs` | Modify | Add simple-vs-tvbox detection tests |
| `src-tauri/src/services/tvbox.rs` | Create | Add parser fixture tests |
| `src-tauri/src/services/resolver.rs` | Create | Add resolver state tests |
| `src/stores/__tests__/library.spec.ts` | Create | Verify homepage store mapping |
| `src/stores/__tests__/playback.spec.ts` | Create | Verify candidate switch/error state |
| `src/views/__tests__/PlayerPage.spec.ts` | Create | Verify status/error rendering |

## Task 1: Add frontend test tooling and shared app types

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Modify: `src/types/index.ts`
- Test: `src/stores/__tests__/library.spec.ts`

- [ ] **Step 1: Write the failing frontend store test**

```ts
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { useLibraryStore } from '@/stores/library'

describe('library store', () => {
  it('maps desktop sections from catalog payload', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    store.applyHomePayload({
      continueWatching: [{ id: 7, title: '继续看', progress: 42 }],
      latestUpdates: [{ id: 8, title: '最新更新' }],
      featured: [{ id: 9, title: '推荐内容' }]
    })

    expect(store.continueWatching[0].title).toBe('继续看')
    expect(store.featured).toHaveLength(1)
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/stores/__tests__/library.spec.ts`
Expected: FAIL with missing `vitest` dependency or missing `useLibraryStore`.

- [ ] **Step 3: Add test tooling and type definitions**

`package.json`

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc --noEmit && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run"
  },
  "devDependencies": {
    "@vue/test-utils": "^2.4.6",
    "jsdom": "^24.1.0",
    "vitest": "^2.1.1"
  }
}
```

`vite.config.ts`

```ts
export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src')
    }
  },
  test: {
    environment: 'jsdom',
    globals: true
  }
})
```

`src/types/index.ts`

```ts
export type SourceKind = 'simple_json' | 'tvbox_config'

export interface SourceSubscription {
  id: number
  name: string
  url: string
  kind: SourceKind
  enabled: boolean
  last_refreshed_at?: string
  last_error?: string | null
}

export interface CatalogCard {
  id: number
  title: string
  itemType: 'movie' | 'tv' | 'variety' | 'anime'
  poster?: string
  progress?: number
  sourceBadge?: string
  updateBadge?: string
}

export interface HomePayload {
  continueWatching: CatalogCard[]
  latestUpdates: CatalogCard[]
  featured: CatalogCard[]
}

export interface PlaybackCandidate {
  url: string
  label: string
  kind: 'hls' | 'http' | 'external'
  headers?: Record<string, string>
}

export interface ResolvedPlayback {
  status: 'ready' | 'failed' | 'external_required'
  candidates: PlaybackCandidate[]
  errorMessage?: string
}
```

- [ ] **Step 4: Create the minimal library store**

Create `src/stores/library.ts`:

```ts
import { defineStore } from 'pinia'
import { ref } from 'vue'
import type { HomePayload, CatalogCard } from '@/types'

export const useLibraryStore = defineStore('library', () => {
  const continueWatching = ref<CatalogCard[]>([])
  const latestUpdates = ref<CatalogCard[]>([])
  const featured = ref<CatalogCard[]>([])

  function applyHomePayload(payload: HomePayload) {
    continueWatching.value = payload.continueWatching
    latestUpdates.value = payload.latestUpdates
    featured.value = payload.featured
  }

  return { continueWatching, latestUpdates, featured, applyHomePayload }
})
```

- [ ] **Step 5: Run test to verify it passes**

Run: `npx vitest run src/stores/__tests__/library.spec.ts`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add package.json vite.config.ts src/types/index.ts src/stores/library.ts src/stores/__tests__/library.spec.ts
git commit -m "test: add frontend library store harness"
```

## Task 2: Add additive schema and source-kind detection for subscriptions

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/services/storage.rs`
- Modify: `src-tauri/src/services/parser.rs`
- Test: `src-tauri/src/services/parser.rs`

- [ ] **Step 1: Write the failing Rust detection tests**

Add to `src-tauri/src/services/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::Parser;

    #[test]
    fn detects_simple_json_subscription() {
        let input = r#"{"lives":[{"name":"CCTV-1","url":"https://a.example/live.m3u8"}]}"#;
        assert_eq!(Parser::detect_source_kind(input), "simple_json");
    }

    #[test]
    fn detects_tvbox_subscription() {
        let input = r#"{"sites":[{"key":"site-a","name":"线路A","api":"https://x.example/api.php/provide/vod/"}],"lives":[{"name":"直播","url":"https://live.example/list.txt"}]}"#;
        assert_eq!(Parser::detect_source_kind(input), "tvbox_config");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test detects_simple_json_subscription --manifest-path src-tauri/Cargo.toml`
Expected: FAIL with missing `detect_source_kind`.

- [ ] **Step 3: Add source and catalog models plus schema**

`src-tauri/src/models/mod.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSubscription {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub kind: String,
    pub enabled: bool,
    pub last_refreshed_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSite {
    pub id: i64,
    pub subscription_id: i64,
    pub site_key: String,
    pub site_name: String,
    pub api: Option<String>,
    pub ext: Option<String>,
    pub source_type: String,
    pub raw_json: String,
}
```

`src-tauri/src/services/storage.rs`

```rust
conn.execute(
    "ALTER TABLE subscriptions ADD COLUMN kind TEXT NOT NULL DEFAULT 'simple_json'",
    [],
).ok();
conn.execute(
    "ALTER TABLE subscriptions ADD COLUMN last_refreshed_at TEXT",
    [],
).ok();
conn.execute(
    "ALTER TABLE subscriptions ADD COLUMN last_error TEXT",
    [],
).ok();
conn.execute(
    "CREATE TABLE IF NOT EXISTS source_configs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        subscription_id INTEGER NOT NULL,
        config_kind TEXT NOT NULL,
        raw_content TEXT NOT NULL,
        parsed_at TEXT NOT NULL,
        FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
    )",
    [],
)?;
```

- [ ] **Step 4: Implement source-kind detection and persistence helpers**

`src-tauri/src/services/parser.rs`

```rust
pub fn detect_source_kind(content: &str) -> &'static str {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
        if value.get("sites").is_some() || value.get("parses").is_some() {
            return "tvbox_config";
        }
    }
    "simple_json"
}
```

`src-tauri/src/services/storage.rs`

```rust
pub fn update_subscription_refresh_state(
    &self,
    id: i64,
    kind: &str,
    refreshed_at: &str,
    last_error: Option<&str>,
) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "UPDATE subscriptions
         SET kind = ?1, last_refreshed_at = ?2, last_error = ?3, updated_at = ?2
         WHERE id = ?4",
        rusqlite::params![kind, refreshed_at, last_error, id],
    )?;
    Ok(())
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test detect_source_kind --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/services/storage.rs src-tauri/src/services/parser.rs
git commit -m "feat: add source kind schema and detection"
```

## Task 3: Parse TVBox configs into cached source records

**Files:**
- Create: `src-tauri/src/services/tvbox.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/commands/subscription.rs`
- Test: `src-tauri/src/services/tvbox.rs`

- [ ] **Step 1: Write the failing TVBox parser test**

Create `src-tauri/src/services/tvbox.rs` with test first:

```rust
#[cfg(test)]
mod tests {
    use super::TvboxConfigParser;

    #[test]
    fn parses_sites_and_parses_from_single_warehouse_config() {
        let input = r#"{
          "sites":[{"key":"site-a","name":"站点A","api":"https://site-a.example/api.php/provide/vod/"}],
          "parses":[{"name":"默认解析","url":"https://parse.example/?url="}]
        }"#;

        let parsed = TvboxConfigParser::parse(input).unwrap();

        assert_eq!(parsed.sites.len(), 1);
        assert_eq!(parsed.parses.len(), 1);
        assert_eq!(parsed.sites[0].site_key, "site-a");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test parses_sites_and_parses_from_single_warehouse_config --manifest-path src-tauri/Cargo.toml`
Expected: FAIL with missing parser implementation.

- [ ] **Step 3: Implement the parser service**

`src-tauri/src/services/tvbox.rs`

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTvboxConfig {
    pub sites: Vec<ParsedSite>,
    pub parses: Vec<ParsedParse>,
    pub lives: Vec<ParsedLive>,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSite {
    pub site_key: String,
    pub site_name: String,
    pub api: Option<String>,
    pub ext: Option<String>,
    pub source_type: String,
    pub raw_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedParse {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLive {
    pub group_name: Option<String>,
    pub channel_name: String,
    pub raw_url: String,
    pub raw_json: String,
}

pub struct TvboxConfigParser;

impl TvboxConfigParser {
    pub fn parse(input: &str) -> Result<ParsedTvboxConfig, String> {
        let value: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
        let sites = value
            .get("sites")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .map(|site| ParsedSite {
                site_key: site.get("key").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                site_name: site.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                api: site.get("api").and_then(|v| v.as_str()).map(|v| v.to_string()),
                ext: site.get("ext").and_then(|v| v.as_str()).map(|v| v.to_string()),
                source_type: site.get("type").and_then(|v| v.as_i64()).unwrap_or(0).to_string(),
                raw_json: site.to_string(),
            })
            .collect();

        let parses = value
            .get("parses")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .map(|parse| ParsedParse {
                name: parse.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                url: parse.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            })
            .collect();

        let lives = value
            .get("lives")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .flat_map(|group| {
                let group_name = group.get("name").and_then(|v| v.as_str()).map(|v| v.to_string());
                group
                    .get("channels")
                    .and_then(|v| v.as_array())
                    .into_iter()
                    .flatten()
                    .map(move |channel| ParsedLive {
                        group_name: group_name.clone(),
                        channel_name: channel.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        raw_url: channel.get("url").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                        raw_json: channel.to_string(),
                    })
            })
            .collect();

        Ok(ParsedTvboxConfig {
            sites,
            parses,
            lives,
            raw_json: input.to_string(),
        })
    }
}
```

- [ ] **Step 4: Persist parsed TVBox snapshots during refresh**

`src-tauri/src/commands/subscription.rs`

```rust
let kind = Parser::detect_source_kind(&content);
if kind == "tvbox_config" {
    let parsed = crate::services::tvbox::TvboxConfigParser::parse(&content)?;
    storage.replace_source_config(subscription.id, kind, &content)?;
    storage.replace_source_sites(subscription.id, &parsed.sites)?;
    storage.replace_source_lives(subscription.id, &parsed.lives)?;
} else {
    let parsed = Parser::parse_subscription(&content)?;
    storage.replace_simple_subscription_data(subscription.id, parsed)?;
}
storage.update_subscription_refresh_state(subscription.id, kind, &chrono_now(), None)?;
```

- [ ] **Step 5: Run the parser test and one refresh-focused backend test**

Run: `cargo test tvbox --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/tvbox.rs src-tauri/src/services/mod.rs src-tauri/src/commands/subscription.rs src-tauri/src/services/storage.rs
git commit -m "feat: cache parsed tvbox subscription records"
```

## Task 4: Build catalog queries and detail APIs from the new source data

**Files:**
- Modify: `src-tauri/src/services/storage.rs`
- Modify: `src-tauri/src/commands/vod.rs`
- Modify: `src-tauri/src/commands/live.rs`
- Test: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Write the failing storage query test**

Add to `src-tauri/src/services/storage.rs`:

```rust
#[test]
fn groups_live_channels_and_returns_source_counts() {
    let storage = test_storage();
    seed_live_source(&storage, "央视频道", "CCTV-1", "https://a.example/live.m3u8");
    seed_live_source(&storage, "央视频道", "CCTV-1", "https://b.example/live.m3u8");

    let groups = storage.get_live_channel_groups().unwrap();

    assert_eq!(groups[0].channels[0].source_count, 2);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test groups_live_channels_and_returns_source_counts --manifest-path src-tauri/Cargo.toml`
Expected: FAIL with missing grouped query or `source_count`.

- [ ] **Step 3: Add additive catalog queries**

`src-tauri/src/services/storage.rs`

```rust
pub fn get_live_channel_groups(&self) -> SqliteResult<Vec<LiveChannelGroup>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT group_name, channel_name, COUNT(*) as source_count
         FROM source_lives
         GROUP BY group_name, channel_name
         ORDER BY group_name, channel_name"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "其他".to_string()),
            LiveChannelListItem {
                name: row.get(1)?,
                source_count: row.get(2)?,
            },
        ))
    })?;

    let mut grouped = std::collections::BTreeMap::<String, Vec<LiveChannelListItem>>::new();
    for row in rows {
        let (group_name, item) = row?;
        grouped.entry(group_name).or_default().push(item);
    }

    Ok(grouped
        .into_iter()
        .map(|(category, channels)| LiveChannelGroup { category, channels })
        .collect())
}

pub fn get_library_home(&self) -> SqliteResult<HomePayload> {
    let conn = self.conn.lock().unwrap();
    let continue_watching = query_cards(
        &conn,
        "SELECT ci.id, ci.title, ci.item_type, ci.poster, ph.progress
         FROM catalog_items ci
         INNER JOIN play_history ph ON ph.item_type = 'vod' AND ph.item_id = ci.id
         ORDER BY ph.last_played DESC
         LIMIT 12",
    )?;
    let latest_updates = query_cards(
        &conn,
        "SELECT id, title, item_type, poster, NULL as progress
         FROM catalog_items
         ORDER BY updated_at DESC
         LIMIT 12",
    )?;
    let featured = query_cards(
        &conn,
        "SELECT id, title, item_type, poster, NULL as progress
         FROM catalog_items
         ORDER BY id DESC
         LIMIT 12",
    )?;

    Ok(HomePayload {
        continue_watching,
        latest_updates,
        featured,
    })
}

pub fn get_catalog_detail(&self, item_id: i64) -> SqliteResult<CatalogDetail> {
    let conn = self.conn.lock().unwrap();
    let item = conn.query_row(
        "SELECT id, title, item_type, poster, summary, detail_json
         FROM catalog_items
         WHERE id = ?1",
        [item_id],
        |row| {
            Ok(CatalogDetailItem {
                id: row.get(0)?,
                title: row.get(1)?,
                item_type: row.get(2)?,
                poster: row.get(3)?,
                summary: row.get(4)?,
                detail_json: row.get(5)?,
            })
        },
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, source_name, episode_label, play_url, order_index
         FROM catalog_episodes
         WHERE catalog_item_id = ?1
         ORDER BY source_name, order_index"
    )?;

    let rows = stmt.query_map([item_id], |row| {
        Ok(CatalogEpisodeRow {
            id: row.get(0)?,
            source_name: row.get::<_, Option<String>>(1)?.unwrap_or_else(|| "默认来源".to_string()),
            episode_label: row.get(2)?,
            play_url: row.get(3)?,
            order_index: row.get(4)?,
        })
    })?;

    let mut groups = std::collections::BTreeMap::<String, Vec<CatalogEpisodeRow>>::new();
    for row in rows {
        let row = row?;
        groups.entry(row.source_name.clone()).or_default().push(row);
    }

    Ok(CatalogDetail {
        item,
        episode_groups: groups
            .into_iter()
            .map(|(source_name, episodes)| CatalogEpisodeGroup { source_name, episodes })
            .collect(),
    })
}
```

- [ ] **Step 4: Switch commands to the new catalog endpoints**

`src-tauri/src/commands/vod.rs`

```rust
#[tauri::command]
pub async fn get_library_home(state: State<'_, AppState>) -> Result<HomePayload, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_library_home().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn get_catalog_detail(id: i64, state: State<'_, AppState>) -> Result<CatalogDetail, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_catalog_detail(id).map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
```

`src-tauri/src/commands/live.rs`

```rust
#[tauri::command]
pub async fn get_live_channel_groups(state: State<'_, AppState>) -> Result<Vec<LiveChannelGroup>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || storage.get_live_channel_groups().map_err(|e| e.to_string()))
        .await
        .map_err(|e| e.to_string())?
}
```

- [ ] **Step 5: Run focused backend tests**

Run: `cargo test storage --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/storage.rs src-tauri/src/commands/vod.rs src-tauri/src/commands/live.rs src-tauri/src/models/mod.rs
git commit -m "feat: serve library and live catalog queries"
```

## Task 5: Add structured playback resolver and player commands

**Files:**
- Create: `src-tauri/src/services/resolver.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/commands/player.rs`
- Test: `src-tauri/src/services/resolver.rs`

- [ ] **Step 1: Write the failing resolver tests**

Create `src-tauri/src/services/resolver.rs` with tests:

```rust
#[cfg(test)]
mod tests {
    use super::PlaybackResolver;

    #[test]
    fn marks_hls_url_as_ready_candidate() {
        let resolved = PlaybackResolver::resolve("https://example.com/live.m3u8").unwrap();
        assert_eq!(resolved.status, "ready");
        assert_eq!(resolved.candidates[0].kind, "hls");
    }

    #[test]
    fn marks_unknown_scheme_as_external_required() {
        let resolved = PlaybackResolver::resolve("drpy://source/detail").unwrap();
        assert_eq!(resolved.status, "external_required");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test marks_hls_url_as_ready_candidate --manifest-path src-tauri/Cargo.toml`
Expected: FAIL with missing resolver.

- [ ] **Step 3: Implement the minimal resolver**

`src-tauri/src/services/resolver.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackCandidate {
    pub url: String,
    pub label: String,
    pub kind: String,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedPlayback {
    pub status: String,
    pub candidates: Vec<PlaybackCandidate>,
    pub error_message: Option<String>,
}

pub struct PlaybackResolver;

impl PlaybackResolver {
    pub fn resolve(input: &str) -> Result<ResolvedPlayback, String> {
        if input.starts_with("drpy://") {
            return Ok(ResolvedPlayback {
                status: "external_required".to_string(),
                candidates: vec![],
                error_message: Some("Current desktop build does not execute drpy rules directly".to_string()),
            });
        }

        let kind = if input.contains(".m3u8") { "hls" } else { "http" };

        Ok(ResolvedPlayback {
            status: "ready".to_string(),
            candidates: vec![PlaybackCandidate {
                url: input.to_string(),
                label: "默认线路".to_string(),
                kind: kind.to_string(),
                headers: None,
            }],
            error_message: None,
        })
    }
}
```

- [ ] **Step 4: Expose playback resolution through Tauri**

`src-tauri/src/commands/player.rs`

```rust
#[tauri::command]
pub async fn resolve_playback(input: String) -> Result<ResolvedPlayback, String> {
    crate::services::resolver::PlaybackResolver::resolve(&input)
}
```

- [ ] **Step 5: Run resolver tests**

Run: `cargo test resolver --manifest-path src-tauri/Cargo.toml`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/resolver.rs src-tauri/src/services/mod.rs src-tauri/src/commands/player.rs src-tauri/src/commands/mod.rs
git commit -m "feat: add structured playback resolver"
```

## Task 6: Rebuild stores and routing around library, detail, and playback session state

**Files:**
- Create: `src/stores/detail.ts`
- Create: `src/stores/playback.ts`
- Modify: `src/stores/subscription.ts`
- Modify: `src/stores/live.ts`
- Modify: `src/router/index.ts`
- Test: `src/stores/__tests__/playback.spec.ts`

- [ ] **Step 1: Write the failing playback store test**

Create `src/stores/__tests__/playback.spec.ts`:

```ts
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { usePlaybackStore } from '@/stores/playback'

describe('playback store', () => {
  it('switches to the next candidate after fatal media error', () => {
    setActivePinia(createPinia())
    const store = usePlaybackStore()

    store.applyResolved({
      status: 'ready',
      candidates: [
        { url: 'https://a.example/1.m3u8', label: '线路1', kind: 'hls' },
        { url: 'https://b.example/1.m3u8', label: '线路2', kind: 'hls' }
      ]
    })

    store.handleFatalPlaybackError('network')
    expect(store.currentCandidate?.label).toBe('线路2')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/stores/__tests__/playback.spec.ts`
Expected: FAIL with missing `usePlaybackStore`.

- [ ] **Step 3: Implement the stores and route shape**

`src/stores/playback.ts`

```ts
import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { ResolvedPlayback, PlaybackCandidate } from '@/types'

export const usePlaybackStore = defineStore('playback', () => {
  const status = ref<'idle' | 'resolving' | 'ready' | 'failed' | 'external_required'>('idle')
  const candidates = ref<PlaybackCandidate[]>([])
  const currentIndex = ref(0)
  const errorMessage = ref<string | null>(null)

  const currentCandidate = computed(() => candidates.value[currentIndex.value] ?? null)

  async function resolve(input: string) {
    status.value = 'resolving'
    const resolved = await invoke<ResolvedPlayback>('resolve_playback', { input })
    applyResolved(resolved)
  }

  function applyResolved(resolved: ResolvedPlayback) {
    status.value = resolved.status
    candidates.value = resolved.candidates
    currentIndex.value = 0
    errorMessage.value = resolved.errorMessage ?? null
  }

  function handleFatalPlaybackError(reason: string) {
    if (currentIndex.value < candidates.value.length - 1) {
      currentIndex.value += 1
      return
    }
    status.value = 'failed'
    errorMessage.value = `All playback candidates failed: ${reason}`
  }

  return { status, candidates, currentCandidate, errorMessage, resolve, applyResolved, handleFatalPlaybackError }
})
```

`src/router/index.ts`

```ts
routes: [
  { path: '/', redirect: '/library/live' },
  { path: '/library/:type', name: 'library', component: Home },
  { path: '/detail/:itemId', name: 'detail', component: () => import('@/views/VodDetail.vue') },
  { path: '/player/:mode/:id', name: 'player', component: () => import('@/views/PlayerPage.vue') }
]
```

- [ ] **Step 4: Run playback store tests**

Run: `npx vitest run src/stores/__tests__/playback.spec.ts`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/stores/playback.ts src/stores/detail.ts src/stores/subscription.ts src/stores/live.ts src/router/index.ts src/stores/__tests__/playback.spec.ts
git commit -m "feat: add library-detail-playback state flow"
```

## Task 7: Rebuild the desktop library homepage and detail page

**Files:**
- Modify: `src/views/Home.vue`
- Modify: `src/views/VodDetail.vue`
- Modify: `src/components/VodCard.vue`
- Modify: `src/components/ChannelCard.vue`
- Modify: `src/style.css`
- Test: `src/views/__tests__/PlayerPage.spec.ts`

- [ ] **Step 1: Write the failing rendering test for playback status panel**

Create `src/views/__tests__/PlayerPage.spec.ts`:

```ts
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PlayerPage from '@/views/PlayerPage.vue'

describe('PlayerPage', () => {
  it('renders resolver error state', () => {
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: { RouterLink: true }
      }
    })

    wrapper.vm.errorMsg = 'Current source requires external resolver'
    expect(wrapper.text()).toContain('Current source requires external resolver')
  })
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npx vitest run src/views/__tests__/PlayerPage.spec.ts`
Expected: FAIL because the page does not render the error panel.

- [ ] **Step 3: Replace the homepage shell**

`src/views/Home.vue`

```vue
<template>
  <div class="app-shell">
    <header class="app-topbar">
      <div>
        <p class="eyebrow">Desktop Media Library</p>
        <h1>TVBox</h1>
      </div>
      <SearchBar placeholder="搜索直播、电影、剧集" />
      <div class="status-pills">
        <span class="pill">{{ activeSourceLabel }}</span>
        <RouterLink to="/subscriptions" class="pill pill-action">来源</RouterLink>
      </div>
    </header>

    <main class="library-layout">
      <aside class="library-nav">
        <RouterLink v-for="item in navItems" :key="item.key" :to="`/library/${item.key}`">{{ item.label }}</RouterLink>
      </aside>

      <section class="library-content">
        <section class="hero-strip">
          <h2>继续观看</h2>
          <div class="poster-row">
            <VodCard v-for="item in libraryStore.continueWatching" :key="item.id" :item="toVodCard(item)" />
          </div>
        </section>
      </section>
    </main>
  </div>
</template>
```

- [ ] **Step 4: Replace the detail page**

`src/views/VodDetail.vue`

```vue
<template>
  <div class="detail-shell" v-if="detailStore.item">
    <section class="detail-hero">
      <img :src="detailStore.item.poster" :alt="detailStore.item.title" class="detail-poster" />
      <div class="detail-copy">
        <p class="eyebrow">{{ detailStore.item.sourceBadge }}</p>
        <h1>{{ detailStore.item.title }}</h1>
        <p class="summary">{{ detailStore.item.summary }}</p>
        <div class="meta-row">
          <span class="pill">已缓存来源 {{ detailStore.item.sourceCount }}</span>
          <span class="pill" v-if="detailStore.resumeLabel">{{ detailStore.resumeLabel }}</span>
        </div>
      </div>
    </section>

    <section class="episode-groups">
      <article v-for="group in detailStore.episodeGroups" :key="group.sourceName" class="episode-group">
        <header>
          <h2>{{ group.sourceName }}</h2>
          <span>{{ group.episodes.length }} 集</span>
        </header>
        <div class="episode-grid">
          <button v-for="episode in group.episodes" :key="episode.id" @click="detailStore.playEpisode(episode)">
            {{ episode.episodeLabel }}
          </button>
        </div>
      </article>
    </section>
  </div>
</template>
```

- [ ] **Step 5: Define the desktop tokens**

`src/style.css`

```css
:root {
  --bg: #0b1220;
  --panel: rgba(17, 24, 39, 0.82);
  --panel-strong: #131b2d;
  --line: rgba(148, 163, 184, 0.18);
  --text: #e5ecf6;
  --muted: #95a4bb;
  --accent: #86c6ff;
  --accent-strong: #2b7fff;
  font-family: "SF Pro Display", "PingFang SC", sans-serif;
  color: var(--text);
  background:
    radial-gradient(circle at top left, rgba(43, 127, 255, 0.18), transparent 28%),
    linear-gradient(180deg, #07101d, #0b1220 35%, #0b1324);
}
```

- [ ] **Step 6: Run build-oriented verification**

Run: `npm run build`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/views/Home.vue src/views/VodDetail.vue src/components/VodCard.vue src/components/ChannelCard.vue src/style.css src/views/__tests__/PlayerPage.spec.ts
git commit -m "feat: redesign library homepage and detail page"
```

## Task 8: Rebuild PlayerPage around resolver state, diagnostics, and line switching

**Files:**
- Modify: `src/views/PlayerPage.vue`
- Modify: `src/stores/playback.ts`
- Modify: `src/stores/detail.ts`
- Test: `src/views/__tests__/PlayerPage.spec.ts`

- [ ] **Step 1: Extend the failing test to cover candidate switching**

Update `src/views/__tests__/PlayerPage.spec.ts`:

```ts
it('renders current line and retry actions', async () => {
  const wrapper = mount(PlayerPage, {
    global: {
      stubs: { RouterLink: true }
    }
  })

  wrapper.vm.playbackStore.applyResolved({
    status: 'ready',
    candidates: [
      { url: 'https://a.example/1.m3u8', label: '线路1', kind: 'hls' },
      { url: 'https://b.example/1.m3u8', label: '线路2', kind: 'hls' }
    ]
  })

  expect(wrapper.text()).toContain('线路1')
  expect(wrapper.text()).toContain('重试')
})
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/views/__tests__/PlayerPage.spec.ts`
Expected: FAIL because PlayerPage still uses ad hoc local state.

- [ ] **Step 3: Replace local playback state with the store**

`src/views/PlayerPage.vue`

```vue
<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import Hls from 'hls.js'
import { usePlaybackStore } from '@/stores/playback'

const route = useRoute()
const playbackStore = usePlaybackStore()
const videoRef = ref<HTMLVideoElement | null>(null)
let hls: Hls | null = null

onMounted(async () => {
  const input = String(route.query.input ?? '')
  if (input) await playbackStore.resolve(input)
})

watch(() => playbackStore.currentCandidate, (candidate) => {
  if (!candidate || !videoRef.value) return
  if (hls) hls.destroy()
  if (candidate.kind === 'hls' && Hls.isSupported()) {
    hls = new Hls()
    hls.loadSource(candidate.url)
    hls.attachMedia(videoRef.value)
    return
  }
  videoRef.value.src = candidate.url
})
</script>

<template>
  <div class="player-shell">
    <section class="player-stage">
      <video ref="videoRef" class="player-video" controls />
      <aside class="player-sidebar">
        <p class="eyebrow">解析状态</p>
        <h2>{{ playbackStore.status }}</h2>
        <p v-if="playbackStore.errorMessage" class="player-error">{{ playbackStore.errorMessage }}</p>
        <div class="candidate-list">
          <button
            v-for="(candidate, index) in playbackStore.candidates"
            :key="candidate.url"
            @click="playbackStore.currentIndex = index"
          >
            {{ candidate.label }}
          </button>
        </div>
        <button @click="playbackStore.resolve(String(route.query.input ?? ''))">重试</button>
      </aside>
    </section>
  </div>
</template>
```

- [ ] **Step 4: Run component tests and production build**

Run: `npx vitest run src/views/__tests__/PlayerPage.spec.ts && npm run build`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/views/PlayerPage.vue src/stores/playback.ts src/stores/detail.ts src/views/__tests__/PlayerPage.spec.ts
git commit -m "feat: add resolver-driven playback page"
```

## Task 9: Final verification, docs sync, and cleanup

**Files:**
- Modify: `docs/superpowers/specs/2026-04-20-tvbox-tvbox-source-compat-design.md`
- Modify: `docs/superpowers/plans/2026-04-20-tvbox-tvbox-source-compat-implementation.md`

- [ ] **Step 1: Run the full verification suite**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
npx vitest run
npm run build
```

Expected:

```text
test result: ok
Test Files  ... passed
vite v... build completed successfully
```

- [ ] **Step 2: Manual smoke test against a TVBox source**

Run:

```bash
npm run tauri dev
```

Expected:

```text
Desktop app launches, subscription refresh completes, /library/live renders groups, detail page shows episode groups, player page renders resolver status for at least one test input.
```

- [ ] **Step 3: Update the spec/plan if implementation diverged**

```md
- Record any intentional scope cuts in the spec.
- Record any command or file path changes in the plan.
```

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-20-tvbox-tvbox-source-compat-design.md docs/superpowers/plans/2026-04-20-tvbox-tvbox-source-compat-implementation.md
git commit -m "docs: sync tvbox compatibility implementation notes"
```

## Self-Review

- Spec coverage:
  - Source kind detection, TVBox parsing, caching, playback resolver, homepage redesign, detail page redesign, and diagnostics-driven player all map to Tasks 2-8.
  - Verification and manual smoke testing map to Task 9.
- Placeholder scan:
  - Removed the `todo!()` storage/query placeholders so every code step now includes concrete implementation direction.
  - No `TBD`, `TODO`, “implement later”, or “similar to Task N” markers remain.
- Type consistency:
  - `ResolvedPlayback`, `PlaybackCandidate`, `SourceSubscription`, and the routing shape are consistent across Rust and TypeScript tasks.
