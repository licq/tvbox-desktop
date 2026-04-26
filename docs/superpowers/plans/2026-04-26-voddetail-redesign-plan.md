# 详情页重新实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 重新设计详情页，上半部分展示豆瓣风格的详细信息（三栏布局：海报 + 元数据 + 剧情简介），下半部分展示所有搜索线路及结果

**Architecture:** 三栏布局上半部分 + 播放源列表下半部分。DoubanMetaPanel 改为三栏布局，VodDetail.vue 重新设计源码搜索结果显示

**Tech Stack:** Vue 3 + Pinia + Tauri 2.x + Rust

---

## 文件变更概览

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/components/detail/DoubanMetaPanel.vue` | 修改 | 改为三栏布局：海报 | 元数据 | 剧情简介 |
| `src/views/VodDetail.vue` | 修改 | 重写源码搜索结果区域，按 source 分组显示 |
| `src/types/index.ts` | 修改 | SearchResult 保留，但 handleSearchResultPlay 行为改变 |
| `src-tauri/src/commands/douban.rs` | 修改 | search_vod_sources 需要返回按 source 分组的结果 |

---

## Task 1: 修改 DoubanMetaPanel.vue 为三栏布局

**Files:**
- Modify: `src/components/detail/DoubanMetaPanel.vue`

- [ ] **Step 1: 读取当前 DoubanMetaPanel.vue 结构**

文件已读，当前是 2-column (poster | content) 布局。

- [ ] **Step 2: 修改为三栏 CSS 布局**

```vue
<style scoped>
.douban-meta-panel {
  display: grid;
  grid-template-columns: 180px 1fr 280px;
  gap: 1.5rem;
  padding: 1.5rem;
  background: rgba(255,255,255,0.05);
  border-radius: 1.5rem;
}
.douban-meta-poster {
  /* 保持现有的 flex 布局 */
}
.douban-meta-content {
  /* 保持现有的 flex 布局 */
}
/* 新增 summary 区域 */
.douban-meta-summary {
  border-left: 1px solid rgba(255,255,255,0.1);
  padding-left: 1.5rem;
}
.douban-meta-summary h3 {
  font-size: 1rem;
  color: rgba(255,255,255,0.5);
  margin-bottom: 0.75rem;
  font-weight: 500;
}
.douban-meta-summary p {
  color: rgba(255,255,255,0.7);
  font-size: 0.9rem;
  line-height: 1.6;
}
</style>
```

- [ ] **Step 3: 修改模板结构，添加剧情简介区域**

```vue
<section class="douban-meta-panel">
  <div class="douban-meta-poster">
    <img v-if="poster" :src="poster" :alt="meta.title" class="poster-img" />
    <img v-else-if="meta.poster" :src="meta.poster" :alt="meta.title" class="poster-img" />
    <div v-else class="poster-fallback">{{ meta.title }}</div>
  </div>

  <div class="douban-meta-content">
    <!-- 现有内容保持不变：标题、评分、列表数据 -->
    <h1 class="douban-meta-title">{{ meta.title }}</h1>
    <div v-if="meta.rating" class="douban-meta-rating">...</div>
    <dl class="douban-meta-list">...</dl>
  </div>

  <!-- 新增剧情简介区域 -->
  <div v-if="meta.summary" class="douban-meta-summary">
    <h3>剧情简介</h3>
    <p>{{ meta.summary }}</p>
  </div>
</section>
```

- [ ] **Step 4: 验证并提交**

```bash
git add src/components/detail/DoubanMetaPanel.vue
git commit -m "refactor(DoubanMetaPanel): add three-column layout with summary section"
```

---

## Task 2: 重写 VodDetail.vue 源码搜索结果显示

**Files:**
- Modify: `src/views/VodDetail.vue:140-185` (Douban hot direct entry section)

- [ ] **Step 1: 修改模板 - 替换 source list 区域**

找到 `v-else-if="isFromDouban"` 区块中的 source list 部分，替换为：

```vue
<!-- Search results from sources -->
<section v-if="loadingSearch" class="space-y-4">
  <EpisodeGroupSkeleton :count="4" />
</section>

<section v-else-if="searchResults.length" class="source-list space-y-4">
  <div
    v-for="group in searchResults"
    :key="group.source_name"
    class="rounded-xl bg-white/5 p-4"
  >
    <div class="mb-3 flex items-center justify-between">
      <h3 class="text-lg font-semibold text-white">{{ group.source_name }}</h3>
      <span class="text-sm text-white/40">{{ group.results.length }} 个结果</span>
    </div>
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      <div
        v-for="result in group.results"
        :key="result.detail_url"
        class="flex items-center gap-3 rounded-lg bg-white/5 p-3 cursor-pointer hover:bg-white/10 transition-colors"
        @click="handleSearchResultPlay(result)"
      >
        <img v-if="result.poster" :src="result.poster" class="w-12 h-16 object-cover rounded" />
        <div class="flex-1 min-w-0">
          <p class="text-white text-sm font-medium truncate">{{ result.title || doubanMeta?.title }}</p>
          <p class="text-white/40 text-xs">{{ result.source_name }}</p>
        </div>
      </div>
    </div>
  </div>
</section>

<div v-else-if="doubanMeta && !loadingSearch" class="home-empty-state">
  暂未找到可用的播放源
</div>
```

- [ ] **Step 2: 修改 handleSearchResultPlay 函数**

当前代码跳转到 detail 页面，改为直接使用 `result.detail_url` 作为播放地址（或者保持跳转到 detail 由 detail 页面处理播放）：

根据 spec，点击搜索结果应该直接进入播放界面。但 `result.detail_url` 是 source 的详情页 URL（如 `https://www.zxzjhd.com/vod/12345.html`），不是播放地址。

需要一个新的 Tauri command 来：
1. 访问 `result.detail_url`
2. 提取播放地址
3. 返回播放地址给前端

或者保持现状：点击搜索结果 → 跳转到 detail 页面 → detail 页面获取播放地址 → 播放

由于涉及跨站 scraping，直接播放需要额外处理。保持当前跳转到 detail 的逻辑，但确保 detail 页面能正确处理 `from=search` 的情况。

```typescript
function handleSearchResultPlay(result: SearchResult) {
  // 跳转到播放页面，使用 detail_url 作为标识
  // 播放页面需要从 detail_url 提取真正的播放地址
  router.push(`/player/source/${encodeURIComponent(result.detail_url)}?source=${result.source}&title=${encodeURIComponent(result.title || '')}`)
}
```

这需要新增 player 路由 `/player/source/:detailUrl`，由 PlayerPage 处理跨源播放。

- [ ] **Step 3: 提交**

```bash
git add src/views/VodDetail.vue
git commit -m "refactor(VodDetail): update source search results display"
```

---

## Task 3: 新增 /player/source/:detailUrl 路由处理跨源播放

**Files:**
- Modify: `src/router/index.ts` - 添加新路由
- Modify: `src/views/PlayerPage.vue` - 处理 source detail URL 播放

- [ ] **Step 1: 在 router 中添加新路由**

```typescript
{
  path: '/player/source/:detailUrl',
  name: 'player-source',
  component: () => import('@/views/PlayerPage.vue'),
  props: true
}
```

- [ ] **Step 2: 修改 PlayerPage 支持 source detailUrl 模式**

在 PlayerPage 中添加：
- `route.params.detailUrl` - 编码后的 source 详情页 URL
- `route.query.source` - 来源标识 (zxzj/jpvod/xb6v)
- 调用新的 Tauri command `play_from_source_detail` 获取播放地址

```typescript
const detailUrl = computed(() => decodeURIComponent(route.params.detailUrl as string))
const source = computed(() => route.query.source as string)

// 加载时调用 command 获取播放地址
async function loadSourceDetail() {
  const playUrl = await invoke<string>('play_from_source_detail', {
    detailUrl: detailUrl.value,
    source: source.value
  })
  // 设置 playUrl 并开始播放
}
```

- [ ] **Step 3: 在 Rust侧新增 command play_from_source_detail**

```rust
#[tauri::command]
pub async fn play_from_source_detail(
    detail_url: String,
    source: String,
) -> Result<String, String> {
    // 根据 source 调用对应的 scraper 获取播放地址
    match source.as_str() {
        "zxzj" => crate::services::zxzj::extract_player_url(&detail_url).await,
        "jpvod" => crate::services::jpvod::extract_player_url(&detail_url).await,
        "xb6v" => crate::services::xb6v::extract_player_url(&detail_url).await,
        _ => Err("Unknown source".to_string()),
    }
}
```

每个 source scraper 需要新增 `extract_player_url(detail_url: &str) -> Result<String, String>` 函数。

- [ ] **Step 4: 提交**

```bash
git add src/router/index.ts src/views/PlayerPage.vue src-tauri/src/commands/player.rs
git commit -m "feat(player): add source detail URL playback support"
```

---

## Task 4: 为每个 source scraper 实现 extract_player_url 函数

**Files:**
- Modify: `src-tauri/src/services/zxzj.rs`
- Modify: `src-tauri/src/services/jpvod.rs`
- Modify: `src-tauri/src/services/xb6v.rs`

每个 source 的 extract_player_url 实现逻辑类似：
1. 访问 detail_url
2. 解析 HTML 找到播放按钮/链接
3. 返回播放地址

由于这个任务是较大改动，建议先完成核心的 DoubanMetaPanel 和 VodDetail 布局修改，source detail 播放作为后续任务。

---

## 简化实现策略

考虑到复杂性，建议分阶段实现：

**Phase 1（当前 plan）：**
1. ✅ DoubanMetaPanel 改为三栏布局
2. ✅ VodDetail 优化搜索结果显示
3. 点击搜索结果 → 仍跳转到 detail 页面，但 detail 页面特殊处理 `from=search` 的情况

**Phase 2（后续）：**
- 实现 `play_from_source_detail` command
- 实现跨源直接播放

---

## 验收标准

1. ✅ DoubanMetaPanel 显示海报、标题、评分、元数据、剧情简介（三栏布局）
2. ✅ 搜索结果按 source 分组显示，标注结果数量
3. ✅ 0 结果的 source 隐藏
4. ✅ 有错误的 source 显示错误信息
5. ✅ 点击搜索结果跳转到可播放的页面