# 搜索结果与豆瓣数据性能优化实现计划

**设计**: `docs/plans/2026-05-22-voddetail-performance-design.md`
**状态**: 准备实现

## 任务列表

### 后端改动

#### 1. 搜索缓存键规范化
**文件**: `src-tauri/src/commands/search.rs`

```rust
// 在 search_all_sources 函数开头添加
let normalized_kw = keyword.trim().to_lowercase();

// 替换所有使用 keyword 的地方为 normalized_kw
```

**验证**: Rust 编译通过

#### 2. 豆瓣元数据缓存激活
**文件**: `src-tauri/src/commands/douban.rs`

在 `fetch_douban_metadata_by_id` 命令中:
1. 添加 `use crate::services::storage::Storage;` (如需要)
2. 查询 `storage.get_douban_search_cache(douban_id_str)`
3. 缓存命中时反序列化返回
4. 缓存未命中时调用 scraper，结果写入缓存

**验证**: Rust 编译通过

---

### 前端改动

#### 3. 请求记忆化
**文件**: `src/views/VodDetail.vue`

添加状态变量:
```typescript
const doubanFetchState = ref<'idle' | 'loading' | 'done'>('idle')
```

在 `loadDetail` 的豆瓣请求部分添加状态检查:
```typescript
if (doubanFetchState.value === 'done') return
if (doubanFetchState.value === 'loading') return
doubanFetchState.value = 'loading'
// ... 请求完成后
doubanFetchState.value = 'done'
```

**验证**: TypeScript 编译通过 + 页面功能正常

#### 4. 搜索快照完整性
**文件**: `src/views/VodDetail.vue`

修改 `searchSources` 函数中快照保存时机:
```typescript
// 找到 setVodDetailSearchSnapshot 调用
// 确保它在 preloadAllSources 之后
await Promise.all(dedupSearchItems.value.map(item => preloadAllSources(item)))
// 然后再调用 setVodDetailSearchSnapshot
```

**验证**: 缓存命中时数据完整

#### 5. Skeleton CSS 修复
**文件**: `src/components/detail/SearchResultCardSkeleton.vue`

修改 `.skeleton-episode-grid` 样式:
```css
/* 替换 flex 为 grid */
.skeleton-episode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(3.25rem, 1fr));
  gap: 0.35rem;
}
```

同步修改响应式媒体查询中的布局

**验证**: Build 通过 + 视觉一致

---

## 实现顺序

1. **后端改动** (1, 2) - 完成后测试 Rust 编译
2. **前端改动** (3, 4, 5) - 完成后测试 Build
3. **整体验证** - 测试返回播放页面的性能

## 风险与注意事项

- 缓存键规范化可能影响现有缓存的兼容性（需要考虑迁移或清除旧缓存）
- 豆瓣缓存的过期时间设置为 7 天，与搜索缓存一致