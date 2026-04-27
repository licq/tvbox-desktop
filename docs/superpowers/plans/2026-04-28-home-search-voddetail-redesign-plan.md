# Home 搜索 & VodDetail 播放源列表重新设计 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 Home 页搜索独立为 tab + 重构 VodDetail 播放源列表为卡片式设计

**Architecture:** Home.vue 新增 "搜索" tab，移除其他 tab 搜索框；EpisodeGroupPanel.vue 重构为卡片式，根据 item_type 区分电影（版本按钮）和剧集（集数芯片）模式

**Tech Stack:** Vue 3 + Pinia + Tauri 2.x

---

## File Structure

### 修改文件

| 文件 | 职责 |
|------|------|
| `src/views/Home.vue` | 新增搜索 tab、搜索 tab 内容区、移除其他 tab 搜索框 |
| `src/components/detail/EpisodeGroupPanel.vue` | 重构为卡片式设计，支持电影/剧集两种模式 |
| `src/views/VodDetail.vue` | 更新下半部分使用新的 EpisodeGroupPanel |

### 无新增文件

所有功能通过修改现有组件实现，无需创建新文件或新路由。

---

### Task 1: Home 导航栏 — 新增搜索 tab + 移除搜索框

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 修改类型定义**

在 `Home.vue` 顶部，修改 `HomeTabKey` 类型以包含 `search`：

```typescript
// Before
type HomeTabKey = 'live' | CatalogItemType

// After
type HomeTabKey = 'live' | CatalogItemType | 'search'
```

- [ ] **Step 2: 修改 tabs 列表，添加搜索 tab**

```typescript
// Before
const tabs = computed(() => {
  const fixedTabs = [
    { key: 'live', label: '直播', eyebrow: 'Live' },
    { key: 'movie', label: '电影', eyebrow: 'Movie' },
    { key: 'series', label: '剧集', eyebrow: 'Series' },
    { key: 'variety', label: '综艺', eyebrow: 'Shows' },
    { key: 'anime', label: '动漫', eyebrow: 'Anime' },
  ]
  return fixedTabs
})

// After
const tabs = computed(() => {
  const fixedTabs: { key: HomeTabKey; label: string; eyebrow?: string }[] = [
    { key: 'live', label: '直播', eyebrow: 'Live' },
    { key: 'movie', label: '电影', eyebrow: 'Movie' },
    { key: 'series', label: '剧集', eyebrow: 'Series' },
    { key: 'variety', label: '综艺', eyebrow: 'Shows' },
    { key: 'anime', label: '动漫', eyebrow: 'Anime' },
    { key: 'search', label: '搜索', eyebrow: 'Search' },
  ]
  return fixedTabs
})
```

- [ ] **Step 3: 移除所有非搜索 tab 的搜索框**

在 template 中，找到 `<section class="home-secondary-browser">` 里的 `<SearchBar>` 部分，将其包裹在条件判断中，仅在搜索 tab 显示：

```diff
- <div class="home-secondary-search">
+ <div v-if="activeTab === 'search'" class="home-secondary-search">
    <SearchBar
-     :placeholder="activeTab === 'live' ? '搜索频道、卫视、央视频道...' : `搜索${formatTypeLabel(activeTab)}...`"
+     placeholder="搜索电影、剧集、综艺..."
      @search="handleVodSearch"
    />
  </div>
```

- [ ] **Step 4: 更新 tab 切换逻辑**

当前 `watch` 在切换到非 live tab 时调用 `fetchDoubanHotByType`。搜索 tab 不需要该操作：

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
    providerSearchResults.value = []
    searchFilter.value = 'all'

    if (nextTab === 'search') {
      // Search tab: do not preload any data
      return
    }

    if (nextTab === 'live') {
      // existing live logic remains
    } else {
      await libraryStore.fetchDoubanHotByType(nextTab)
    }
  },
  { immediate: true }
)
```

- [ ] **Step 5: 更新 handleVodSearch**

搜索 tab 只处理 VOD 搜索（无直播搜索）：

```typescript
// Before
async function handleVodSearch(keyword: string) {
  if (keyword) {
    providerSearchResults.value = []
    if (activeTab.value !== 'live') {
      void libraryStore.fetchCatalog(activeTab.value, keyword)
      await searchAllProviders(keyword)
    }
    return
  }
  // No keyword: clear search
  providerSearchResults.value = []
  searchKeyword.value = ''
  if (activeTab.value !== 'live') {
    void libraryStore.fetchCatalog(activeTab.value)
  }
}

// After - simplified, only works in search tab
async function handleVodSearch(keyword: string) {
  if (keyword) {
    searchKeyword.value = keyword
    providerSearchResults.value = []
    void libraryStore.fetchCatalog(activeTab.value, keyword)
    await searchAllProviders(keyword)
    return
  }
  // No keyword: clear search area
  searchKeyword.value = ''
  providerSearchResults.value = []
}
```

- [ ] **Step 6: hydrateSources 跳过搜索 tab**

```typescript
async function hydrateSources() {
  // Skip data loading for search tab
  if (activeTab.value === 'search') return

  // ... rest of existing logic
}
```

- [ ] **Step 7: 提交**

```bash
git add src/views/Home.vue
git commit -m "feat(home): add search tab, remove search bars from other tabs"
```

---

### Task 2: Home 搜索 Tab 内容区 — 搜索框 + 筛选 + 结果网格

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 添加 MediaCard 导入和搜索相关状态**

在 `<script setup>` 开头添加 import：

```typescript
import MediaCard from '@/components/media/MediaCard.vue'
```

在 `searchKeyword` 和 `providerSearchResults` 下方添加搜索筛选状态：

```typescript
import type { CatalogItemType, DoubanHot, LiveChannel, SourceSearchResult, ProviderCatalogItem } from '@/types'

interface FlatSearchItem extends ProviderCatalogItem {
  source_name: string
}

const searchFilter = ref<'all' | CatalogItemType>('all')

const allSearchItems = computed<FlatSearchItem[]>(() => {
  return providerSearchResults.value.flatMap(group =>
    group.results.map(item => ({ ...item, source_name: group.source_name }))
  )
})

const filteredSearchItems = computed(() => {
  if (searchFilter.value === 'all') return allSearchItems.value
  return allSearchItems.value.filter(
    item => item.item_type === searchFilter.value
  )
})
```

- [ ] **Step 2: 添加搜索 tab 内容区的 template**

当前 Home.vue template 在 `<section class="home-secondary-browser">` 内的结构是：

```
v-if="activeTab === 'live'" → 直播内容
v-else-if="loadingProviderSearch" → 加载中
v-else-if="providerSearchResults.length" → 搜索结果（按 source 分组）
v-else → 豆瓣热播网格
```

改为：

```
v-if="activeTab === 'live'" → 直播内容（不变）
v-else-if="activeTab === 'search'" → 搜索 tab 内容（新）
v-else → 豆瓣热播（移除搜索相关逻辑）
```

在 `v-if="activeTab === 'live'"` 的 `</div>` 结束之后，添加搜索 tab 内容区：

```vue
<div v-else-if="activeTab === 'search'">
  <!-- Search results area -->
  <div v-if="loadingProviderSearch" class="flex min-h-[220px] items-center justify-center">
    <LoadingSpinner />
  </div>

  <div v-else-if="searchKeyword && providerSearchResults.length" class="mt-6">
    <!-- Filter pills -->
    <div class="mb-4 flex flex-wrap gap-2">
      <button
        :class="['rounded-full px-3 py-1 text-xs transition-colors', searchFilter === 'all' ? 'bg-accent/20 text-accent-strong' : 'bg-white/5 text-white/50 hover:text-white/70']"
        @click="searchFilter = 'all'"
      >
        全部
        <span class="opacity-50">({{ allSearchItems.length }})</span>
      </button>
      <button
        v-for="type in (['movie', 'series', 'variety', 'anime'] as const)"
        :key="type"
        :class="['rounded-full px-3 py-1 text-xs transition-colors', searchFilter === type ? 'bg-accent/20 text-accent-strong' : 'bg-white/5 text-white/50 hover:text-white/70']"
        @click="searchFilter = type"
      >
        {{ formatTypeLabel(type) }}
        <span class="opacity-50">({{ allSearchItems.filter(i => i.item_type === type).length }})</span>
      </button>
    </div>

    <!-- Result count -->
    <div class="mb-4 text-sm text-white/40">
      找到 <span class="text-white/60">{{ filteredSearchItems.length }}</span> 个结果
    </div>

    <!-- Result grid -->
    <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
      <MediaCard
        v-for="item in filteredSearchItems"
        :key="item.source_item_key"
        :title="item.title"
        :poster="item.poster"
        :subtitle="item.source_name"
        class="cursor-pointer"
        @click="handleProviderResultClick(item)"
      />
    </div>
  </div>

  <!-- Has search keyword but no results -->
  <div v-else-if="searchKeyword && !loadingProviderSearch" class="home-empty-state">
    未找到与"{{ searchKeyword }}"相关的内容
  </div>

  <!-- Initial state (no search yet) -->
  <div v-else class="home-empty-state">
    输入关键词搜索电影、剧集、综艺和动漫
  </div>
</div>
```

- [ ] **Step 3: 提交**

```bash
git add src/views/Home.vue
git commit -m "feat(home): build search tab content with filters and result grid"
```

---

### Task 3: EpisodeGroupPanel — 重构为卡片式设计

**Files:**
- Modify: `src/components/detail/EpisodeGroupPanel.vue`

- [ ] **Step 1: 重写组件脚本和模板**

```vue
<script setup lang="ts">
import { computed, ref } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisode, CatalogEpisodeGroup, CatalogItemType } from '@/types'

const COLLAPSE_THRESHOLD = 24

const props = defineProps<{
  group: CatalogEpisodeGroup
  item_type?: CatalogItemType
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()

const isMovie = computed(() => props.item_type === 'movie')
const expanded = ref(false)

const displayEpisodes = computed(() => {
  if (expanded.value || props.group.episodes.length <= COLLAPSE_THRESHOLD) {
    return props.group.episodes
  }
  return props.group.episodes.slice(0, COLLAPSE_THRESHOLD)
})
</script>

<template>
  <section class="source-group-card">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <span class="source-group-name">{{ group.source_name }}</span>
        <SourceBadge
          :label="isMovie ? `${group.episodes.length} 个版本` : `${group.episodes.length} 集`"
          tone="warm"
        />
      </div>
      <span class="source-group-type-tag">{{ isMovie ? '电影' : '剧集' }}</span>
    </div>

    <div class="source-group-body">
      <!-- Movie mode: version buttons -->
      <div v-if="isMovie" class="version-button-row">
        <button
          v-for="episode in group.episodes"
          :key="episode.id"
          class="version-button"
          @click="emit('play', episode)"
        >
          ▶ {{ episode.episode_label }}
        </button>
      </div>

      <!-- Series mode: episode chips grid -->
      <div v-else>
        <div class="episode-chip-grid">
          <EpisodeChip
            v-for="episode in displayEpisodes"
            :key="episode.id"
            :label="episode.episode_label"
            state="playable"
            @click="emit('play', episode)"
          />
        </div>
        <button
          v-if="group.episodes.length > COLLAPSE_THRESHOLD && !expanded"
          class="expand-button"
          @click="expanded = true"
        >
          展开全部 ({{ group.episodes.length }}集)
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.7rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.7rem 1rem;
}
.version-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.version-button {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 0.5rem;
  padding: 0.35rem 0.7rem;
  font-size: 0.72rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.version-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.episode-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.expand-button {
  margin-top: 0.5rem;
  border-radius: 0.5rem;
  padding: 0.35rem 0.8rem;
  font-size: 0.72rem;
  background: rgba(216, 154, 87, 0.1);
  border: 1px solid rgba(216, 154, 87, 0.2);
  color: rgba(240, 179, 107, 0.9);
  cursor: pointer;
  transition: all 180ms ease;
}
.expand-button:hover {
  background: rgba(216, 154, 87, 0.18);
}
</style>
```

- [ ] **Step 2: 提交**

```bash
git add src/components/detail/EpisodeGroupPanel.vue
git commit -m "feat(detail): redesign EpisodeGroupPanel as card-based with movie/series modes"
```

---

### Task 4: VodDetail — 集成新的 EpisodeGroupPanel

**Files:**
- Modify: `src/views/VodDetail.vue`

- [ ] **Step 1: 添加 providerItemType 状态**

在 `<script setup>` 中，`providerEpisodes` ref 下方添加：

```typescript
const providerItemType = ref<CatalogItemType>('movie')
```

- [ ] **Step 2: 在 handleSearchResultPlay 中记录 item_type**

```diff
async function handleSearchResultPlay(result: SearchResult) {
  const source = result.source
  const ids = result.detail_url

  if (!source || !ids) {
    searchError.value = '播放信息不完整'
    return
  }

  loadingProviderDetail.value = true
  providerDetailError.value = null
  providerEpisodes.value = null
+ providerItemType.value = result.item_type as CatalogItemType
```

- [ ] **Step 3: 给所有 EpisodeGroupPanel 添加 item_type prop**

找到 VodDetail.vue template 中所有使用 EpisodeGroupPanel 的地方（共两处）：

1. Provider episodes（搜索结果模式下点击某个 source 后的详情）：

```diff
  <EpisodeGroupPanel
    v-for="group in providerEpisodes"
    :key="group.source_name"
    :group="group"
+   :item_type="providerItemType"
    @play="handleProviderEpisodePlay"
  />
```

2. Catalog detail episodes（普通模式）：

```diff
  <EpisodeGroupPanel
    v-for="group in detailStore.episodeGroups"
    :key="group.source_name"
    :group="group"
+   :item_type="detailStore.item?.item_type"
    @play="handlePlay"
  />
```

- [ ] **Step 4: 提交**

```bash
git add src/views/VodDetail.vue
git commit -m "feat(detail): integrate card-based EpisodeGroupPanel with item_type"
```

---

### Task 5: 验证构建

**Files:**
- Build: `npm run build`
- Test: `npm run test`

- [ ] **Step 1: 运行 TypeScript 检查 + Vite 构建**

Run: `npm run build`
Expected: 无 TypeScript 或构建错误

- [ ] **Step 2: 运行现有测试**

Run: `npm run test`
Expected: PASS

- [ ] **Step 3: 修复任何构建错误直到通过**

- [ ] **Step 4: 提交修复**

```bash
git add -A
git commit -m "fix: resolve type and build issues from search tab and card redesign"
```
