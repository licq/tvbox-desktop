# 搜索结果与豆瓣数据性能优化设计

**日期**: 2026-05-22
**状态**: 已批准

## 问题概述

1. **豆瓣数据慢**: 即使数据已在数据库，每次进入页面仍然慢
2. **搜索结果缓存无效**: 已缓存的搜索结果仍然需要重新加载
3. **蒙版布局不一致**: Skeleton 是横向布局，真实内容是纵向布局

## 根本原因

### 问题 1 & 2: 缓存架构缺陷

1. **缓存键不一致**:
   - 后端 (`commands/search.rs`): 直接使用原始 `keyword`
   - 前端 (`vodDetailSearchCache.ts`): 使用 `keyword.trim().toLowerCase()`

2. **后端豆瓣缓存未启用**: `get_douban_search_cache` 表存在但未被使用

3. **前端无请求记忆化**: 每次进入页面都会重新请求豆瓣元数据

4. **搜索快照不完整**: `preloadAllSources` 在快照保存前未完成

### 问题 3: CSS 布局不一致

- Skeleton: `flex-wrap: wrap` (横向排列)
- Real Card: CSS Grid `grid-template-columns: repeat(auto-fill, minmax(3rem, 1fr))`

## 解决方案: 双层缓存架构

```
[用户返回] → [前端内存缓存] → [后端 SQLite] → [Scraper]
                  ↓               ↓
            命中则秒级加载   命中则避免网络请求
```

## 详细设计

### 1. 后端: 缓存键规范化 (`commands/search.rs`)

```rust
let normalized_kw = keyword.trim().to_lowercase();
// 使用 normalized_kw 查询和写入缓存
```

影响命令:
- `search_all_sources`
- 所有豆瓣相关命令 (`fetch_douban_metadata_by_id`, `search_douban_subject_by_keyword` 等)

### 2. 后端: 激活豆瓣元数据缓存 (`commands/douban.rs`)

`fetch_douban_metadata_by_id` 命令流程:
1. 规范化 `douban_id` 为字符串键
2. 查询 `get_douban_search_cache(douban_id_str)`
3. 缓存命中: 反序列化返回，跳过 scraper
4. 缓存未命中: 调用 scraper → 结果写入 `set_douban_search_cache`
5. 缓存过期时间: 7 天

### 3. 前端: 请求记忆化 (`views/VodDetail.vue`)

```typescript
const doubanFetchState = ref<'idle' | 'loading' | 'done'>('idle')

// loadDetail 中
if (doubanFetchState.value === 'done') return  // 跳过已完成的请求
if (doubanFetchState.value === 'loading') return  // 跳过进行中的请求

doubanFetchState.value = 'loading'
// ... 完成后设置 doubanFetchState.value = 'done'
```

### 4. 前端: 搜索快照完整性 (`views/VodDetail.vue`)

修改 `searchSources` 函数的快照保存时机:
```typescript
// 当前: loadingSearch = false 后立即保存
// 修复: 等待所有 source preload 完成后再保存

await Promise.all(dedupSearchItems.value.map(item => preloadAllSources(item)))
// 此时 providerDetailCache 已完整

setVodDetailSearchSnapshot(normalizeVodDetailSearchKey(title), {
  searchResults: searchResults.value,
  providerDetailEntries: Array.from(providerDetailCache.value.entries()),
})
```

### 5. 前端: 修复 Skeleton CSS (`SearchResultCardSkeleton.vue`)

```css
/* 修改前: flex-wrap */
.skeleton-episode-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
  justify-content: flex-end;
}

/* 修改后: CSS Grid，与真实卡片一致 */
.skeleton-episode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(3.25rem, 1fr));
  gap: 0.35rem;
}
```

## 改动清单

| 文件 | 改动 | 说明 |
|------|------|------|
| `src-tauri/src/commands/search.rs` | 缓存键规范化 | `keyword.trim().to_lowercase()` |
| `src-tauri/src/commands/douban.rs` | 激活豆瓣缓存 | 先查 `get_douban_search_cache` 再请求 scraper |
| `src/views/VodDetail.vue` | 请求记忆化 | 添加 `doubanFetchState` 避免重复请求 |
| `src/views/VodDetail.vue` | 快照完整性 | 等待 `preloadAllSources` 完成后再保存快照 |
| `src/components/detail/SearchResultCardSkeleton.vue` | CSS Grid | 布局与真实卡片一致 |

## 预期效果

- 豆瓣数据: 缓存命中时直接返回，无需网络请求
- 搜索结果: 前端内存缓存 + 后端 SQLite 缓存双重保障
- UI 体验: Skeleton 与真实内容布局一致，避免视觉跳跃