# 详情页重新设计实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现新的详情页，上半部分展示豆瓣风格元数据（导演/编剧/主演/类型等），下半部分列出所有线路及可播放剧集。

**Architecture:** Rust 端新增 WebView-based Douban subject 元数据抓取命令；前端新增 DoubanMetaPanel 组件 + DetailMetaSkeleton 骨架屏；修改 VodDetail.vue 布局为全线路展开模式。

**Tech Stack:** Vue 3 + TypeScript (前端), Rust + Tauri 2 + WKWebView (后端)

---

## 文件结构

```
src/
  views/VodDetail.vue              # 修改: 新布局
  stores/detail.ts                 # 修改: 新增 doubanMeta state
  components/detail/
    DoubanMetaPanel.vue           # 新增: 豆瓣风格元信息
    DetailMetaSkeleton.vue        # 新增: 加载骨架屏
    EpisodeGroupPanel.vue         # 修改: 始终展开，移除推荐逻辑
    RecommendedSourcePanel.vue    # 删除
    DetailHero.vue               # 删除
    EpisodeGroupSkeleton.vue      # 保留（detailStore.loading 时显示）

src-tauri/
  src/
    models/mod.rs                 # 修改: 新增 DoubanSubjectMeta
    commands/douban.rs           # 修改: 新增 fetch_douban_subject_metadata
    services/douban.rs           # 修改: 新增 WebView 抓取逻辑
    services/storage.rs           # 修改: 数据库迁移添加 douban_id 字段
  src/main.rs                     # 修改: 注册新命令
  capabilities/main.json           # 修改: 添加 webview 权限
```

---

## Task 1: 数据库迁移 — 添加 douban_id 字段

**Files:**
- Modify: `src-tauri/src/services/storage.rs` (migrate_if_needed 方法 + init_tables 的 catalog_items 表定义)

- [ ] **Step 1: 添加数据库迁移**

在 `migrate_if_needed` 方法末尾添加检查和 ALTER TABLE:

```rust
fn migrate_if_needed(&self) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    // ... 现有迁移检查 ...

    // 迁移: 检查 catalog_items 是否有 douban_id 字段
    let mut stmt = conn.prepare(
        "SELECT COUNT(*) FROM pragma_table_info('catalog_items') WHERE name='douban_id'"
    )?;
    let has_column: bool = stmt.query_row([], |row| row.get::<_, i32>(0))? > 0;

    if !has_column {
        conn.execute(
            "ALTER TABLE catalog_items ADD COLUMN douban_id INTEGER REFERENCES douban_hot(id)",
            [],
        )?;
        log::info!("Migrated catalog_items: added douban_id column");
    }
    Ok(())
}
```

- [ ] **Step 2: 更新 init_tables 中的 catalog_items 建表语句（仅注释说明）**

在 catalog_items 建表语句的 CREATE TABLE IF NOT EXISTS 块后添加注释说明正式建表时需包含 douban_id INTEGER 字段。由于使用 ALTER TABLE 加列，现有 SQLite 可以正常工作。

- [ ] **Step 3: 添加 storage 方法用于更新和查询 douban_id**

在 `Storage` impl 块中添加两个方法：

```rust
/// 更新 catalog_item 的 douban_id
pub fn update_catalog_douban_id(&self, catalog_id: i64, douban_id: i64) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    conn.execute(
        "UPDATE catalog_items SET douban_id = ?1 WHERE id = ?2",
        rusqlite::params![douban_id, catalog_id],
    )?;
    Ok(())
}

/// 根据 title + year 在 douban_hot 中模糊匹配，返回 douban_id
pub fn find_douban_id_by_title(&self, title: &str, year: Option<i32>) -> SqliteResult<Option<i64>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT id, name, year FROM douban_hot LIMIT 500"
    )?;
    let mut best_match: Option<i64> = None;
    let mut best_score = 0.8f64;

    let normalized = normalize_for_match(title);
    let mut rows = stmt.query([])?;

    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let name: String = row.get(1)?;
        let douban_year: Option<i32> = row.get(2)?;

        let score = calculate_match_score(&normalized, &normalize_for_match(&name));
        if score > best_score {
            if let (Some(dy), Some(vy)) = (douban_year, year) {
                if (dy - vy).abs() > 1 {
                    continue;
                }
            }
            best_score = score;
            best_match = Some(id);
        }
    }

    Ok(best_match)
}

fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
}

fn calculate_match_score(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    if a_chars.is_empty() || b_chars.is_empty() { return 0.0; }
    let a_ngrams: std::collections::HashSet<String> = (0..a_chars.len())
        .filter_map(|i| if i + 2 <= a_chars.len() { Some(a_chars[i..i+2].iter().collect()) } else { None })
        .collect();
    let b_ngrams: std::collections::HashSet<String> = (0..b_chars.len())
        .filter_map(|i| if i + 2 <= b_chars.len() { Some(b_chars[i..i+2].iter().collect()) } else { None })
        .collect();
    let intersection = a_ngrams.intersection(&b_ngrams).count();
    let union = a_ngrams.union(&b_ngrams).count();
    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}
```

- [ ] **Step 4: 验证编译**

Run: `cd src-tauri && cargo check 2>&1 | tail -20`
Expected: 编译成功，无错误

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "feat(storage): add douban_id column migration and lookup"
```

---

## Task 2: Rust Model — 新增 DoubanSubjectMeta

**Files:**
- Modify: `src-tauri/src/models/mod.rs`

- [ ] **Step 1: 添加 DoubanSubjectMeta 结构体**

在 `mod.rs` 文件末尾 (在最后一个 struct 之后) 添加:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoubanSubjectMeta {
    pub douban_id: i64,
    pub title: String,
    pub rating: Option<f64>,
    pub rating_count: Option<i64>,
    pub director: Vec<String>,
    pub writer: Vec<String>,
    pub actors: Vec<String>,
    pub genre: Vec<String>,
    pub country: Vec<String>,
    pub language: Vec<String>,
    pub release_date: Vec<String>,
    pub runtime: Option<String>,
    pub summary: Option<String>,
    pub poster: Option<String>,
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check 2>&1 | tail -10`
Expected: 编译成功

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/models/mod.rs
git commit -m "feat(models): add DoubanSubjectMeta struct"
```

---

## Task 3: Rust — WebView Douban 抓取命令

**Files:**
- Modify: `src-tauri/src/services/douban.rs`
- Modify: `src-tauri/src/commands/douban.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/capabilities/main.json`

- [ ] **Step 1: 添加 WebView 权限到 capabilities/main.json**

```json
{
  "$schema": "https://schema.tauri.app/config/2/capability",
  "identifier": "main-capability",
  "description": "Main window capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:window:allow-is-fullscreen",
    "core:window:allow-set-fullscreen",
    "core:webview:allow-create-webview-window",
    "core:webview:allow-webview-close"
  ]
}
```

- [ ] **Step 2: 在 services/douban.rs 添加 WebView 抓取函数**

在 `DoubanCrawler` impl 末尾添加:

```rust
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};
use scraper::{Html, Selector};

pub struct DoubanSubjectScraper;

impl DoubanSubjectScraper {
    /// 使用 WebView 加载 Douban subject 页面并提取元数据
    pub async fn scrape(app: &AppHandle, douban_id: i64) -> Result<DoubanSubjectMeta, String> {
        let url = format!("https://movie.douban.com/subject/{}/", douban_id);

        // 创建隐藏 webview window
        let webview = WebviewWindowBuilder::new(
            app,
            format!("douban-scrape-{}", douban_id),
            WebviewUrl::Url(url.parse().map_err(|e| format!("Invalid URL: {}", e))?),
        )
        .title("Douban Scraper")
        .inner_size(1280.0, 800.0)
        .visible(false)
        .build()
        .map_err(|e| format!("Failed to create webview: {}", e))?;

        // 等待页面加载 (通过 poll 方式，最长 10 秒)
        let webview_clone = webview.clone();
        let loaded = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if webview_clone.eval("document.readyState").is_ok() {
                    break;
                }
            }
        }).await;

        if loaded.is_err() {
            webview.close().ok();
            return Err("Timeout waiting for Douban page".to_string());
        }

        // 额外等待，确保 DOM 完全渲染
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // 执行 JS 提取 #info HTML
        let info_html = webview
            .eval("document.getElementById('info')?.innerHTML ?? ''")
            .map_err(|e| format!("JS eval error: {}", e))?
            .as_str()
            .ok_or("No result from JS eval")?
            .to_string();

        // 执行 JS 提取 summary
        let summary = webview
            .eval("document.querySelector('[property=\"v:summary\"]')?.innerText ?? document.querySelector('#link-report span')?.innerText ?? ''")
            .map_err(|e| format!("JS eval error: {}", e)?
            .as_str()
            .map(|s| s.to_string());

        // 执行 JS 提取 rating
        let rating = webview
            .eval("document.querySelector('.rating_num')?.innerText ?? ''")
            .map_err(|e| format!("JS eval error: {}", e)?
            .as_str()
            .and_then(|s| s.trim().parse::<f64>().ok());

        // 执行 JS 提取 rating count
        let rating_count = webview
            .eval("document.querySelector('.rating_sum span')?.innerText ?? ''")
            .map_err(|e| format!("JS eval error: {}", e)?
            .as_str()
            .and_then(|s| {
                let re = regex::Regex::new(r"(\d+)").ok()?;
                re.captures(s)?.get(1)?.as_str().parse::<i64>().ok()
            });

        // 提取 title
        let title = webview
            .eval("document.querySelector('h1 span[property=\"v:itemreviewed\"]')?.innerText ?? document.querySelector('h1')?.innerText ?? ''")
            .map_err(|e| format!("JS eval error: {}", e)?
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_default();

        webview.close().ok();

        // 解析 HTML
        let document = Html::parse_document(&info_html);

        let director = Self::extract_by_property(&document, "v:directedBy");
        let writer = Self::extract_by_property(&document, "v:writer");
        let actors = Self::extract_by_property_limit(&document, "v:starring", 5);
        let genre = Self::extract_by_property(&document, "v:genre");

        // 从纯文本行中提取其他字段
        let info_text = info_html.replace(|c: char| c == '<' || c == '>', "\n");
        let country = Self::extract_info_field(&info_text, "制片国家/地区:");
        let language = Self::extract_info_field(&info_text, "语言:");
        let release_date = Self::extract_info_field(&info_text, "上映日期:");
        let runtime = Self::extract_info_field(&info_text, "片长:").pop().or(None);

        Ok(DoubanSubjectMeta {
            douban_id,
            title,
            rating,
            rating_count,
            director,
            writer,
            actors,
            genre,
            country,
            language,
            release_date,
            runtime,
            summary,
            poster: None,
        })
    }

    fn extract_by_property(doc: &Html, prop: &str) -> Vec<String> {
        let selector = Selector::parse(&format!("[property=\"{}\"]", prop)).ok();
        selector
            .and_then(|s| doc.select(&s).last().cloned())
            .map(|el| {
                el.text()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extract_by_property_limit(doc: &Html, prop: &str, limit: usize) -> Vec<String> {
        let selector = match Selector::parse(&format!("[property=\"{}\"]", prop)) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        doc.select(&selector)
            .take(limit)
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn extract_info_field(text: &str, key: &str) -> Vec<String> {
        let mut results = vec![];
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.contains(key) {
                // 收集该行及后续行（跨行字段）
                let val = line.replace(key, "").trim().to_string();
                if !val.is_empty() {
                    results.push(val);
                }
                // 尝试合并多行
                for j in (i+1)..lines.len() {
                    let next = lines[j].trim();
                    if next.is_empty() || next.starts_with('<') || next.contains(':') {
                        break;
                    }
                    if !next.is_empty() {
                        results.push(next.to_string());
                    }
                }
            }
        }
        results
    }
}
```

- [ ] **Step 3: 在 commands/douban.rs 添加命令**

在文件末尾 (在最后一个 `#[tauri::command]` 之前) 添加:

```rust
#[tauri::command]
pub async fn fetch_douban_subject_metadata(
    state: State<'_, AppState>,
    item_id: i64,
) -> Result<Option<DoubanSubjectMeta>, String> {
    let title = {
        let conn = state.storage.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT title FROM catalog_items WHERE id = ?1").map_err(|e| e.to_string())?;
        stmt.query_row([item_id], |row| row.get::<_, String>(0)).ok()
    };

    let douban_id = if let Some(ref t) = title {
        // 先查 catalog_items 是否有 douban_id
        let existing: Option<i64> = {
            let conn = state.storage.conn.lock().unwrap();
            conn.query_row(
                "SELECT douban_id FROM catalog_items WHERE id = ?1",
                [item_id],
                |row| row.get("douban_id"),
            ).ok()
        };
        if let Some(id) = existing {
            Some(id)
        } else {
            // 尝试模糊匹配
            state.storage.find_douban_id_by_title(t, None).ok().flatten()
        }
    } else {
        None
    };

    if let Some(dbid) = douban_id {
        let app = state.storage.app_handle.clone();
        let meta = DoubanSubjectScraper::scrape(&app, dbid).await;
        match meta {
            Ok(m) => Ok(Some(m)),
            Err(e) => {
                log::warn!("Failed to fetch Douban meta for {}: {}", dbid, e);
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}
```

**注意**: Tauri 命令可以接收 `app: AppHandle` 作为参数（自动注入），不需要存在 Storage 里。

- [ ] **Step 4: 确认 AppHandle 可以注入到命令**

Tauri 的 `#[tauri::command]` 宏自动支持 `app: AppHandle` 参数注入，无需修改 `AppState` 或 `Storage`。

- [ ] **Step 5: 注册新命令到 main.rs**

在 `generate_handler!` 数组中添加:

```rust
tvbox_lib::commands::douban::fetch_douban_subject_metadata,
```

- [ ] **Step 6: 验证编译**

Run: `cd src-tauri && cargo check 2>&1 | tail -30`
Expected: 编译成功，无错误

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/services/douban.rs src-tauri/src/commands/douban.rs src-tauri/src/main.rs src-tauri/capabilities/main.json
git commit -m "feat(douban): add WebView-based subject metadata scraper"
```

---

## Task 4: 前端 — DoubanMetaPanel 组件

**Files:**
- Create: `src/components/detail/DoubanMetaPanel.vue`

- [ ] **Step 1: 编写 DoubanMetaPanel 组件**

```vue
<script setup lang="ts">
interface DoubanMeta {
  douban_id: number
  title: string
  rating: number | null
  rating_count: number | null
  director: string[]
  writer: string[]
  actors: string[]
  genre: string[]
  country: string[]
  language: string[]
  release_date: string[]
  runtime: string | null
  summary: string | null
  poster: string | null
}

defineProps<{
  meta: DoubanMeta
  poster?: string
}>()

function formatList(items: string[], max: number): string {
  if (items.length <= max) return items.join(' / ')
  return items.slice(0, max).join(' / ') + ' / 更多...'
}
</script>

<template>
  <section class="douban-meta-panel">
    <div class="douban-meta-poster">
      <img v-if="poster" :src="poster" :alt="meta.title" class="poster-img" />
      <img v-else-if="meta.poster" :src="meta.poster" :alt="meta.title" class="poster-img" />
      <div v-else class="poster-fallback">{{ meta.title }}</div>
    </div>

    <div class="douban-meta-content">
      <h1 class="douban-meta-title">{{ meta.title }}</h1>

      <div v-if="meta.rating" class="douban-meta-rating">
        <span class="rating-star">★</span>
        <span class="rating-num">{{ meta.rating.toFixed(1) }}</span>
        <span v-if="meta.rating_count" class="rating-count">
          ({{ (meta.rating_count / 10000).toFixed(1) }}万人评价)
        </span>
      </div>

      <dl class="douban-meta-list">
        <template v-if="meta.director.length">
          <dt>导演</dt>
          <dd>{{ meta.director.join(' / ') }}</dd>
        </template>
        <template v-if="meta.writer.length">
          <dt>编剧</dt>
          <dd>{{ meta.writer.join(' / ') }}</dd>
        </template>
        <template v-if="meta.actors.length">
          <dt>主演</dt>
          <dd>{{ formatList(meta.actors, 5) }}</dd>
        </template>
        <template v-if="meta.genre.length">
          <dt>类型</dt>
          <dd>{{ meta.genre.join(' / ') }}</dd>
        </template>
        <template v-if="meta.country.length">
          <dt>制片国家/地区</dt>
          <dd>{{ meta.country.join(' / ') }}</dd>
        </template>
        <template v-if="meta.language.length">
          <dt>语言</dt>
          <dd>{{ meta.language.join(' / ') }}</dd>
        </template>
        <template v-if="meta.release_date.length">
          <dt>上映日期</dt>
          <dd>{{ meta.release_date.join(' / ') }}</dd>
        </template>
        <template v-if="meta.runtime">
          <dt>片长</dt>
          <dd>{{ meta.runtime }}</dd>
        </template>
      </dl>

      <div v-if="meta.summary" class="douban-meta-summary">
        <p>{{ meta.summary }}</p>
      </div>
    </div>
  </section>
</template>
```

- [ ] **Step 2: 添加 Tailwind 样式（通过现有的 style 系统或全局 CSS）**

确保 `.douban-meta-panel`, `.douban-meta-poster`, `.douban-meta-content`, `.douban-meta-title`, `.douban-meta-rating`, `.douban-meta-list`, `.douban-meta-summary` 等类在 Tailwind 或是组件 scoped style 中有样式。

- [ ] **Step 3: 提交**

```bash
git add src/components/detail/DoubanMetaPanel.vue
git commit -m "feat(DoubanMetaPanel): new Douban-style metadata display component"
```

---

## Task 5: 前端 — DetailMetaSkeleton 骨架屏

**Files:**
- Create: `src/components/detail/DetailMetaSkeleton.vue`

- [ ] **Step 1: 编写骨架屏组件**

```vue
<script setup lang="ts">
// 骨架屏：与 DoubanMetaPanel 布局一致的加载状态
</script>

<template>
  <section class="douban-meta-panel">
    <div class="douban-meta-poster">
      <div class="skeleton-pulse skeleton-poster"></div>
    </div>
    <div class="douban-meta-content">
      <div class="skeleton-pulse skeleton-title"></div>
      <div class="skeleton-pulse skeleton-rating"></div>
      <div class="skeleton-meta-lines">
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
      </div>
      <div class="skeleton-pulse skeleton-summary"></div>
    </div>
  </section>
</template>

<style scoped>
.skeleton-pulse {
  background: linear-gradient(90deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.1) 50%, rgba(255,255,255,0.05) 100%);
  background-size: 200% 100%;
  animation: pulse 1.5s ease-in-out infinite;
  border-radius: 0.5rem;
}
@keyframes pulse {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
.skeleton-poster { width: 180px; height: 260px; }
.skeleton-title { height: 2rem; width: 60%; margin-bottom: 1rem; }
.skeleton-rating { height: 1.5rem; width: 30%; margin-bottom: 1.5rem; }
.skeleton-meta-lines { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.5rem; }
.skeleton-meta-line { height: 1rem; width: 80%; }
.skeleton-summary { height: 4rem; width: 100%; }
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/detail/DetailMetaSkeleton.vue
git commit -m "feat(DetailMetaSkeleton): loading skeleton for Douban metadata zone"
```

---

## Task 6: 前端 — 重构 SourceGroupPanel (始终展开)

**Files:**
- Modify: `src/components/detail/EpisodeGroupPanel.vue`

- [ ] **Step 1: 修改 EpisodeGroupPanel 为始终展开，移除推荐逻辑**

将组件重命名为语义化的名称，但保持文件名为 `EpisodeGroupPanel.vue`（避免大量路由/引用变更）：

```vue
<script setup lang="ts">
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisode, CatalogEpisodeGroup } from '@/types'

defineProps<{
  group: CatalogEpisodeGroup
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()
</script>

<template>
  <section class="source-group-panel">
    <div class="source-group-header">
      <div class="source-group-title-row">
        <span class="section-title">{{ group.source_name }}</span>
        <SourceBadge :label="`${group.episodes.length} 个播放源`" tone="neutral" />
      </div>
    </div>

    <div class="episode-chip-grid">
      <EpisodeChip
        v-for="episode in group.episodes"
        :key="episode.id"
        :label="episode.episode_label"
        state="playable"
        @click="emit('play', episode)"
      />
    </div>
  </section>
</template>
```

移除原有的 `expanded` ref、watch、折叠按钮逻辑。CSS 保留 `.source-group-panel`, `.source-group-header`, `.source-group-title-row`, `.episode-chip-grid` 的样式（不需要折叠/展开态）。

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `cd /Users/dustin/Workspace/tvbox && npm run build 2>&1 | grep -E '(error|warning|VodDetail|DetailHero|RecommendedSource)' | head -20`
Expected: 无相关错误

- [ ] **Step 3: 提交**

```bash
git add src/components/detail/EpisodeGroupPanel.vue
git commit -m "refactor(EpisodeGroupPanel): always expanded, remove collapse logic"
```

---

## Task 7: 前端 — 更新 VodDetail.vue 布局

**Files:**
- Modify: `src/views/VodDetail.vue`

- [ ] **Step 1: 更新 VodDetail.vue 模板和脚本**

```vue
<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import DoubanMetaPanel from '@/components/detail/DoubanMetaPanel.vue'
import DetailMetaSkeleton from '@/components/detail/DetailMetaSkeleton.vue'
import EpisodeGroupPanel from '@/components/detail/EpisodeGroupPanel.vue'
import EpisodeGroupSkeleton from '@/components/detail/EpisodeGroupSkeleton.vue'
import type { CatalogEpisode } from '@/types'
import type { CatalogDetailItem, CatalogEpisodeGroup } from '@/types'

// DoubanSubjectMeta from Rust command (camelCase to match Rust #[serde(rename_all = "camelCase")])
interface DoubanSubjectMeta {
  doubanId: number
  title: string
  rating: number | null
  ratingCount: number | null
  director: string[]
  writer: string[]
  actors: string[]
  genre: string[]
  country: string[]
  language: string[]
  releaseDate: string[]
  runtime: string | null
  summary: string | null
  poster: string | null
}

const route = useRoute()
const router = useRouter()
const detailStore = useDetailStore()

const itemId = computed(() => Number(route.params.itemId))

const doubanMeta = ref<DoubanSubjectMeta | null>(null)
const loadingDouban = ref(false)

const backdropStyle = computed(() => {
  const poster = detailStore.item?.poster
  if (!poster) return undefined
  return {
    backgroundImage: `linear-gradient(90deg, rgba(7, 10, 15, 0.96), rgba(7, 10, 15, 0.78) 45%, rgba(7, 10, 15, 0.92)), url(${poster})`,
    backgroundSize: 'cover',
    backgroundPosition: 'center'
  }
})

async function loadDetail() {
  if (!Number.isFinite(itemId.value) || itemId.value <= 0) {
    return
  }

  await detailStore.fetchDetail(itemId.value)

  // 同时获取 Douban 元数据
  loadingDouban.value = true
  try {
    const meta = await invoke<DoubanSubjectMeta | null>('fetch_douban_subject_metadata', {
      itemId: itemId.value,
    })
    doubanMeta.value = meta
  } catch {
    doubanMeta.value = null
  } finally {
    loadingDouban.value = false
  }
}

onMounted(loadDetail)
watch(itemId, loadDetail)

function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`)
}
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1400px]">
      <button class="action-button action-button-secondary" type="button" @click="router.back()">
        返回片库
      </button>

      <!-- 加载状态: 无 item -->
      <div v-if="detailStore.loading && !detailStore.item" class="surface-panel mt-6 flex min-h-[420px] items-center justify-center rounded-[2.4rem]">
        <LoadingSpinner size="lg" />
      </div>

      <div v-else-if="detailStore.item" class="mt-6 space-y-6">
        <!-- 顶部: Douban 元信息 (有数据时) -->
        <DoubanMetaPanel
          v-if="doubanMeta"
          :meta="doubanMeta"
          :poster="detailStore.item.poster"
          class="top-zone"
        />

        <!-- 顶部: Douban 元信息骨架屏 (加载中) -->
        <DetailMetaSkeleton v-else-if="loadingDouban" class="top-zone" />

        <!-- 底部: 全线路列表 -->
        <section v-if="detailStore.loading && detailStore.item" class="space-y-4">
          <EpisodeGroupSkeleton :count="8" />
        </section>

        <section v-else-if="detailStore.episodeGroups.length" class="source-list space-y-4">
          <EpisodeGroupPanel
            v-for="group in detailStore.episodeGroups"
            :key="group.source_name"
            :group="group"
            @play="handlePlay"
          />
        </section>

        <div v-else-if="detailStore.item" class="home-empty-state">
          当前内容没有可展示的播放入口。
        </div>
      </div>

      <div v-else class="surface-panel mt-6 flex min-h-[320px] items-center justify-center rounded-[2rem] text-sm text-white/45">
        没有找到内容详情。
      </div>
    </div>
  </div>
</template>
```

**注意**: 需要添加 `ref` 的导入 (`import { ref } from 'vue'`).

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `npm run build 2>&1 | grep -E 'error|failed' | head -20`
Expected: 无相关错误

- [ ] **Step 3: 提交**

```bash
git add src/views/VodDetail.vue
git commit -m "feat(VodDetail): new layout with DoubanMetaPanel and all sources expanded"
```

---

## Task 8: 前端 — 删除废弃组件

**Files:**
- Delete: `src/components/detail/RecommendedSourcePanel.vue`
- Delete: `src/components/detail/DetailHero.vue`

**注意**: detail.ts store 不需要修改 — DoubanMeta 在 VodDetail.vue 组件内部通过本地 `ref` + `invoke` 获取。

- [ ] **Step 1: 删除文件**

```bash
rm src/components/detail/RecommendedSourcePanel.vue
rm src/components/detail/DetailHero.vue
```

- [ ] **Step 2: 验证无引用残留**

Run: `grep -r "RecommendedSourcePanel\|DetailHero" src/ --include="*.vue" --include="*.ts"`
Expected: 无结果（组件已无引用）

- [ ] **Step 3: 提交**

```bash
git rm src/components/detail/RecommendedSourcePanel.vue src/components/detail/DetailHero.vue
git commit -m "chore: remove deprecated DetailHero and RecommendedSourcePanel"
```

---

## Task 9: 验证与收尾

**Files:**
- Full `src-tauri` build
- Full `npm run build`

- [ ] **Step 1: Rust 完整编译**

Run: `cd src-tauri && cargo build 2>&1 | tail -20`
Expected: 编译成功

- [ ] **Step 2: 前端完整构建**

Run: `npm run build 2>&1 | tail -20`
Expected: 构建成功

- [ ] **Step 3: 最终提交**

```bash
git add -A
git commit -m "feat: new detail page with Douban metadata and all sources expanded"
```

---

## Spec 覆盖检查

| Spec 需求 | 对应任务 |
|-----------|---------|
| 上半部分 Douban 风格元数据展示 | Task 4 (Rust WebView) + Task 5 (DoubanMetaPanel) + Task 7 (VodDetail.vue) |
| 导演/编剧/主演/类型/制片国家/语言/上映日期/片长/评分/简介 | Task 4 (DoubanSubjectMeta 字段) |
| 下半部分全线路列表 | Task 7 (VodDetail.vue) + Task 6 (重构 EpisodeGroupPanel) |
| 线路旁显示结果数量 | Task 6 (SourceBadge 显示 episode count) |
| 点击链接直接播放 | Task 7 (handlePlay → /player/vod/) |
| douban_id 关联 | Task 1 (数据库迁移) |
| 骨架屏 | Task 5 (DetailMetaSkeleton) |
| WebView PoW 解决 | Task 3 (WebView 抓取) |

## 类型一致性检查

- `DoubanSubjectMeta` (Rust) ↔ `interface DoubanSubjectMeta` (TypeScript): 字段名一致 (snake_case Rust → camelCase TypeScript via serde rename)
- `CatalogEpisode.play_url` ↔ `handlePlay` router.push: 一致
- `CatalogEpisodeGroup.source_name` ↔ `EpisodeGroupPanel.group.source_name`: 一致
