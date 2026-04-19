# VOD Classification & Douban Hot List Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement 6-tab homepage with live channel grouping, Douban hot list crawler, channel deduplication with multi-source switching

**Architecture:**
- Rust backend: Douban crawler (new service), modified storage for grouped channels, new API endpoints
- Vue frontend: 6-tab Home.vue, merged LiveChannel type with sources array, multi-source player support
- SQLite: Add `douban_hot` table, modify channels query for grouping

**Tech Stack:** Rust (Tauri 2.x), Vue 3, TypeScript, Pinia, SQLite (rusqlite), reqwest for HTTP

---

## File Structure

### Rust Backend Files

| File | Action | Purpose |
|------|--------|---------|
| `src-tauri/src/services/douban.rs` | **CREATE** | Douban HTML crawler service |
| `src-tauri/src/services/mod.rs` | **MODIFY** | Add douban module |
| `src-tauri/src/services/storage.rs` | **MODIFY** | Add `douban_hot` table, grouped channels query, merge logic |
| `src-tauri/src/commands/douban.rs` | **CREATE** | Tauri commands for Douban API |
| `src-tauri/src/commands/mod.rs` | **MODIFY** | Add douban module |
| `src-tauri/src/commands/live.rs` | **MODIFY** | Return merged channels with sources |
| `src-tauri/src/commands/vod.rs` | **MODIFY** | Add type filter + search |
| `src-tauri/src/models/mod.rs` | **MODIFY** | Add `DoubanHot` model, modify `LiveChannel` for sources |

### Frontend Files

| File | Action | Purpose |
|------|--------|---------|
| `src/types/index.ts` | **MODIFY** | `LiveChannel.sources[]`, `DoubanHotItem` |
| `src/stores/live.ts` | **MODIFY** | Add grouped channels support |
| `src/stores/vod.ts` | **MODIFY** | Add type filter + search, Douban hot items |
| `src/stores/douban.ts` | **CREATE** | Pinia store for Douban hot list |
| `src/views/Home.vue` | **MODIFY** | 6 tabs: live/hot/movie/tv/variety/anime |
| `src/components/ChannelCard.vue` | **MODIFY** | Show multi-source indicator |
| `src/views/PlayerPage.vue` | **MODIFY** | Add source switching UI |

---

## Implementation Phases

### Phase 1: Douban Crawler (Backend)

#### Task 1: Add DoubanHot Model

**Files:**
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: Add DoubanHot struct**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanHot {
    pub id: i64,
    pub name: String,
    pub year: Option<i32>,
    pub poster: Option<String>,
    pub rating: Option<f64>,
    pub rank: i32,
    pub updated_at: String,
}
```

#### Task 2: Create Douban Crawler Service

**Files:**
- Create: `src-tauri/src/services/douban.rs`
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Add douban module export**

```rust
pub mod douban;
```

- [ ] **Step 2: Create douban.rs with crawler**

```rust
use crate::models::DoubanHot;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

pub struct DoubanCrawler {
    client: Client,
}

impl DoubanCrawler {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    pub async fn fetch_hot_list(&self) -> Result<Vec<DoubanHot>, String> {
        let url = "https://movie.douban.com/chart";
        let resp = self.client.get(url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        let html = resp.text().await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let document = Html::parse_document(&html);
        self.parse_hot_list(&document)
    }

    fn parse_hot_list(&self, document: &Html) -> Result<Vec<DoubanHot>, String> {
        let mut items = Vec::new();
        // TODO: Parse HTML structure
        Ok(items)
    }
}
```

#### Task 3: Add douban_hot Table to Storage

**Files:**
- Modify: `src-tauri/src/services/storage.rs:29-105`

- [ ] **Step 1: Add table creation in init_tables()**

```rust
conn.execute(
    "CREATE TABLE IF NOT EXISTS douban_hot (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        year INTEGER,
        poster TEXT,
        rating REAL,
        rank INTEGER NOT NULL,
        updated_at TEXT NOT NULL
    )",
    [],
)?;

conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_douban_name ON douban_hot(name)",
    [],
)?;
```

- [ ] **Step 2: Add storage methods**

```rust
pub fn get_douban_hot(&self) -> SqliteResult<Vec<DoubanHot>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, year, poster, rating, rank, updated_at FROM douban_hot ORDER BY rank LIMIT 100"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DoubanHot {
            id: row.get(0)?,
            name: row.get(1)?,
            year: row.get(2)?,
            poster: row.get(3)?,
            rating: row.get(4)?,
            rank: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn upsert_douban_hot(&self, items: &[DoubanHot]) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    for item in items {
        conn.execute(
            "INSERT OR REPLACE INTO douban_hot (name, year, poster, rating, rank, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![item.name, item.year, item.poster, item.rating, item.rank, item.updated_at],
        )?;
    }
    Ok(())
}

pub fn clear_douban_hot(&self) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute("DELETE FROM douban_hot", [])?;
    Ok(())
}
```

#### Task 4: Create Douban Commands

**Files:**
- Create: `src-tauri/src/commands/douban.rs`
- Modify: `src-tauri/src/commands/mod.rs`

- [ ] **Step 1: Add douban command handlers**

```rust
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn get_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    state.storage.get_douban_hot().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn fetch_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    let crawler = crate::services::douban::DoubanCrawler::new();
    let items = crawler.fetch_hot_list().await?;
    state.storage.clear_douban_hot().map_err(|e| e.to_string())?;
    state.storage.upsert_douban_hot(&items).map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
pub async fn get_matched_hot_list(state: State<'_, AppState>) -> Result<Vec<MatchedHotItem>, String> {
    // Get Douban items and match with VOD items from subscriptions
    let douban_items = state.storage.get_douban_hot().map_err(|e| e.to_string())?;
    let vod_items = state.storage.get_vod_items(None, 0).map_err(|e| e.to_string())?;

    let matched = douban_items
        .into_iter()
        .filter_map(|douban| {
            // Try to find matching VOD item
            vod_items.iter().find(|vod| {
                fuzzy_match(&douban.name, &vod.name, douban.year)
            }).map(|vod| MatchedHotItem {
                douban,
                vod_id: vod.id,
                vod_name: vod.name.clone(),
            })
        })
        .collect();

    Ok(matched)
}

fn fuzzy_match(douban_name: &str, vod_name: &str, douban_year: Option<i32>) -> bool {
    // Normalize names
    let d = normalize_name(douban_name);
    let v = normalize_name(vod_name);

    // Calculate similarity
    let similarity = calculate_similarity(&d, &v);
    if similarity < 0.8 {
        return false;
    }

    // Year check (if both have year info)
    if let (Some(dy), Some(vy)) = (douban_year, extract_year(vod_name)) {
        if (dy - vy).abs() > 1 {
            return false;
        }
    }

    true
}

fn normalize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn calculate_similarity(a: &str, b: &str) -> f64 {
    // Simple Jaccard similarity based on character n-grams
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();

    if a_chars.is_empty() || b_chars.is_empty() {
        return 0.0;
    }

    let a_ngrams: std::collections::HashSet<String> = (0..a_chars.len())
        .filter_map(|i| {
            if i + 2 <= a_chars.len() {
                Some(a_chars[i..i+2].iter().collect())
            } else {
                None
            }
        })
        .collect();

    let b_ngrams: std::collections::HashSet<String> = (0..b_chars.len())
        .filter_map(|i| {
            if i + 2 <= b_chars.len() {
                Some(b_chars[i..i+2].iter().collect())
            } else {
                None
            }
        })
        .collect();

    let intersection = a_ngrams.intersection(&b_ngrams).count();
    let union = a_ngrams.union(&b_ngrams).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn extract_year(name: &str) -> Option<i32> {
    // Extract 4-digit year from name like "Movie (2023)" or "Movie 2023"
    let re = regex::Regex::new(r"\((\d{4})\)|(\d{4})").ok()?;
    for cap in re.captures_iter(name) {
        if let Some(m1) = cap.get(1) {
            return m1.as_str().parse().ok();
        }
        if let Some(m2) = cap.get(2) {
            return m2.as_str().parse().ok();
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedHotItem {
    pub douban: DoubanHot,
    pub vod_id: i64,
    pub vod_name: String,
}
```

- [ ] **Step 2: Export in commands/mod.rs**

```rust
pub mod douban;
```

#### Task 4b: Add Regex Dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add regex to dependencies**

```toml
regex = "1"
```

---

### Phase 2: Channel Merging & Multi-Source

#### Task 5: Update LiveChannel Model with Sources

**Files:**
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: Add ChannelSource and MergedLiveChannel**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelSource {
    pub url: String,
    pub subscription_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedLiveChannel {
    pub id: i64,
    pub name: String,
    pub logo: Option<String>,
    pub category: Option<String>,
    pub sources: Vec<ChannelSource>,
}
```

#### Task 6: Add Merged Channels Query in Storage

**Files:**
- Modify: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Add grouped/merged channels query**

```rust
pub fn get_merged_live_channels(&self) -> SqliteResult<Vec<MergedLiveChannel>> {
    let conn = self.conn.lock().unwrap();

    // Get all channels grouped by name+category
    let mut stmt = conn.prepare(
        "SELECT lc.name, lc.logo, lc.category,
                GROUP_CONCAT(lc.url || '|' || lc.subscription_id) as sources
         FROM live_channels lc
         INNER JOIN subscriptions s ON lc.subscription_id = s.id
         WHERE s.enabled = 1
         GROUP BY lc.name, lc.category
         ORDER BY lc.category, lc.name"
    )?;

    let rows = stmt.query_map([], |row| {
        let sources_str: String = row.get(3)?;
        let sources: Vec<ChannelSource> = sources_str
            .split(',')
            .filter_map(|s| {
                let parts: Vec<&str> = s.split('|').collect();
                if parts.len() == 2 {
                    Some(ChannelSource {
                        url: parts[0].to_string(),
                        subscription_id: parts[1].parse().unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(MergedLiveChannel {
            id: 0, // Will be assigned by frontend
            name: row.get(0)?,
            logo: row.get(1)?,
            category: row.get(2)?,
            sources,
        })
    })?;
    rows.collect()
}

pub fn get_merged_live_channels_by_category(&self, category: &str) -> SqliteResult<Vec<MergedLiveChannel>> {
    let conn = self.conn.lock().unwrap();

    let mut stmt = conn.prepare(
        "SELECT lc.name, lc.logo, lc.category,
                GROUP_CONCAT(lc.url || '|' || lc.subscription_id) as sources
         FROM live_channels lc
         INNER JOIN subscriptions s ON lc.subscription_id = s.id
         WHERE s.enabled = 1 AND lc.category = ?1
         GROUP BY lc.name, lc.category
         ORDER BY lc.name"
    )?;

    let rows = stmt.query_map([category], |row| {
        let sources_str: String = row.get(3)?;
        let sources: Vec<ChannelSource> = sources_str
            .split(',')
            .filter_map(|s| {
                let parts: Vec<&str> = s.split('|').collect();
                if parts.len() == 2 {
                    Some(ChannelSource {
                        url: parts[0].to_string(),
                        subscription_id: parts[1].parse().unwrap_or(0),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(MergedLiveChannel {
            id: 0,
            name: row.get(0)?,
            logo: row.get(1)?,
            category: row.get(2)?,
            sources,
        })
    })?;
    rows.collect()
}
```

#### Task 7: Update Live Commands for Merged Channels

**Files:**
- Modify: `src-tauri/src/commands/live.rs`

- [ ] **Step 1: Update get_live_channels command**

```rust
#[tauri::command]
pub fn get_live_channels(state: State<'_, AppState>, category: Option<String>) -> Result<Vec<MergedLiveChannel>, String> {
    match category {
        Some(cat) => state.storage.get_merged_live_channels_by_category(&cat)
            .map_err(|e| e.to_string()),
        None => state.storage.get_merged_live_channels()
            .map_err(|e| e.to_string()),
    }
}

#[tauri::command]
pub fn get_live_channel_groups(state: State<'_, AppState>) -> Result<Vec<LiveChannelGroup>, String> {
    // Returns categories with channels grouped
    let categories = state.storage.get_live_categories().map_err(|e| e.to_string())?;

    let mut groups = Vec::new();
    for cat in categories {
        // Filter out non-TV categories
        if ["原创IP", "视频源", "手工绘画", "生活杂谈", "一起看", "电子榨菜"].contains(&cat.as_str()) {
            continue;
        }

        let channels = state.storage.get_merged_live_channels_by_category(&cat)
            .map_err(|e| e.to_string())?;

        if !channels.is_empty() {
            groups.push(LiveChannelGroup {
                category: cat,
                channels,
            });
        }
    }

    // Sort by predefined order
    groups.sort_by(|a, b| {
        let order = |c: &str| {
            match c {
                "央视频道" | "央视IPV4" => 0,
                "卫视频道" | "卫视IPV4" => 1,
                "港台" => 2,
                "运动体育" => 3,
                _ => 4,
            }
        };
        order(&a.category).cmp(&order(&b.category))
    });

    Ok(groups)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveChannelGroup {
    pub category: String,
    pub channels: Vec<MergedLiveChannel>,
}
```

---

### Phase 3: Frontend Type Updates

#### Task 8: Update TypeScript Types

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: Update LiveChannel and add DoubanHotItem**

```typescript
export interface ChannelSource {
  url: string
  subscription_id: number
}

export interface LiveChannel {
  id: number
  name: string
  logo?: string
  category: string
  sources: ChannelSource[]
}

export interface LiveChannelGroup {
  category: string
  channels: LiveChannel[]
}

export interface DoubanHotItem {
  id: number
  name: string
  year?: number
  poster?: string
  rating?: number
  rank: number
}
```

#### Task 9: Create Douban Store

**Files:**
- Create: `src/stores/douban.ts`

- [ ] **Step 1: Create Pinia store**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DoubanHotItem, VodItem } from '@/types'

export interface MatchedHotItem {
  douban: DoubanHotItem
  vod_id: number
  vod_name: string
}

export const useDoubanStore = defineStore('douban', () => {
  const items = ref<DoubanHotItem[]>([])
  const matchedItems = ref<MatchedHotItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchHot() {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<DoubanHotItem[]>('get_douban_hot')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchMatchedHot() {
    loading.value = true
    error.value = null
    try {
      matchedItems.value = await invoke<MatchedHotItem[]>('get_matched_hot_list')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function refreshHot() {
    loading.value = true
    error.value = null
    try {
      await invoke('fetch_douban_hot')
      await fetchMatchedHot()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { items, matchedItems, loading, error, fetchHot, fetchMatchedHot, refreshHot }
})
```

#### Task 10: Update Live Store

**Files:**
- Modify: `src/stores/live.ts`

- [ ] **Step 1: Add grouped channels support**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LiveChannel, LiveChannelGroup } from '@/types'

export const useLiveStore = defineStore('live', () => {
  const channels = ref<LiveChannel[]>([])
  const groups = ref<LiveChannelGroup[]>([])
  const categories = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchChannels(category?: string) {
    loading.value = true
    error.value = null
    try {
      channels.value = await invoke<LiveChannel[]>('get_live_channels', { category: category || null })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchGroups() {
    loading.value = true
    error.value = null
    try {
      groups.value = await invoke<LiveChannelGroup[]>('get_live_channel_groups')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchCategories() {
    try {
      categories.value = await invoke<string[]>('get_live_categories')
    } catch (e) {
      error.value = String(e)
    }
  }

  return { channels, groups, categories, loading, error, fetchChannels, fetchGroups, fetchCategories }
})
```

#### Task 11: Update Vod Store

**Files:**
- Modify: `src/stores/vod.ts`

- [ ] **Step 1: Add type filter and search**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VodItem } from '@/types'

export const useVodStore = defineStore('vod', () => {
  const items = ref<VodItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchItems(type?: string) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('get_vod_items', { vtype: type || null, page: 0 })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function search(keyword: string) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('search_vod', { keyword })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { items, loading, error, fetchItems, search }
})
```

---

### Phase 4: Home.vue 6-Tab UI

#### Task 12: Rewrite Home.vue with 6 Tabs

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: Add tabs configuration and state**

```typescript
const tabs = [
  { key: 'live', label: '直播', icon: '📺' },
  { key: 'hot', label: '热门', icon: '🔥' },
  { key: 'movie', label: '电影', icon: '🎬' },
  { key: 'tv', label: '电视剧', icon: '📺' },
  { key: 'variety', label: '综艺', icon: '🎭' },
  { key: 'anime', label: '动漫', icon: '🅰️' }
]

const activeTab = ref('live')
const searchKeyword = ref('')
```

- [ ] **Step 2: Tab content rendering**

```vue
<!-- Live Tab: Grouped channels -->
<div v-if="activeTab === 'live'" class="live-section">
  <SearchBar placeholder="搜索频道..." @search="handleLiveSearch" />
  <div v-for="group in liveStore.groups" :key="group.category" class="channel-group mb-6">
    <h3 class="text-lg font-bold mb-3">{{ group.category }}</h3>
    <div class="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
      <ChannelCard
        v-for="channel in group.channels.slice(0, 20)"
        :key="channel.id"
        :channel="channel"
        @play="handlePlayChannel"
      />
    </div>
    <button
      v-if="group.channels.length > 20"
      class="mt-2 text-primary hover:underline"
      @click="showAllChannels(group.category)"
    >
      展开更多 ({{ group.channels.length }})
    </button>
  </div>
</div>

<!-- Hot Tab -->
<div v-if="activeTab === 'hot'">
  <div v-if="doubanStore.loading" class="flex justify-center py-8">
    <LoadingSpinner />
  </div>
  <div v-else class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
    <VodCard
      v-for="item in matchedHotItems"
      :key="item.id"
      :item="item"
      @click="handleVodClick"
    />
  </div>
</div>

<!-- Movie/Tv/Variety/Anime Tabs -->
<div v-if="['movie', 'tv', 'variety', 'anime'].includes(activeTab)">
  <SearchBar :placeholder="`搜索${getTabLabel(activeTab)}...`" @search="handleVodSearch" />
  <div class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
    <VodCard
      v-for="item in vodStore.items.slice(0, 20)"
      :key="item.id"
      :item="item"
      @click="handleVodClick"
    />
  </div>
  <button
    v-if="vodStore.items.length > 20"
    class="mt-4 w-full py-2 bg-gray-700 rounded hover:bg-gray-600"
    @click="showAllVod"
  >
    加载更多
  </button>
</div>
```

---

### Phase 5: Player Multi-Source Support

#### Task 13: Update PlayerPage for Source Switching

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Add source switching state**

```typescript
const sources = ref<ChannelSource[]>([])
const currentSourceIndex = ref(0)

const currentSource = computed(() => sources.value[currentSourceIndex.value])

function switchToSource(index: number) {
  if (index >= 0 && index < sources.value.length) {
    currentSourceIndex.value = index
    playSource(sources.value[index].url)
  }
}

async function playSource(url: string) {
  if (isDrpyProtocol(url)) {
    await openExternal(url)
    return
  }
  initHlsPlayer(url)
}
```

- [ ] **Step 2: Add source selector UI**

```vue
<!-- In control bar -->
<div class="source-selector flex items-center gap-2">
  <span class="text-sm">{{ currentSourceIndex + 1 }}/{{ sources.length }}</span>
  <button
    v-for="(_, i) in sources"
    :key="i"
    :class="['px-2 py-1 text-xs rounded', i === currentSourceIndex ? 'bg-primary' : 'bg-gray-700']"
    @click="switchToSource(i)"
  >
    源{{ i + 1 }}
  </button>
</div>
```

- [ ] **Step 3: Auto-switch on error**

```typescript
// In initHlsPlayer error handler
hls.on(Hls.Events.ERROR, (_event, data) => {
  if (data.fatal) {
    console.error('HLS fatal error:', data)
    // Try next source
    if (currentSourceIndex.value < sources.value.length - 1) {
      switchToSource(currentSourceIndex.value + 1)
    } else {
      errorMsg.value = '所有源均不可用'
    }
  }
})
```

---

## Verification Checklist

After implementation, verify:

- [ ] App builds without errors: `cargo build --release` and `npm run build`
- [ ] SQLite has `douban_hot` table
- [ ] Douban crawler fetches and stores data
- [ ] Live channels are grouped by category with correct ordering
- [ ] Multi-source channels show all sources
- [ ] Player switches sources on error
- [ ] 6 tabs display correctly
- [ ] Search works across tabs
- [ ] No console errors

---

## Test Commands

```bash
# Build Rust
cd src-tauri && cargo build 2>&1 | head -50

# Build Frontend
npm run build 2>&1 | tail -30

# Check SQLite
sqlite3 ~/Library/Application\ Support/com.tvbox.app/tvbox.db ".schema douban_hot"
sqlite3 ~/Library/Application\ Support/com.tvbox.app/tvbox.db "SELECT COUNT(*) FROM douban_hot"
sqlite3 ~/Library/Application\ Support/com.tvbox.app/tvbox.db "SELECT DISTINCT category FROM live_channels ORDER BY category"
```
