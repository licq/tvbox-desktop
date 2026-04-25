# 懒加载详情优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化详情页加载体验 - 显示骨架屏，后台按需抓取剧集列表

**Architecture:** 前端 store 添加 lazy loading 状态管理，VodDetail.vue 显示骨架屏；后端已有懒加载逻辑无需修改

**Tech Stack:** Vue 3 + Pinia + TypeScript, Rust

---

## 文件映射

| 文件 | 变更 |
|---|---|
| `src/stores/detail.ts` | 添加 lazy loading 状态和方法 |
| `src/views/VodDetail.vue` | 添加骨架屏 UI 逻辑 |
| `src/components/detail/EpisodeGroupSkeleton.vue` | 新建骨架屏组件 |

---

## Task 1: 添加 lazy loading 状态到 detail store

**Files:**
- Modify: `src/stores/detail.ts:6-45`

- [ ] **Step 1: Read current detail store**

```typescript
// src/stores/detail.ts 当前结构
export const useDetailStore = defineStore('detail', () => {
  const item = ref<CatalogDetailItem | null>(null)
  const episodeGroups = ref<CatalogEpisodeGroup[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const recommendedGroup = computed(() => episodeGroups.value[0] ?? null)

  async function fetchDetail(itemId: number) {
    loading.value = true
    // ...现有逻辑
  }
```

- [ ] **Step 2: 添加 lazy loading 相关状态**

在 `loading` 后面添加:
```typescript
const isLazyLoading = ref(false)  // 后台正在抓取详情
const lazyLoadError = ref<string | null>(null)  // 懒加载错误信息
```

- [ ] **Step 3: 添加 lazy loading 方法**

```typescript
async function fetchDetailWithLazyLoad(itemId: number) {
  loading.value = true
  error.value = null
  try {
    const detail = await invoke<CatalogDetail>('get_catalog_detail', { id: itemId })
    item.value = detail.item
    episodeGroups.value = detail.episode_groups

    // 如果剧集为空，触发后台懒加载
    if (detail.episode_groups.length === 0 && detail.item.detail_json) {
      isLazyLoading.value = true
      lazyLoadError.value = null
      // 后台抓取已在 get_catalog_detail 中触发
      // 等待一下再检查一次
      await new Promise(resolve => setTimeout(resolve, 500))
      const refreshed = await invoke<CatalogDetail>('get_catalog_detail', { id: itemId })
      episodeGroups.value = refreshed.episode_groups
      isLazyLoading.value = false
    }
  } catch (e) {
    item.value = null
    episodeGroups.value = []
    error.value = String(e)
    throw e
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 4: 暴露新状态**

```typescript
return {
  item,
  episodeGroups,
  loading,
  error,
  isLazyLoading,
  lazyLoadError,
  recommendedGroup,
  fetchDetail,
  fetchDetailWithLazyLoad,
  reset
}
```

- [ ] **Step 5: 更新 fetchDetail 调用**

将 `fetchDetail` 中的逻辑改为在初始加载后检查是否需要懒加载（保持兼容）。

---

## Task 2: 创建骨架屏组件

**Files:**
- Create: `src/components/detail/EpisodeGroupSkeleton.vue`

- [ ] **Step 1: 创建骨架屏组件**

```vue
<script setup lang="ts">
defineProps<{
  count?: number  // 显示几个骨架项，默认 6
}>()
</script>

<template>
  <div class="episode-group-skeleton space-y-4">
    <div class="skeleton-header">
      <div class="skeleton-title"></div>
    </div>
    <div class="flex flex-wrap gap-2">
      <div
        v-for="i in (count ?? 6)"
        :key="i"
        class="skeleton-chip"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.skeleton-header {
  @apply mb-3;
}

.skeleton-title {
  @apply h-5 w-32 rounded bg-white/10 animate-pulse;
}

.skeleton-chip {
  @apply h-9 w-20 rounded-full bg-white/10 animate-pulse;
}
</style>
```

---

## Task 3: 修改 VodDetail.vue 显示骨架屏

**Files:**
- Modify: `src/views/VodDetail.vue:26-50`
- Modify: `src/views/VodDetail.vue:76-89`

- [ ] **Step 1: 更新 onMounted 使用新方法**

```typescript
async function loadDetail() {
  if (!Number.isFinite(itemId.value) || itemId.value <= 0) {
    return
  }
  await detailStore.fetchDetailWithLazyLoad(itemId.value)
}
```

- [ ] **Step 2: 在 EpisodeGroupPanel 前添加骨架屏**

```vue
<!-- 懒加载中显示骨架屏 -->
<section v-if="detailStore.isLazyLoading" class="space-y-4">
  <EpisodeGroupSkeleton :count="8" />
</section>

<!-- 已有剧集时正常显示 -->
<section v-else-if="detailStore.episodeGroups.length" class="space-y-4">
  <EpisodeGroupPanel
    v-for="group in detailStore.episodeGroups"
    :key="group.source_name"
    :group="group"
    :recommended="group.source_name === detailStore.recommendedGroup?.source_name"
    @play="handlePlay"
  />
</section>

<!-- 有基本信息但没有剧集且不是懒加载状态 -->
<div v-else-if="!detailStore.isLazyLoading && detailStore.item" class="home-empty-state">
  当前内容没有可展示的播放入口。
</div>
```

---

## Task 4: 验证编译通过

**Files:**
- Test: `src/stores/detail.ts`

- [ ] **Step 1: 运行 TypeScript 检查**

```bash
npm run build 2>&1 | head -50
```

Expected: 无 TypeScript 错误

---

## Task 5: 运行测试

- [ ] **Step 1: 运行现有测试**

```bash
npm run test 2>&1
```

Expected: 所有测试通过

---

## 验证清单

1. `npm run build` 编译通过
2. `npm run test` 测试通过
3. 手动测试：点击一个没有预加载 episodes 的视频，观察骨架屏出现然后展开

---

## 风险与注意事项

1. **双重调用**: `fetchDetailWithLazyLoad` 中调用了两次 `get_catalog_detail`，第二次是为了获取更新后的数据。但后端 `get_catalog_detail` 在 `episode_groups.is_empty()` 时会触发懒加载，这个逻辑已经在后端存在，不需要前端主动触发两次。优化方案：前端只调用一次，后端异步抓取完成后返回的数据自然包含 episodes。

2. **实际后端逻辑**: 查看 `get_catalog_detail` 代码，后端已经在首次调用时自动触发懒加载。问题是前端不知道后端正在异步抓取，需要返回某个标志让前端知道"数据还在后台加载中"。但由于后端是同步返回的，这个设计已经工作，只是前端需要在等待期间显示 loading 状态。

简化方案：前端调用 `get_catalog_detail` 时，如果有 episode_groups 且 loading 结束后仍然为空，前端应该轮询或者等待一段时间后重新调用。但当前实现已经能够工作（后端会等待抓取完成），只是前端 UI 需要改进显示 loading 状态。

**关于后端懒加载行为（重要）**：

查看 `get_catalog_detail` 代码，后端已经是同步阻塞的：
```rust
// vod.rs:90-120
if detail.episode_groups.is_empty() {
    if let Some(detail_json) = detail.item.detail_json.clone() {
        match scrape_catalog_detail_from_json(&detail_json).await {
            // 抓取完成后自动返回更新后的数据
```

这意味着：
- 如果 SQLite 有 episodes → 立即返回（有数据）
- 如果 SQLite 无 episodes，但有 detail_json → **阻塞抓取**，完成后返回（有数据）

**问题**：当条目从未被访问过时（无 episodes），第一次点击需要等待抓取（约 1-3 秒）。当前 UI 只有整体 loading spinner，无法让用户感知"正在后台加载详情"。

**解决方案**：
- 前端：在 episode 区域显示"加载中..."骨架屏（不阻塞页面其他部分）
- 数据流：前端调用 `get_catalog_detail`，后端阻塞等待抓取，前端收到完整数据后关闭骨架屏

**注意**：不需要修改后端逻辑，只需要在 `get_catalog_detail` 返回后检查状态并正确显示 UI。

---

## 修正后的 Task 1: detail store 简化版

**不需要 `fetchDetailWithLazyLoad`** —— 后端已经是同步阻塞的。只需要在 UI 上根据 `item` 是否有值来显示骨架屏或整体 spinner。