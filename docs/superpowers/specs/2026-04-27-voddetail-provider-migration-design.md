# VodDetail Provider 化改造设计

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 VodDetail 页面的搜索和详情获取迁移到 Provider 系统，Home 页面的 VOD 数据保持豆瓣不变

**Architecture:**

- VOD 数据完全通过 Provider 实时拉取（无 VOD DB 预填充）
- `catalog_items` 表仅存储 Live 频道数据（Home 页面 Live tab 用）
- VOD 首页（movie/series/variety/anime）只用豆瓣热播数据（现有逻辑不变）
- Provider 的 `search()` / `detail()` / `play()` 构成完整 VOD 数据流

---

## 1. 数据流设计

### 各场景数据来源

| 场景 | 数据来源 | 说明 |
|------|---------|------|
| Home-Live tab | DB (`catalog_items`) | refresh_subscription 时写入 |
| Home-VOD tabs (movie/series/variety/anime) | Douban API | 现有逻辑不变 |
| VodDetail-Douban入口 (?douban=1) | Provider `search_all` | 用片名搜索，返回分组的搜索结果 |
| VodDetail-普通入口 | DB基本信息 → Provider `detail` | 实时拉取剧集列表 |
| 播放 | Provider `play()` | 实时获取播放地址 |

### 关键原则

1. **Home 页面 VOD tabs 保持豆瓣数据**：不做 Provider 调用
2. **VOD 数据不预填充**：Provider 结果直接展示，不写 DB
3. **实时拉取**：所有 VOD 数据（搜索/详情/播放）均通过 Provider 实时获取
4. **Live 数据才写 DB**：refresh_subscription 只刷新 Live 频道数据

---

## 2. Frontend 改动

### 2.1 store/detail.ts

`fetchDetail` 改为：实时调用 `provider.detail()` 获取剧集列表

```typescript
async function fetchDetail(itemId: number) {
  loading.value = true
  error.value = null
  try {
    // 从 DB 获取基本信息（title, poster, type 等）
    const detail = await invoke<CatalogDetail>('get_catalog_detail', { id: itemId })
    item.value = detail.item

    // 实时调用 Provider.detail() 获取剧集
    if (detail.item.detail_json) {
      const parsed = JSON.parse(detail.item.detail_json)
      const source = parsed.source
      const ids = parsed.ids || parsed.url || ''
      if (source && ids) {
        const episodes = await invoke<CatalogEpisode[]>('provider_detail', {
          source,
          ids,
        })
        if (episodes.length) {
          episodeGroups.value = [{
            source_name: parsed.source_name || source,
            episodes,
          }]
        }
      }
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

### 2.2 store/library.ts

**不需要改动** - `fetchCatalog` 只用于 Live tab，VOD tabs 用 Douban 数据（现有逻辑）。

### 2.3 views/VodDetail.vue

#### 豆瓣入口 (`?douban=1`)

**已有逻辑**，调用 `search_all_sources` 显示分组结果：

```typescript
async function searchSources(title: string) {
  loadingSearch.value = true
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
    // 按 source 分组，展示搜索结果
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      for (const item of r.items) {
        // item.detail_json 包含 {"source": "...", "ids": "..."}
        const detailData = item.detail_json ? JSON.parse(item.detail_json) : {}
        grouped[r.source_name] ||= []
        grouped[r.source_name].push({
          source: detailData.source || r.source_key,
          source_name: r.source_name,
          detail_url: item.detail_json || '',
          item_type: item.item_type as any,
          title: item.title,
          poster: item.poster,
        })
      }
    }
    searchResults.value = Object.entries(grouped).map(([source_name, results]) => ({ source_name, results }))
  } finally {
    loadingSearch.value = false
  }
}
```

#### 播放入口

点击搜索结果时，解析 `detail_url`（JSON 字符串）获取 source 和 ids，调用 Provider play：

```typescript
async function handleSearchResultPlay(result: SearchResult) {
  // detail_url 是 JSON 字符串: {"source": "site_key", "ids": "vod_id", "url": "..."}
  const parsed = JSON.parse(result.detail_url)
  const source = parsed.source
  const ids = parsed.ids || parsed.url || ''

  // 调用 provider_play 获取真实播放 URL
  const targets = await invoke<PlaybackTarget[]>('provider_play', {
    source,
    flag: 'auto',
    play_url: ids,
  })

  if (targets.length > 0) {
    const target = targets[0]
    router.push(`/player/source/${encodeURIComponent(target.target_url)}?source=${source}`)
  }
}
```

### 2.4 views/Home.vue

**不需要改动** - VOD tabs 只显示豆瓣数据（现有逻辑）。

---

## 3. Backend 改动

### 3.1 新增 Command: `provider_detail`

```rust
#[tauri::command]
pub async fn provider_detail(
    source: String,
    ids: String,
    state: State<'_, AppState>,
) -> Result<Vec<ScrapedCatalogEpisode>, String> {
    let registry = state.provider_registry.read().await;
    if let Some(provider) = registry.get(&source) {
        match provider.detail(&ids).await {
            Ok(Some(item)) => Ok(item.episodes),
            Ok(None) => Ok(Vec::new()),
            Err(e) => Err(format!("provider detail failed: {}", e)),
        }
    } else {
        Err(format!("provider not found: {}", source))
    }
}
```

### 3.2 新增 Command: `provider_play`

```rust
#[tauri::command]
pub async fn provider_play(
    source: String,
    flag: String,
    play_url: String,
    state: State<'_, AppState>,
) -> Result<Vec<PlaybackTarget>, String> {
    let registry = state.provider_registry.read().await;
    if let Some(provider) = registry.get(&source) {
        provider.play(&flag, &play_url).await.map_err(|e| format!("{}", e))
    } else {
        Err(format!("provider not found: {}", source))
    }
}
```

### 3.3 修改 `refresh_subscription`

移除 VOD 数据写入，只刷新 Live 数据：

```rust
// 移除 scrape_supported_tvbox_catalogs 调用
// 遍历 parsed.lives，写入/更新 live_channels 表
for live in &parsed.lives {
    // 写入 Live 频道数据到 DB
}
// 不再写入 catalog_items 表
```

### 3.4 修改 `get_catalog_detail`

移除 Provider 回退逻辑（迁移到前端实时调用）：

```rust
// get_catalog_detail 现在只返回 DB 数据，不做 Provider 补全
// Provider 实时拉取由前端控制
```

---

## 4. 类型定义更新

### 4.1 前端新增类型

```typescript
// src/types/index.ts

export interface ProviderSearchResult {
  source_key: string
  source_name: string
  items: ProviderCatalogItem[]
}

export interface ProviderCatalogItem {
  source_item_key: string  // "site_key:vod_id"
  title: string
  item_type: string
  poster?: string
  summary?: string
  detail_json?: string  // JSON: {"source": "site_key", "ids": "vod_id"}
  episodes: CatalogEpisode[]
}
```

### 4.2 更新 `SearchResult`

```typescript
export interface SearchResult {
  source: string  // 改为 string 兼容所有 source
  source_name: string
  detail_url: string  // JSON 字符串: {"source": "site_key", "ids": "vod_id"}
  item_type: 'movie' | 'series' | 'variety' | 'anime' | 'generic'
  title?: string
  poster?: string
}
```

---

## 5. 播放流程（完整路径）

### 豆瓣入口

```
VodDetail.searchSources(片名)
  → search_all_sources (实时)
  → 展示分组搜索结果 (item.title, poster, source_name)
  → handleSearchResultPlay(result)
      → provider_play(source, flag, ids)
      → 跳转到 /player/source/{target_url}?source={source}
```

### 普通入口

```
VodDetail.fetchDetail(itemId)
  → get_catalog_detail (DB)
  → provider_detail(source, ids) (实时)
  → 展示 episode_groups
  → handlePlay(episode)
      → provider_play(source, flag, play_url)
      → 跳转到 /player/source/{target_url}?source={source}
```

---

## 6. 任务分解

1. 新增 `provider_detail` Tauri command
2. 新增 `provider_play` Tauri command
3. 更新 `SearchResult` 前端类型
4. 修改 `refresh_subscription` 移除 VOD 写入（只保留 Live）
5. 修改 `get_catalog_detail` 移除 Provider 回退
6. 更新 `views/VodDetail.vue` 的搜索和播放逻辑
7. 验证播放流程 end-to-end