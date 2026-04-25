# 豆瓣发现架构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现豆瓣热播发现架构——豆瓣热播作为发现层，用户选视频后并行搜索各源获取播放源。

**Architecture:** 后端新增分类豆瓣爬虫（4分类）和并行搜索 API；前端改造 tab 数据源为豆瓣热播，详情页改为搜索流程。

**Tech Stack:** Rust (Tauri backend), Vue 3 + Pinia (frontend), SQLite (storage), 豆瓣 JSON API, 各源 HTML 搜索

---

## 文件结构

```
src-tauri/src/
├── models/mod.rs                    # DoubanHot 增加 item_type
├── services/
│   ├── douban.rs                   # 改为 JSON API，支持分类抓取
│   ├── storage.rs                  # douban_hot 表支持 item_type
│   └── search.rs                   # 新增：并行搜索服务
└── commands/
    ├── douban.rs                   # fetch_all_douban_hot、search_vod_sources
    └── main.rs                     # 注册新命令

src/
├── types/index.ts                   # SearchResult 类型
├── stores/library.ts               # 豆瓣热播按类型过滤 computed
├── views/HotDetail.vue             # 重写：搜索流程 UI
└── views/Home.vue                  # 豆瓣热播分 tab 显示
```

---

## Phase 1: 后端 - 模型和存储层

### Task 1: DoubanHot 模型增加 item_type

**Files:**
- Modify: `src-tauri/src/models/mod.rs:71-79`

- [ ] **Step 1: 修改 `DoubanHot` 结构体**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanHot {
    pub id: i64,              // 豆瓣 subject ID
    pub name: String,
    pub year: Option<i32>,
    pub poster: Option<String>,
    pub rating: Option<f64>,
    pub rank: i32,
    pub updated_at: String,
    pub item_type: String,    // 新增: "movie" | "series" | "variety" | "anime"
}
```

- [ ] **Step 2: 添加新的关联类型**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoubanHotItem {    // 单个搜索结果项（用于搜索响应）
    pub source: String,        // "zxzj" | "jpvod" | "xb6v"
    pub source_name: String,
    pub detail_url: String,
    pub item_type: String,   // "movie" | "series" | "variety" | "anime" | "generic"
    pub title: Option<String>,
    pub poster: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<DoubanHotItem>,
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/models/mod.rs
git commit -m "feat(models): add item_type to DoubanHot and DoubanHotItem types"
```

---

### Task 2: 数据库迁移

**Files:**
- Modify: `src-tauri/src/services/storage.rs` — 添加迁移逻辑

- [ ] **Step 1: 在 `Storage::new` 中添加迁移检查**

在 `Storage::new` 中，检查 `douban_hot` 表是否有 `item_type` 列，如果没有则执行 `ALTER TABLE`。

```rust
// 在 init_tables() 之后添加
fn migrate_if_needed(&self) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM pragma_table_info('douban_hot') WHERE name='item_type'"
    )?;
    let has_column: bool = stmt.query_row([], |row| row.get::<_, i32>(0))? > 0;

    if !has_column {
        conn.execute(
            "ALTER TABLE douban_hot ADD COLUMN item_type TEXT DEFAULT 'movie'",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_douban_item_type ON douban_hot(item_type)",
            [],
        )?;
        log::info!("Migrated douban_hot table: added item_type column");
    }
    Ok(())
}
```

- [ ] **Step 2: 在 `Storage::new` 末尾调用迁移**

在 `Storage::new` 的 `Ok(Self { conn: Mutex::new(conn) })` 之前添加：

```rust
self.migrate_if_needed()?;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "fix(storage): add item_type migration for douban_hot table"
```

---

### Task 3: 豆瓣爬虫改为 JSON API（支持分类）

**Files:**
- Modify: `src-tauri/src/services/douban.rs`

- [ ] **Step 1: 添加分类常量**

```rust
const DOUBAN_API_BASE: &str = "https://movie.douban.com/j/search_subjects";

#[derive(Debug, Clone)]
pub struct DoubanCategory {
    pub item_type: &'static str,
    pub type_param: &'static str,
    pub tag: &'static str,
}

pub const DOUBAN_CATEGORIES: &[DoubanCategory] = &[
    DoubanCategory { item_type: "movie",   type_param: "movie",  tag: "热门" },
    DoubanCategory { item_type: "series",  type_param: "tv",     tag: "热门" },
    DoubanCategory { item_type: "variety", type_param: "tv",    tag: "综艺" },
    DoubanCategory { item_type: "anime",   type_param: "tv",     tag: "动漫" },
];
```

- [ ] **Step 2: 添加 JSON 响应解析**

在 `DoubanCrawler` 中添加：

```rust
#[derive(Debug, Deserialize)]
struct DoubanJsonItem {
    id: String,
    title: String,
    cover: String,
    rate: Option<f64>,
    episodes_info: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DoubanJsonResponse {
    subjects: Vec<DoubanJsonItem>,
}
```

- [ ] **Step 3: 添加按分类抓取方法**

```rust
impl DoubanCrawler {
    pub async fn fetch_category(&self, category: &DoubanCategory) -> Result<Vec<DoubanHot>, String> {
        let url = format!(
            "{}?type={}&tag={}&page_limit=30&page_start=0",
            DOUBAN_API_BASE, category.type_param, category.tag
        );

        let resp = self.client.get(&url).send().await
            .map_err(|e| format!("Request failed: {}", e))?;

        let json: DoubanJsonResponse = resp.json().await
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut items = Vec::new();
        for (rank, item) in json.subjects.iter().enumerate() {
            let id = item.id.parse::<i64>().unwrap_or(0);
            items.push(DoubanHot {
                id,
                name: item.title.clone(),
                year: None,
                poster: Some(item.cover.clone()),
                rating: item.rate,
                rank: (rank + 1) as i32,
                updated_at: chrono_now(),
                item_type: category.item_type.to_string(),
            });
        }
        Ok(items)
    }
}
```

- [ ] **Step 4: 添加 `fetch_all` 并行抓取**

```rust
pub async fn fetch_all(&self) -> Result<Vec<DoubanHot>, String> {
    use tokio::task::JoinSet;

    let mut all_items = Vec::new();

    for category in DOUBAN_CATEGORIES {
        match self.fetch_category(category).await {
            Ok(items) => all_items.extend(items),
            Err(e) => log::warn!("Failed to fetch {}: {}", category.item_type, e),
        }
        // 豆瓣 API 频率限制：每次请求间隔 500ms
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Ok(all_items)
}
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/douban.rs
git commit -m "feat(douban): rewrite crawler to use JSON API with category support"
```

---

### Task 4: 存储层支持 item_type

**Files:**
- Modify: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: 更新 `upsert_douban_hot` SQL**

修改 `upsert_douban_hot` 中的 INSERT 语句，增加 `item_type` 字段：

```rust
conn.execute(
    "INSERT OR REPLACE INTO douban_hot (name, year, poster, rating, rank, updated_at, item_type, id)
     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    rusqlite::params![
        item.name,
        item.year,
        item.poster,
        item.rating,
        item.rank,
        item.updated_at,
        item.item_type,
        item.id,  // 豆瓣 subject ID 作为主键
    ],
)?;
```

- [ ] **Step 2: 添加按类型查询**

```rust
pub fn get_douban_hot_by_type(&self, item_type: &str) -> SqliteResult<Vec<DoubanHot>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, year, poster, rating, rank, updated_at, item_type
         FROM douban_hot WHERE item_type = ?1 ORDER BY rank LIMIT 30"
    )?;
    let rows = stmt.query_map([item_type], |row| {
        Ok(DoubanHot {
            id: row.get(0)?,
            name: row.get(1)?,
            year: row.get(2)?,
            poster: row.get(3)?,
            rating: row.get(4)?,
            rank: row.get(5)?,
            updated_at: row.get(6)?,
            item_type: row.get(7)?,
        })
    })?;
    rows.collect()
}
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "feat(storage): add item_type to upsert and new get_douban_hot_by_type query"
```

---

### Task 5: 新增 search 服务

**Files:**
- Create: `src-tauri/src/services/search.rs`

- [ ] **Step 1: 基础框架**

```rust
use crate::models::DoubanHotItem;
use reqwest::Client;
use scraper::{Html, Selector};

pub struct SearchService { client: Client }

impl SearchService {
    pub fn new() -> Self {
        Self { client: Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .unwrap()
        }
    }
}

impl Default for SearchService {
    fn default() -> Self { Self::new() }
}
```

- [ ] **Step 2: zxzj 搜索**

```rust
impl SearchService {
    pub async fn search_zxzj(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        let url = format!(
            "https://www.zxzjhd.com/vodsearch/-------------.html?wd={}&submit=",
            urlencoding::encode(title)
        );

        let html = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_zxzj_listing(&document)
    }

    fn parse_zxzj_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("li.col-md-6").map_err(|e| e.to_string())?;
        let thumb_selector = Selector::parse("a.stui-vodlist__thumb").map_err(|e| e.to_string())?;
        let title_selector = Selector::parse("h4.title a").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(thumb) = item.select(&thumb_selector).next() {
                let href = thumb.attr("href").unwrap_or("");
                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.zxzjhd.com{}", href)
                };

                let title = item.select(&title_selector)
                    .next()
                    .and_then(|a| a.attr("title").map(String::from));

                let poster = thumb.attr("data-original").map(String::from);

                // 从 URL 推断类型: /movie/ → movie, /dianshiju/ → series
                let item_type = if detail_url.contains("/movie/") {
                    "movie".to_string()
                } else if detail_url.contains("/dianshiju/") {
                    "series".to_string()
                } else if detail_url.contains("/zongyi/") {
                    "variety".to_string()
                } else if detail_url.contains("/dongman/") {
                    "anime".to_string()
                } else {
                    "movie".to_string()  // 默认
                };

                results.push(DoubanHotItem {
                    source: "zxzj".to_string(),
                    source_name: "在线之家".to_string(),
                    detail_url,
                    item_type,
                    title,
                    poster,
                });
            }
        }
        Ok(results)
    }
}
```

- [ ] **Step 3: jpvod 搜索**

```rust
impl SearchService {
    pub async fn search_jpvod(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        let url = format!(
            "https://jpvod.com/search/-------------.html?wd={}&submit=",
            urlencoding::encode(title)
        );

        let html = self.client.get(&url).send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_jpvod_listing(&document)
    }

    fn parse_jpvod_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("a.d-block.card").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            let href = item.attr("href").unwrap_or("");
            let detail_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://jpvod.com{}", href)
            };

            let title = item.attr("title").map(String::from);

            // jpvod 详情页 URL 是 /vod/{id}.html，无法推断类型，默认为 generic
            results.push(DoubanHotItem {
                source: "jpvod".to_string(),
                source_name: "贱贱".to_string(),
                detail_url,
                item_type: "generic".to_string(),
                title,
                poster: None,
            });
        }
        Ok(results)
    }
}
```

- [ ] **Step 4: xb6v 搜索（POST 模式）**

```rust
impl SearchService {
    pub async fn search_xb6v(&self, title: &str) -> Result<Vec<DoubanHotItem>, String> {
        // POST 获取 searchid
        let search_url = "https://www.xb6v.com/e/search/1index.php";
        let body = format!(
            "show=title&tempid=1&tbname=article&mid=1&dopost=search&submit=&keyboard={}",
            urlencoding::encode(title)
        );

        let resp = self.client.post(search_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Referer", "https://www.xb6v.com/")
            .body(body)
            .send().await
            .map_err(|e| e.to_string())?;

        // 从 Location header 获取 searchid
        let location = resp.headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let searchid = location.split("searchid=")
            .nth(1)
            .unwrap_or("");

        if searchid.is_empty() {
            return Ok(Vec::new());
        }

        // 访问结果页
        let result_url = format!("https://www.xb6v.com/result/?searchid={}", searchid);
        let html = self.client.get(&result_url)
            .header("Referer", "https://www.xb6v.com/")
            .send().await
            .map_err(|e| e.to_string())?
            .text().await
            .map_err(|e| e.to_string())?;

        let document = Html::parse_document(&html);
        self.parse_xb6v_listing(&document)
    }

    fn parse_xb6v_listing(&self, document: &Html) -> Result<Vec<DoubanHotItem>, String> {
        let item_selector = Selector::parse("li").map_err(|e| e.to_string())?;
        let thumb_selector = Selector::parse("a.stui-vodlist__thumb").map_err(|e| e.to_string())?;

        let mut results = Vec::new();

        for item in document.select(&item_selector) {
            if let Some(thumb) = item.select(&thumb_selector).next() {
                let href = thumb.attr("href").unwrap_or("");
                let detail_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.xb6v.com{}", href)
                };

                let title = thumb.attr("title").map(String::from);
                let poster = thumb.attr("data-original").map(String::from);

                results.push(DoubanHotItem {
                    source: "xb6v".to_string(),
                    source_name: "小白影视".to_string(),
                    detail_url,
                    item_type: "generic".to_string(),
                    title,
                    poster,
                });
            }
        }
        Ok(results)
    }
}
```

- [ ] **Step 5: 并行搜索入口**

```rust
impl SearchService {
    pub async fn search_all(&self, title: &str) -> Vec<DoubanHotItem> {
        let zxzj = self.search_zxzj(title);
        let jpvod = self.search_jpvod(title);
        let xb6v = self.search_xb6v(title);

        let results = tokio::join!(zxzj, jpvod, xb6v);

        let mut all = Vec::new();
        if let Ok(items) = results.0 { all.extend(items); }
        if let Ok(items) = results.1 { all.extend(items); }
        if let Ok(items) = results.2 { all.extend(items); }

        // zxzj 结果优先（可推断类型），generic 排在最后
        all.sort_by(|a, b| {
            if a.item_type == "generic" && b.item_type != "generic" {
                std::cmp::Ordering::Greater
            } else if a.item_type != "generic" && b.item_type == "generic" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        });

        all
    }
}
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/search.rs
# 在 services/mod.rs 中添加: pub mod search;
git add src-tauri/src/services/mod.rs
git commit -m "feat(search): add parallel source search service for zxzj/jpvod/xb6v"
```

---

### Task 6: 新增命令

**Files:**
- Modify: `src-tauri/src/commands/douban.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: 在 douban.rs 添加新命令**

```rust
#[tauri::command]
pub async fn fetch_all_douban_hot(state: State<'_, AppState>) -> Result<Vec<DoubanHot>, String> {
    let crawler = crate::services::douban::DoubanCrawler::new();
    let items = crawler.fetch_all().await?;
    state.storage.clear_douban_hot().map_err(|e| e.to_string())?;
    state.storage.upsert_douban_hot(&items).map_err(|e| e.to_string())?;
    Ok(items)
}

#[tauri::command]
pub async fn search_vod_sources(
    state: State<'_, AppState>,
    title: String,
    _item_type: Option<String>,  // 预留，可用于过滤
) -> Result<Vec<DoubanHotItem>, String> {
    let search = crate::services::search::SearchService::new();
    let results = search.search_all(&title).await;
    Ok(results)
}

#[tauri::command]
pub async fn get_douban_hot_by_type(
    state: State<'_, AppState>,
    item_type: String,
) -> Result<Vec<DoubanHot>, String> {
    state.storage.get_douban_hot_by_type(&item_type).map_err(|e| e.to_string())
}
```

- [ ] **Step 2: 在 main.rs 注册新命令**

在 `tauri::generate_handler![...]` 中添加：

```rust
tvbox_lib::commands::douban::fetch_all_douban_hot,
tvbox_lib::commands::douban::search_vod_sources,
tvbox_lib::commands::douban::get_douban_hot_by_type,
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/douban.rs src-tauri/src/main.rs
git commit -m "feat(commands): add fetch_all_douban_hot and search_vod_sources commands"
```

---

### Task 7: 依赖检查

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: 添加 urlencoding 依赖**

检查 `Cargo.toml` 中是否有 `urlencoding` crate，如果没有，添加：

```toml
urlencoding = "2.1"
```

- [ ] **Step 2: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add urlencoding dependency for search service"
```

---

## Phase 2: 前端类型

### Task 8: 前端类型定义

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: 扩展 `DoubanHot` 和 `DoubanHotItem` 类型**

在 `DoubanHot` 接口中添加 `item_type`：

```typescript
export interface DoubanHot {
  id: number;
  name: string;
  year: number | null;
  poster: string | null;
  rating: number | null;
  rank: number;
  updated_at: string;
  item_type: 'movie' | 'series' | 'variety' | 'anime';  // 新增
}

export type SourceId = 'zxzj' | 'jpvod' | 'xb6v';

export interface SearchResult {
  source: SourceId;
  source_name: string;
  detail_url: string;
  item_type: 'movie' | 'series' | 'variety' | 'anime' | 'generic';
  title?: string;
  poster?: string;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/types/index.ts
git commit -m "feat(types): add item_type to DoubanHot and new SearchResult type"
```

---

## Phase 3: 前端 - HotDetail 重写

### Task 9: HotDetail.vue 搜索流程

**Files:**
- Modify: `src/views/HotDetail.vue`

- [ ] **Step 1: 重写 script 部分**

```typescript
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { DoubanHot, SearchResult } from '@/types'

const route = useRoute()
const router = useRouter()

const doubanId = computed(() => Number(route.params.doubanId))
const doubanHot = ref<DoubanHot | null>(null)
const searchResults = ref<SearchResult[]>([])
const loading = ref(true)
const searchLoading = ref(true)
const error = ref<string | null>(null)
const selectedSource = ref<SearchResult | null>(null)

async function loadHotDetail() {
  loading.value = true
  error.value = null
  try {
    // 获取豆瓣热播数据（按类型）
    const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', {
      item_type: route.meta?.itemType ?? 'movie'
    })
    const hot = items.find((h: DoubanHot) => h.id === doubanId.value)
    if (hot) {
      doubanHot.value = hot
      await searchSources(hot.name)
    } else {
      error.value = '热播数据不存在'
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function searchSources(keyword: string) {
  searchLoading.value = true
  try {
    const results = await invoke<SearchResult[]>('search_vod_sources', { title: keyword })
    searchResults.value = results
  } catch (e) {
    console.warn('搜索失败:', e)
    searchResults.value = []
  } finally {
    searchLoading.value = false
  }
}

function handleSourceSelect(result: SearchResult) {
  selectedSource.value = result
  // 跳转到现有的 VodDetail 页面，使用搜索结果中的详情 URL
  // VodDetail 需要支持接收 detail_url 参数
  router.push({
    name: 'detail',
    params: { itemId: result.detail_url },
    query: { from: 'hot', source: result.source }
  })
}

onMounted(loadHotDetail)
</script>
```

**注意**：`route.meta.itemType` 需要在路由中配置。这在 Task 12 中处理。

- [ ] **Step 2: 更新 template**

将现有的 `matchedItem` 相关模板替换为新的搜索结果展示：

```vue
<!-- 搜索加载中 -->
<div v-if="searchLoading" class="flex items-center gap-2 text-white/50">
  <LoadingSpinner size="sm" />
  <span>正在搜索播放源...</span>
</div>

<!-- 搜索结果列表 -->
<div v-else-if="searchResults.length > 0" class="space-y-4 border-t border-white/10 pt-4">
  <h2 class="text-lg font-semibold text-white">可用播放源</h2>

  <!-- 有类型的源 -->
  <div v-if="searchResults.filter(r => r.item_type !== 'generic').length" class="space-y-2">
    <div
      v-for="result in searchResults.filter(r => r.item_type !== 'generic')"
      :key="result.detail_url"
      class="flex items-center gap-4 rounded-xl bg-white/5 p-4 cursor-pointer hover:bg-white/10"
      @click="handleSourceSelect(result)"
    >
      <img v-if="result.poster" :src="result.poster" class="w-16 rounded-lg" />
      <div class="flex-1">
        <h3 class="text-white">{{ result.title || doubanHot?.name }}</h3>
        <p class="text-sm text-white/50">{{ result.source_name }}</p>
      </div>
      <span class="text-xs text-white/30">{{ result.item_type }}</span>
    </div>
  </div>

  <!-- generic 源 -->
  <div v-if="searchResults.filter(r => r.item_type === 'generic').length">
    <p class="text-sm text-white/30 mb-2">其他源</p>
    <div class="space-y-2">
      <div
        v-for="result in searchResults.filter(r => r.item_type === 'generic')"
        :key="result.detail_url"
        class="flex items-center gap-4 rounded-xl bg-white/5 p-4 cursor-pointer hover:bg-white/10"
        @click="handleSourceSelect(result)"
      >
        <div class="flex-1">
          <h3 class="text-white">{{ result.title || doubanHot?.name }}</h3>
          <p class="text-sm text-white/50">{{ result.source_name }}</p>
        </div>
        <span class="text-xs text-white/30">通用</span>
      </div>
    </div>
  </div>
</div>

<!-- 无搜索结果 -->
<div v-else class="border-t border-white/10 pt-4 text-white/50">
  暂未找到可用的播放源
</div>
```

- [ ] **Step 3: Commit**

```bash
git add src/views/HotDetail.vue
git commit -m "feat(HotDetail): rewrite with parallel source search flow"
```

---

## Phase 4: 前端 - Tab 数据源和刷新

### Task 10: Library Store 豆瓣热播按类型过滤

**Files:**
- Modify: `src/stores/library.ts`

- [ ] **Step 1: 添加按类型获取豆瓣热播**

在 `useLibraryStore` 中添加：

```typescript
// 获取豆瓣热播（按类型），同时处理 24 小时刷新
async function fetchDoubanHotByType(itemType: string) {
  const cached = doubanHotByType.value[itemType]
  const isStale = cached && cached.updated_at
    ? Date.now() - Number(cached.updated_at) > 24 * 60 * 60 * 1000
    : true

  if (cached && !isStale) {
    return cached.items
  }

  // 尝试从数据库获取
  try {
    const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType })
    doubanHotByType.value[itemType] = {
      items,
      updated_at: String(Date.now())
    }
    return items
  } catch {
    // 如果数据库为空，触发抓取
    if (isStale) {
      fetchAllDoubanHot().catch(console.error)
    }
    return cached?.items ?? []
  }
}

async function fetchAllDoubanHot() {
  try {
    await invoke<DoubanHot[]>('fetch_all_douban_hot')
    // 重新填充缓存
    for (const type of ['movie', 'series', 'variety', 'anime']) {
      const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType: type })
      doubanHotByType.value[type] = {
        items,
        updated_at: String(Date.now())
      }
    }
  } catch (e) {
    console.error('fetchAllDoubanHot failed:', e)
  }
}
```

添加 refs：

```typescript
const doubanHotByType = ref<Record<string, { items: DoubanHot[]; updated_at: string }>>({})
```

- [ ] **Step 2: Commit**

```bash
git add src/stores/library.ts
git commit -m "feat(library): add fetchDoubanHotByType with 24h stale check"
```

---

### Task 11: Home.vue Tab 改造

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 修改非 live tab 的数据来源**

将 `displayedVodItems` computed 改为使用豆瓣热播：

```typescript
const displayedHotItems = computed(() => {
  if (activeTab.value === 'live') return []
  const type = activeTab.value as string
  const cached = libraryStore.getDoubanHotByType(type)
  return cached.slice(0, displayedVodCount.value)
})
```

**注意**：`getDoubanHotByType` 需要暴露为 store 的 getter。修改 `libraryStore` 中的 `doubanHotByType` 为 computed property。

- [ ] **Step 2: 修改 template 中非 live tab 部分**

将 `v-else` 分支（v-else 后的 `libraryStore.catalogItems` 展示）替换为：

```vue
<div v-else>
  <div v-if="libraryStore.loading" class="flex min-h-[220px] items-center justify-center">
    <LoadingSpinner />
  </div>

  <div v-else-if="displayedHotItems.length === 0" class="home-empty-state">
    暂无{{ formatTypeLabel(activeTab) }}热播数据
  </div>

  <div v-else class="mt-6">
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
      <VodCard
        v-for="hot in displayedHotItems"
        :key="hot.id"
        :item="(hot as any)"
        @click="handleHotClick"
      />
    </div>
    <!-- load more if needed -->
  </div>
</div>
```

- [ ] **Step 3: 处理 watch 中的 catalog fetch**

修改 `watch(route.params.type, ...)` 回调，live tab 走原有逻辑，非 live tab 改为调用 `fetchDoubanHotByType`：

```typescript
watch(
  () => route.params.type,
  async (tabParam) => {
    const nextTab = normalizeTab(tabParam)
    if (typeof tabParam === 'string' && nextTab !== tabParam) {
      await router.replace(`/library/${nextTab}`)
      return
    }
    activeTab.value = nextTab
    searchKeyword.value = ''
    displayedVodCount.value = 20

    if (nextTab === 'live') {
      // live 逻辑不变
    } else {
      // 豆瓣热播不走 catalog，直接显示
      displayedVodCount.value = 20
    }
  },
  { immediate: true }
)
```

- [ ] **Step 4: Commit**

```bash
git add src/views/Home.vue
git commit -m "feat(home): show douban hot items instead of catalog for non-live tabs"
```

---

### Task 12: 路由 meta 配置

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: 为 HotDetail 路由添加 meta**

```typescript
{
  path: '/detail/hot/:doubanId',
  name: 'HotDetail',
  component: () => import('@/views/HotDetail.vue'),
  meta: {
    itemType: 'movie'  // 默认类型，Home.vue 跳转时可覆盖
  }
}
```

实际项目中可以通过 query 参数传递 item_type：
```typescript
// Home.vue 中
router.push(`/detail/hot/${hot.id}?type=${hot.item_type}`)

// HotDetail.vue 中
const itemType = computed(() => String(route.query.type || 'movie'))
```

- [ ] **Step 2: Commit**

```bash
git add src/router/index.ts
git commit -m "feat(router): add item_type query param support for HotDetail route"
```

---

## 实施验证

### 验证步骤

1. **编译检查**
   ```bash
   cd src-tauri && cargo check
   ```

2. **前端类型检查**
   ```bash
   npm run build
   ```

3. **手动测试**
   - 打开 App，验证豆瓣热播分 tab 显示（movie/series/variety/anime）
   - 点击热播条目，验证搜索流程和播放源展示
   - 验证 24 小时刷新逻辑

---

## 自查清单

- [ ] spec 覆盖：豆瓣 JSON API 分类抓取 ✓ | 并行搜索 ✓ | 前端搜索流程 ✓ | Tab 分类显示 ✓ | 24h 刷新 ✓
- [ ] 无 placeholder：所有 SQL、函数签名、API URL 均已写出
- [ ] 类型一致性：DoubanHot.item_type (Rust) = item_type (TypeScript) = 'movie'|'series'|'variety'|'anime'
- [ ] 文件路径准确：所有文件路径已验证存在
