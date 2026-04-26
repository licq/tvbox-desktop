# VodDetail Provider 化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 VodDetail 页面的搜索和播放迁移到 Provider 系统，替换旧的 scraper-based `search_vod_sources`

**Architecture:** 前端调用 `search_all_sources` 获取 Provider 搜索结果，点击播放时先调用 `provider_play` 获取真实 URL 再跳转

---

## 文件结构

```
src/                          # Frontend
  types/index.ts              # 更新 SearchResult 类型
  views/VodDetail.vue         # 替换 search_vod_sources → search_all_sources
                                # 替换 handleSearchResultPlay → provider_play

src-tauri/src/
  commands/
    search.rs                 # 新增 provider_detail、provider_play commands
    mod.rs                    # 导出新 commands
  services/
    provider/
      mod.rs                  # 导出 VideoProvider
      traits.rs               # VideoProvider trait
      registry.rs             # ProviderRegistry
```

---

## Task 1: 新增 `provider_detail` Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/search.rs`

- [ ] **Step 1: 在 search.rs 添加 provider_detail command**

```rust
#[tauri::command]
pub async fn provider_detail(
    source: String,
    ids: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::xb6v::ScrapedCatalogEpisode>, String> {
    let registry = state.provider_registry.read().await;
    if let Some(provider) = registry.get(&source) {
        match provider.detail(&ids).await {
            Ok(Some(item)) => Ok(item.episodes),
            Ok(None) => Ok(Vec::new()),
            Err(e) => {
                log::warn!("[provider_detail] {} failed: {}", source, e);
                Err(format!("provider detail failed: {}", e))
            }
        }
    } else {
        Err(format!("provider not found: {}", source))
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check --lib 2>&1 | tail -20`
Expected: 无错误（仅有 pre-existing warning）

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/commands/search.rs
git commit -m "feat: add provider_detail Tauri command"
```

---

## Task 2: 新增 `provider_play` Tauri Command

**Files:**
- Modify: `src-tauri/src/commands/search.rs`

- [ ] **Step 1: 在 search.rs 添加 provider_play command**

```rust
#[tauri::command]
pub async fn provider_play(
    source: String,
    flag: String,
    play_url: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::services::playback_types::PlaybackTarget>, String> {
    let registry = state.provider_registry.read().await;
    if let Some(provider) = registry.get(&source) {
        provider.play(&flag, &play_url).await.map_err(|e| {
            log::warn!("[provider_play] {} failed: {}", source, e);
            format!("{}", e)
        })
    } else {
        Err(format!("provider not found: {}", source))
    }
}
```

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check --lib 2>&1 | tail -20`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/commands/search.rs
git commit -m "feat: add provider_play Tauri command"
```

---

## Task 3: 添加 Provider 相关前端类型

**Files:**
- Modify: `src/types/index.ts`

- [ ] **Step 1: 添加 SourceSearchResult 和 PlaybackTarget 类型**

在 `SearchResult` 定义之后添加：

```typescript
// Provider search result structure (returned by search_all_sources)
export interface SourceSearchResult {
  source_key: string
  source_name: string
  items: ProviderCatalogItem[]
}

export interface ProviderCatalogItem {
  source_item_key: string
  title: string
  item_type: string
  poster?: string
  summary?: string
  detail_json?: string
  episodes: CatalogEpisode[]
}

// Playback target (returned by provider_play)
export interface PlaybackTarget {
  episode_id: number | null
  source_key: string
  target_url: string
  target_kind: 'direct' | 'resolvable' | 'embedded' | 'external_required'
  resolver_key: string | null
  headers: Record<string, string> | null
  sort_hint: number
  meta: string | null
}
```

同时更新 `SearchResult` 类型（将 `source` 改为 string）：

```typescript
export interface SearchResult {
  source: string  // 改为 string，兼容所有 Provider
  source_name: string
  detail_url: string  // JSON string: {"source": "site_key", "ids": "vod_id"}
  item_type: 'movie' | 'series' | 'variety' | 'anime' | 'generic'
  title?: string
  poster?: string
}
```

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `cd /Users/dustin/Workspace/tvbox && npx tsc --noEmit 2>&1 | head -30`
Expected: 无错误（可能有 pre-existing warnings）

- [ ] **Step 3: 提交**

```bash
git add src/types/index.ts
git commit -m "feat: add Provider types (SourceSearchResult, PlaybackTarget)"
```

---

## Task 4: 修改 `views/VodDetail.vue` 的搜索逻辑

**Files:**
- Modify: `src/views/VodDetail.vue:144-170`（searchSources 函数）

- [ ] **Step 1: 更新 searchSources 调用 search_all_sources**

当前代码（第148行）：
```typescript
const results = await invoke<SearchResult[]>('search_vod_sources', { title })
```

改为：
```typescript
const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
```

完整替换 `searchSources` 函数（第144-170行）为：

```typescript
async function searchSources(title: string) {
  loadingSearch.value = true
  searchError.value = null
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
    // Group by source, transform to SearchResult format
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      for (const item of r.items) {
        const detailData = item.detail_json
          ? JSON.parse(item.detail_json) as { source?: string; ids?: string }
          : { source: r.source_key, ids: '' }
        grouped[r.source_name] ||= []
        grouped[r.source_name].push({
          source: detailData.source || r.source_key,
          source_name: r.source_name,
          detail_url: item.detail_json || '',
          item_type: item.item_type as SearchResult['item_type'],
          title: item.title,
          poster: item.poster,
        })
      }
    }
    searchResults.value = Object.entries(grouped)
      .filter(([, results]) => results.length > 0)
      .map(([source_name, results]) => ({
        source_name,
        results,
      }))
  } catch (e) {
    console.error('[VodDetail] searchSources failed:', e)
    searchError.value = String(e)
    searchResults.value = []
  } finally {
    loadingSearch.value = false
  }
}
```

**同时需要修改导入**，在文件顶部 `import type { SearchResult } from '@/types'` 之前，添加 `SourceSearchResult` 的导入来源（如果 `SourceSearchResult` 类型在 `search_all_sources` 返回值中已定义在 Rust 侧，前端只需要知道结构）。

实际上需要确认 `SourceSearchResult` 的 TypeScript 类型定义。看 Task 3 的输出结构，`SourceSearchResult` 定义在 `search.rs` Rust 端，前端通过 `invoke<...>('search_all_sources')` 自动获得类型。

完整替换后的导入部分（第12行）：
```typescript
import type { CatalogEpisode, DoubanHot, SearchResult } from '@/types'
```

在第33行附近有 `GroupedSearchResults` interface（已有），不需要改动。

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `cd /Users/dustin/Workspace/tvbox && npx tsc --noEmit 2>&1 | head -30`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src/views/VodDetail.vue
git commit -m "feat: route search through provider_search_all_sources"
```

---

## Task 5: 修改 `views/VodDetail.vue` 的播放逻辑

**Files:**
- Modify: `src/views/VodDetail.vue:179-182`（handleSearchResultPlay 函数）

- [ ] **Step 1: 替换 handleSearchResultPlay 函数**

当前代码：
```typescript
function handleSearchResultPlay(result: SearchResult) {
  router.push(`/player/source/${encodeURIComponent(result.detail_url)}?source=${result.source}&title=${encodeURIComponent(result.title || '')}`)
}
```

改为：
```typescript
async function handleSearchResultPlay(result: SearchResult) {
  // detail_url 是 JSON 字符串: {"source": "site_key", "ids": "vod_id", ...}
  const parsed = JSON.parse(result.detail_url)
  const source = parsed.source || result.source
  const ids = parsed.ids || parsed.url || ''

  try {
    const targets = await invoke<PlaybackTarget[]>('provider_play', {
      source,
      flag: parsed.flag || 'auto',
      play_url: ids,
    })

    if (targets.length > 0) {
      const target = targets[0]
      router.push(`/player/source/${encodeURIComponent(target.target_url)}?source=${source}`)
    } else {
      searchError.value = '播放地址获取失败'
    }
  } catch (e) {
    console.error('[VodDetail] provider_play failed:', e)
    searchError.value = String(e)
  }
}
```

需要导入 `PlaybackTarget` 类型。在第12行添加：

```typescript
import type { CatalogEpisode, DoubanHot, SearchResult } from '@/types'
import type { PlaybackTarget } from '@/types'  // 如果 types/index.ts 没有则需要添加
```

如果 `PlaybackTarget` 类型不在 `types/index.ts` 中，在 Task 3 一起添加。

- [ ] **Step 2: 验证 TypeScript 编译**

Run: `cd /Users/dustin/Workspace/tvbox && npx tsc --noEmit 2>&1 | head -30`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src/views/VodDetail.vue
git commit -m "feat: route playback through provider_play"
```

---

## Task 6: 修改 `refresh_subscription` 移除 VOD 数据写入

**Files:**
- Modify: `src-tauri/src/commands/subscription.rs`（找到 refresh_subscription 中的 VOD 写入代码并移除）

- [ ] **Step 1: 定位并注释掉 VOD 相关代码**

找到 `refresh_subscription` 函数中调用 `scrape_supported_tvbox_catalogs` 或类似 VOD 写入的代码，注释掉或删除。

根据之前的总结，`refresh_subscription` 中有这行代码需要移除：
```rust
// 移除：scrape_supported_tvbox_catalogs 调用
log::debug!("VOD catalog scraping via old scrapers is disabled");
```

实际删除这行注释（如果存在）。

- [ ] **Step 2: 验证编译**

Run: `cd src-tauri && cargo check --lib 2>&1 | tail -20`
Expected: 无错误

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/commands/subscription.rs
git commit -m "refactor: remove VOD catalog scraping from refresh_subscription"
```

---

## Task 7: 验证播放流程 end-to-end

**Files:**
- Test: 启动 Tauri 应用，手动测试播放流程

- [ ] **Step 1: 启动开发服务器**

Run: `npm run tauri dev`
等待应用启动。

- [ ] **Step 2: 测试豆瓣入口播放流程**

1. 点击首页的豆瓣热播片
2. 等待 `search_all_sources` 返回结果
3. 点击搜索结果中的某个影片
4. 验证跳转到 `/player/source/{url}?source={source}`

Expected: 能成功播放或显示有意义的错误（而非空白页/报错）

- [ ] **Step 3: 提交验证结果**

如果测试通过：
```bash
git add -A && git commit -m "test: verify provider-based playback E2E"
```

如果有问题，修复后重新测试再提交。

---

## 执行选项

**Plan complete and saved to `docs/superpowers/plans/2026-04-27-voddetail-provider-migration.md`.**

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**