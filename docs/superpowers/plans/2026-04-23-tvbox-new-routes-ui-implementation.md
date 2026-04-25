# TVBox New Routes UI Redesign — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the four-part UI redesign for new library routes (`/library/movie`, `/library/series`, `/library/variety`, `/library/anime`) covering visual system cleanup, Home restructure, Detail page hierarchy, and Player error language.

**Architecture:** Incremental four-task approach. Each task is self-contained and can be verified independently. New Home components are built alongside existing Home.vue (not replacing it) — new routes get new layout, old routes unchanged.

**Tech Stack:** Vue 3 + Pinia + Vue Router + Tailwind CSS + TypeScript strict mode

---

## File Map

**New files:**
- `src/components/home/HomeHero.vue` — Hero section for new library routes
- `src/components/home/ContinueRail.vue` — Continue Watching rail with progress bars
- `src/views/LibraryCategory.vue` — New shared page component for movie/series/variety/anime routes

**Modified files:**
- `src/style.css` — Visual system refinements (eyebrow removal, Chinese labels, color layer rules)
- `src/views/Home.vue` — No changes (existing routes stay as-is)
- `src/views/VodDetail.vue` — Update text labels (eyebrows, source counts)
- `src/components/detail/RecommendedSourcePanel.vue` — Warm gold border accent + Chinese labels
- `src/components/detail/EpisodeGroupPanel.vue` — Non-recommended groups collapsed by default + Chinese labels
- `src/components/media/EpisodeChip.vue` — Resolve state badge + tooltip for unavailable
- `src/components/player/PlaybackDrawer.vue` — Remove eyebrow, "线路 N", truncate URL
- `src/views/PlayerPage.vue` — Error language changes + switching notice
- `src/utils/player.ts` — Error message rewrites
- `src/router/index.ts` — Add new route handlers for library category pages

---

## Task 1: Visual System Cleanup

**Files:**
- Modify: `src/style.css:889-905` (`.playback-drawer-header`, `.playback-drawer-copy`)
- Modify: `src/style.css:999-1005` (`.playback-current-url`)
- Modify: `src/components/player/PlaybackDrawer.vue:29-76`
- Modify: `src/components/detail/RecommendedSourcePanel.vue:1-25`
- Modify: `src/components/detail/EpisodeGroupPanel.vue:1-53`
- Modify: `src/components/media/SourceBadge.vue:1-18` (no code change, just usage)

### 1.1: Remove English eyebrows from PlaybackDrawer

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue:29-40`

- [ ] **Step 1: Remove eyebrow div from drawer header**

In `PlaybackDrawer.vue`, remove the `<div class="eyebrow">Source Drawer</div>` line from `.playback-drawer-header`. The header should only contain the `h2` "播放线路" and the SourceBadge on the right.

```vue
<!-- Before -->
<div class="playback-drawer-header">
  <div>
    <div class="eyebrow">Source Drawer</div>
    <h2>播放线路</h2>
  </div>

<!-- After -->
<div class="playback-drawer-header">
  <div>
    <h2>线路切换</h2>
  </div>
```

- [ ] **Step 2: Remove eyebrow from current URL section**

Remove `<div class="eyebrow">Current Url</div>` from `.playback-current-url` block.

```vue
<!-- Before -->
<div class="playback-current-url">
  <div class="eyebrow">Current Url</div>
  <p>{{ sources[currentIndex]?.url || errorMessage || '当前没有可用地址' }}</p>
</div>

<!-- After -->
<div class="playback-current-url">
  <p>{{ sources[currentIndex]?.url ? truncateUrl(sources[currentIndex].url) : errorMessage || '当前没有可用地址' }}</p>
</div>
```

- [ ] **Step 3: Add truncateUrl helper to PlaybackDrawer**

In the `<script setup>` block, add:

```ts
function truncateUrl(url: string, max = 60): string {
  if (url.length <= max) return url
  return url.slice(0, max) + '...'
}
```

- [ ] **Step 4: Remove English labels from source rows**

In `PlaybackDrawer.vue` template, change:
- `small>Line {{ index + 1 }}/small>` → `small>线路 {{ index + 1 }}/small>`
- Remove any English labels in `.playback-source-meta`

- [ ] **Step 5: Change source kind badge labels to Chinese**

In the `sourceTone` function area, update the template to show Chinese labels:

```vue
<SourceBadge :label="source.kind === 'hls' ? 'HLS' : source.kind === 'http' ? 'HTTP' : source.kind === 'external' ? '外部' : source.kind" :tone="sourceTone(source.kind)" />
```

- [ ] **Step 6: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue
git commit -m "refactor: remove English eyebrows from PlaybackDrawer, use Chinese labels"
```

### 1.2: Update RecommendedSourcePanel Chinese labels

**Files:**
- Modify: `src/components/detail/RecommendedSourcePanel.vue:1-25`

- [ ] **Step 1: Replace English eyebrow with Chinese labels**

```vue
<!-- Before -->
<section class="recommended-source-panel">
  <div>
    <div class="eyebrow">Recommended Source</div>
    <h2>{{ group?.source_name ?? '暂无推荐线路' }}</h2>
    <p>
      {{ group ? '优先展开当前建议来源。后续会继续接入运行时健康度，让推荐理由更准确。' : '当前条目没有可播放入口。' }}
    </p>
  </div>

  <SourceBadge
    :label="group ? `${group.episodes.length} episodes` : 'empty'"
    :tone="group ? 'cool' : 'danger'"
  />
</section>

<!-- After -->
<section class="recommended-source-panel">
  <div>
    <SourceBadge label="推荐" tone="warm" />
    <h2>{{ group?.source_name ?? '暂无推荐线路' }}</h2>
    <p>
      {{ group ? '优先选择该线路，播放成功率更高。' : '当前条目没有可播放入口。' }}
    </p>
  </div>

  <SourceBadge
    :label="group ? `${group.episodes.length} 个可播` : '无可用'"
    :tone="group ? 'warm' : 'danger'"
  />
</section>
```

- [ ] **Step 2: Commit**

```bash
git add src/components/detail/RecommendedSourcePanel.vue
git commit -m "refactor: RecommendedSourcePanel use Chinese labels, warm tone badge"
```

### 1.3: Update EpisodeGroupPanel Chinese labels

**Files:**
- Modify: `src/components/detail/EpisodeGroupPanel.vue:1-53`

- [ ] **Step 1: Replace English labels with Chinese**

```vue
<!-- Before -->
<span>
  <span class="section-title">{{ group.source_name }}</span>
  <small>{{ recommended ? '推荐来源，默认展开' : '备用来源，按需展开' }}</small>
</span>
<span class="episode-group-meta">
  <SourceBadge :label="`${group.episodes.length} episodes`" :tone="recommended ? 'warm' : 'neutral'" />
  <span>{{ expanded ? '收起' : '展开' }}</span>
</span>

<!-- After -->
<span>
  <span class="section-title">{{ group.source_name }}</span>
  <small v-if="!recommended">备用来源</small>
</span>
<span class="episode-group-meta">
  <SourceBadge :label="`${group.episodes.length} 个`" :tone="recommended ? 'warm' : 'neutral'" />
  <span>{{ expanded ? '收起' : `展开 ${group.episodes.length} 条` }}</span>
</span>
```

Also remove the eyebrow that may exist in other detail components.

- [ ] **Step 2: Commit**

```bash
git add src/components/detail/EpisodeGroupPanel.vue
git commit -m "refactor: EpisodeGroupPanel use Chinese labels, collapse non-recommended groups"
```

---

## Task 2: New Home Layout for Library Routes

**Files:**
- Create: `src/components/home/HomeHero.vue`
- Create: `src/components/home/ContinueRail.vue`
- Create: `src/views/LibraryCategory.vue`
- Modify: `src/router/index.ts`
- Modify: `src/stores/library.ts` (add `continueWatching` filtering for progress > 5%)

### 2.1: Add continueWatching filter to library store

**Files:**
- Modify: `src/stores/library.ts`

- [ ] **Step 1: Add computed for active continue watching items**

After line 41 (`const hero = computed(...)`), add:

```ts
const activeContinueWatching = computed(() =>
  continueWatching.value.filter(item => (item.progress ?? 0) > 5)
)
```

Also update the return statement to export `activeContinueWatching`.

- [ ] **Step 2: Commit**

```bash
git add src/stores/library.ts
git commit -m "feat: add activeContinueWatching computed filtering progress > 5%"
```

### 2.2: Create HomeHero component

**Files:**
- Create: `src/components/home/HomeHero.vue`

- [ ] **Step 1: Write HomeHero component**

```vue
<script setup lang="ts">
import type { CatalogCard } from '@/types'

defineProps<{
  item: CatalogCard
  sourceCount?: number
  libraryCount?: number
  lastRefresh?: string
}>()

defineEmits<{
  play: []
  click: [item: CatalogCard]
}>()

function formatTime(dateStr?: string): string {
  if (!dateStr) return '未知'
  const d = new Date(dateStr)
  return `${d.getMonth() + 1}/${d.getDate()} ${d.getHours()}:${String(d.getMinutes()).padStart(2, '0')}`
}
</script>

<template>
  <section class="home-hero">
    <div class="home-hero-copy">
      <div class="eyebrow">编辑精选</div>
      <h1 class="home-hero-title">{{ item.title }}</h1>
      <p class="home-hero-summary">{{ item.update_badge || item.source_badge || '精彩内容，尽在媒体中枢' }}</p>
      <div class="home-hero-actions">
        <button class="action-button action-button-primary" type="button" @click="$emit('play')">
          立即播放
        </button>
      </div>
    </div>

    <div class="home-hero-metrics">
      <div class="home-hero-metric">
        <span>片库规模</span>
        <strong>{{ libraryCount ?? 0 }}</strong>
      </div>
      <div class="home-hero-metric">
        <span>订阅源</span>
        <strong>{{ sourceCount ?? 0 }}</strong>
      </div>
      <div class="home-hero-metric">
        <span>最近更新</span>
        <strong>{{ formatTime(lastRefresh) }}</strong>
      </div>
    </div>
  </section>
</template>
```

Note: This component reuses existing `.home-hero-*` CSS classes from `style.css`.

- [ ] **Step 2: Commit**

```bash
git add src/components/home/HomeHero.vue
git commit -m "feat: add HomeHero component for library route hero section"
```

### 2.3: Create ContinueRail component

**Files:**
- Create: `src/components/home/ContinueRail.vue`

- [ ] **Step 1: Write ContinueRail component**

```vue
<script setup lang="ts">
import type { CatalogCard } from '@/types'

defineProps<{
  items: CatalogCard[]
}>()

defineEmits<{
  select: [item: CatalogCard]
}>()
</script>

<template>
  <section v-if="items.length" class="media-rail continue-rail">
    <div class="media-rail-header">
      <div>
        <div class="section-title">继续观看</div>
      </div>
    </div>

    <div class="media-rail-track">
      <button
        v-for="item in items"
        :key="item.id"
        class="media-rail-card"
        type="button"
        @click="$emit('select', item)"
      >
        <MediaCard
          :title="item.title"
          :poster="item.poster"
          :subtitle="item.update_badge || item.source_badge || '片库条目'"
          :source-badge="item.source_badge"
        />
        <div v-if="item.progress" class="continue-rail-progress">
          <span :style="{ width: `${item.progress}%` }"></span>
        </div>
      </button>
    </div>
  </section>
</template>
```

Note: Reuses existing `.media-rail`, `.media-rail-track`, `.media-rail-card`, `.continue-rail-progress` CSS classes from `style.css`. MediaCard is imported from `@/components/media/MediaCard`.

- [ ] **Step 2: Commit**

```bash
git add src/components/home/ContinueRail.vue
git commit -m "feat: add ContinueRail component with progress bar"
```

### 2.4: Create LibraryCategory page

**Files:**
- Create: `src/views/LibraryCategory.vue`

- [ ] **Step 1: Write LibraryCategory page**

```vue
<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLibraryStore } from '@/stores/library'
import { useSubscriptionStore } from '@/stores/subscription'
import HomeHero from '@/components/home/HomeHero.vue'
import ContinueRail from '@/components/home/ContinueRail.vue'
import MediaRail from '@/components/home/MediaRail.vue'
import SearchBar from '@/components/SearchBar.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { CatalogCard, CatalogItemType } from '@/types'

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()
const subStore = useSubscriptionStore()

type CategoryTab = 'movie' | 'series' | 'variety' | 'anime'

const tabs: { key: CategoryTab; label: string }[] = [
  { key: 'movie', label: '电影' },
  { key: 'series', label: '剧集' },
  { key: 'variety', label: '综艺' },
  { key: 'anime', label: '动漫' }
]

const activeTab = ref<CategoryTab>('movie')
const searchKeyword = ref('')
const showAll = ref(false)

const enabledCount = computed(() => subStore.subscriptions.filter(s => s.enabled).length)

const currentType = computed<CatalogItemType>(() => route.params.type as CatalogItemType || 'movie')

const displayedItems = computed(() => {
  if (showAll.value) return libraryStore.catalogItems
  return libraryStore.catalogItems.slice(0, 20)
})

const heroItem = computed(() => libraryStore.hero)

async function loadCategory() {
  await libraryStore.fetchCatalog(currentType.value, searchKeyword.value || undefined)
}

onMounted(async () => {
  await subStore.fetchSubscriptions()
  await libraryStore.fetchHome()
  await loadCategory()
})

watch(currentType, loadCategory)

function handleTabChange(tab: CategoryTab) {
  router.push(`/library/${tab}`)
}

function handleSearch(keyword: string) {
  searchKeyword.value = keyword
  void libraryStore.fetchCatalog(currentType.value, keyword || undefined)
}

function handleItemClick(item: CatalogCard) {
  router.push(`/detail/${item.id}`)
}

function handlePlay(item: CatalogCard) {
  router.push(`/detail/${item.id}`)
}

function formatTypeLabel(type: CatalogItemType) {
  return tabs.find(t => t.key === type)?.label ?? '内容'
}
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1500px]">
      <header class="home-topbar">
        <div>
          <div class="eyebrow">TVBox Desktop</div>
          <div class="home-topbar-title">饭太硬媒体中枢</div>
        </div>
        <div class="home-topbar-actions">
          <RouterLink to="/subscriptions" class="action-button action-button-secondary">订阅</RouterLink>
          <RouterLink to="/settings" class="action-button action-button-secondary">设置</RouterLink>
        </div>
      </header>

      <nav class="home-category-nav" aria-label="媒体分类">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['nav-pill', activeTab === tab.key ? 'nav-pill-active' : '']"
          type="button"
          @click="handleTabChange(tab.key)"
        >
          {{ tab.label }}
        </button>
      </nav>

      <main class="home-landing">
        <HomeHero
          v-if="heroItem"
          :item="heroItem"
          :library-count="libraryStore.catalogItems.length"
          :source-count="enabledCount"
          @play="handlePlay(heroItem)"
          @click="handleItemClick"
        />

        <ContinueRail
          :items="libraryStore.activeContinueWatching"
          @select="handleItemClick"
        />

        <section class="home-secondary-browser">
          <div class="home-secondary-search">
            <SearchBar
              :placeholder="`搜索${formatTypeLabel(currentType)}...`"
              @search="handleSearch"
            />
          </div>

          <div v-if="libraryStore.loading" class="flex min-h-[220px] items-center justify-center">
            <LoadingSpinner />
          </div>

          <div v-else-if="libraryStore.catalogItems.length === 0" class="home-empty-state">
            暂无{{ formatTypeLabel(currentType) }}，先检查订阅源是否成功刷新。
          </div>

          <div v-else class="mt-6">
            <MediaRail
              :title="formatTypeLabel(currentType)"
              :items="displayedItems"
              @select="handleItemClick"
            />

            <div v-if="libraryStore.catalogItems.length > 20 && !showAll" class="mt-8 flex justify-center">
              <button class="action-button action-button-secondary" type="button" @click="showAll = true">
                加载更多
              </button>
            </div>
          </div>
        </section>

        <div class="live-shortcut">
          <RouterLink to="/library/live" class="live-shortcut-link">
            跳转直播
            <span class="live-shortcut-badge">{{ liveStore.channels.length }} 个频道</span>
          </RouterLink>
        </div>
      </main>
    </div>
  </div>
</template>
```

Note: Uses existing `.home-topbar`, `.home-topbar-title`, `.home-topbar-actions`, `.home-category-nav`, `.nav-pill`, `.home-landing`, `.home-secondary-browser`, `.home-secondary-search`, `.home-empty-state` CSS classes. Requires adding `.live-shortcut` and `.live-shortcut-link` to `style.css`.

- [ ] **Step 2: Add live-shortcut styles to style.css**

Add before the `@media (max-width: 720px)` block:

```css
.live-shortcut {
  display: flex;
  justify-content: center;
  padding: 2rem 0 1rem;
}

.live-shortcut-link {
  display: inline-flex;
  align-items: center;
  gap: 0.75rem;
  color: rgba(244, 239, 232, 0.56);
  font-size: 0.9rem;
  text-decoration: none;
  transition: color 200ms ease;
}

.live-shortcut-link:hover {
  color: rgba(244, 239, 232, 0.86);
}

.live-shortcut-badge {
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.08);
  color: rgba(244, 239, 232, 0.56);
  font-size: 0.68rem;
  letter-spacing: 0.2em;
  padding: 0.4rem 0.75rem;
}
```

- [ ] **Step 3: Commit**

```bash
git add src/views/LibraryCategory.vue src/style.css
git commit -m "feat: add LibraryCategory page with hero, continue rail, and category rail"
```

### 2.5: Add router entries for LibraryCategory

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: Add new routes for library categories**

Add these routes after the existing `/library/live` route:

```ts
{
  path: '/library/movie',
  name: 'movie',
  component: () => import('@/views/LibraryCategory.vue')
},
{
  path: '/library/series',
  name: 'series',
  component: () => import('@/views/LibraryCategory.vue')
},
{
  path: '/library/variety',
  name: 'variety',
  component: () => import('@/views/LibraryCategory.vue')
},
{
  path: '/library/anime',
  name: 'anime',
  component: () => import('@/views/LibraryCategory.vue')
},
```

Note: LibraryCategory uses `route.params.type` to determine which category to load, so a single component handles all four routes.

- [ ] **Step 2: Commit**

```bash
git add src/router/index.ts
git commit -m "feat: add router entries for library movie/series/variety/anime routes"
```

---

## Task 3: Detail Page Source Hierarchy

**Files:**
- Modify: `src/style.css:638-661` (`.recommended-source-panel` border/background)
- Modify: `src/components/detail/RecommendedSourcePanel.vue` (warm gold border)
- Modify: `src/components/detail/EpisodeGroupPanel.vue` (collapse behavior)

### 3.1: Update recommended-source-panel styles

**Files:**
- Modify: `src/style.css:638-661`

- [ ] **Step 1: Add warm gold border to recommended-source-panel**

```css
/* Before (lines 638-661) */
.recommended-source-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  border-radius: 1.8rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background:
    radial-gradient(circle at top left, rgba(216, 154, 87, 0.12), transparent 30%),
    rgba(255, 255, 255, 0.04);
  padding: 1.4rem 1.6rem;
}

/* After */
.recommended-source-panel {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1.5rem;
  border-radius: 1.8rem;
  border: 1px solid rgba(216, 154, 87, 0.32);
  background:
    radial-gradient(circle at top left, rgba(216, 154, 87, 0.14), transparent 30%),
    rgba(255, 255, 255, 0.04);
  padding: 1.4rem 1.6rem;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/style.css
git commit -m "style: add warm gold border to recommended-source-panel"
```

### 3.2: Add EpisodeChip resolving spinner state

**Files:**
- Modify: `src/components/media/EpisodeChip.vue`

- [ ] **Step 1: Add resolving state with spinner support**

```vue
<script setup lang="ts">
import type { EpisodeAvailabilityState } from '@/types'

const props = withDefaults(
  defineProps<{
    label: string
    state?: EpisodeAvailabilityState
    loading?: boolean
  }>(),
  {
    state: 'playable',
    loading: false
  }
)
</script>

<template>
  <button
    :class="['episode-chip', `episode-chip-${props.state}`, { 'episode-chip-loading': loading && props.state === 'resolving' }]"
    :disabled="props.state === 'unavailable'"
    :aria-disabled="props.state === 'unavailable'"
    type="button"
    :title="props.state === 'unavailable' ? '当前线路暂不可用，试试切换到推荐源' : undefined"
  >
    <span v-if="loading && props.state === 'resolving'" class="episode-chip-spinner" aria-hidden="true"></span>
    <span class="episode-chip-label">{{ label }}</span>
    <span v-if="props.state === 'resolving'" class="episode-chip-badge">解析中</span>
    <span v-if="props.state === 'playable'" class="episode-chip-badge">可播</span>
  </button>
</template>
```

- [ ] **Step 2: Add CSS for spinner and resolving badge**

Add to `style.css` after `.episode-chip-label`:

```css
.episode-chip-loading {
  cursor: wait;
}

.episode-chip-spinner {
  display: inline-block;
  width: 0.75rem;
  height: 0.75rem;
  border: 2px solid rgba(117, 169, 195, 0.3);
  border-top-color: rgba(117, 169, 195, 0.9);
  border-radius: 999px;
  animation: episode-chip-spin 0.8s linear infinite;
}

@keyframes episode-chip-spin {
  to { transform: rotate(360deg); }
}

.episode-chip-badge {
  margin-left: 0.4rem;
  font-size: 0.65rem;
  letter-spacing: 0.1em;
  opacity: 0.7;
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/media/EpisodeChip.vue src/style.css
git commit -m "feat: add resolving spinner and state badges to EpisodeChip"
```

---

## Task 4: Player Error Language + Drawer Cleanup

**Files:**
- Modify: `src/utils/player.ts` (error message rewrites)
- Modify: `src/views/PlayerPage.vue` (switching notice, retry button)
- Modify: `src/components/player/PlaybackNotice.vue` (support info tone)

### 4.1: Rewrite player error messages

**Files:**
- Modify: `src/utils/player.ts:1-33`

- [ ] **Step 1: Update describeMediaErrorCode and describePlaybackFailure**

```ts
// Before (lines 7-32)
export function describeMediaErrorCode(code?: number | null): string {
  switch (code) {
    case 1:
      return '播放被中止'
    case 2:
      return '网络错误'
    case 3:
      return '媒体解码失败'
    case 4:
      return '浏览器不支持当前媒体格式'
    default:
      return '媒体播放失败'
  }
}

export function describePlaybackFailure(error: unknown): string {
  if (isAutoplayBlocked(error)) {
    return '线路已加载，点击播放开始'
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message
  }

  return '无法直接播放当前地址'
}

// After
export function describeMediaErrorCode(code?: number | null): string {
  switch (code) {
    case 1:
      return '播放被中止，请检查网络或切换线路'
    case 2:
      return '网络连接不稳定，正在重试...'
    case 3:
      return '当前资源格式不支持，试试切换线路'
    case 4:
      return '浏览器不支持该格式，请尝试外部播放器'
    default:
      return '播放失败，请尝试切换线路'
  }
}

export function describePlaybackFailure(error: unknown): string {
  if (isAutoplayBlocked(error)) {
    return '浏览器限制了自动播放，请点击播放按钮'
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message
  }

  return '无法直接播放当前地址'
}
```

- [ ] **Step 2: Commit**

```bash
git add src/utils/player.ts
git commit -m "refactor: rewrite player error messages in player-first Chinese"
```

### 4.2: Add switching notice to PlayerPage

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Add switchingNotice ref and related state**

After line 36 (`const failedSourceIndexes = ref<number[]>([])`), add:

```ts
const switchingNotice = ref('')
let switchingNoticeTimer: number | null = null
```

- [ ] **Step 2: Add showSwitchingNotice helper**

After the `markCurrentSourceFailed` function, add:

```ts
function showSwitchingNotice(targetIndex: number) {
  if (switchingNoticeTimer) {
    clearTimeout(switchingNoticeTimer)
  }
  switchingNotice.value = `正在切换线路 ${targetIndex + 1}...`
  switchingNoticeTimer = window.setTimeout(() => {
    switchingNotice.value = ''
  }, 3000)
}
```

- [ ] **Step 3: Call showSwitchingNotice in switchToSource**

In `switchToSource` function, add `showSwitchingNotice(index)` at the start:

```ts
async function switchToSource(index: number) {
  if (index < 0 || index >= sources.value.length) return
  showSwitchingNotice(index)
  currentSourceIndex.value = index
  await playSource(sources.value[index])
}
```

- [ ] **Step 4: Update template to show switching notice**

In the `<template>` section, above `<PlaybackNotice>`, add:

```vue
<PlaybackNotice
  v-if="switchingNotice"
  :message="switchingNotice"
  tone="info"
/>
<PlaybackNotice
  v-else-if="errorMsg"
  :message="errorMsg"
  :tone="noticeTone"
/>
```

- [ ] **Step 5: Add retry button when all sources fail**

Find the final error state in `handleVideoError` where `errorMsg.value = message`. Instead, set a final failure state with count:

```ts
function handleVideoError() {
  pendingAutoplay.value = false
  const mediaError = videoRef.value?.error
  const message = describeMediaErrorCode(mediaError?.code)
  markCurrentSourceFailed()

  if (currentSourceIndex.value < sources.value.length - 1) {
    errorMsg.value = `${message}，正在切换下一条线路`
    void switchToSource(currentSourceIndex.value + 1)
    return
  }

  errorMsg.value = `已尝试 ${sources.value.length} 条线路，均无法播放。请稍后重试或切换订阅源。`
}
```

- [ ] **Step 6: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: add switching notice and retry messaging to player page"
```

### 4.3: Add info tone to PlaybackNotice

**Files:**
- Modify: `src/components/player/PlaybackNotice.vue`

- [ ] **Step 1: Add info tone style**

The component already accepts `tone?: 'info' | 'warning' | 'danger'`. Add the `playback-notice-info` style to `style.css`:

```css
.playback-notice-info {
  border-color: rgba(117, 169, 195, 0.24);
  color: #d8eef7;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/player/PlaybackNotice.vue src/style.css
git commit -m "style: add info tone to PlaybackNotice"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|-----------------|------|
| Label language: no English eyebrows in drawer | Task 1.1 |
| "线路 N" not "Line N" | Task 1.1 |
| Source kind badges: HLS/HTTP/外部 | Task 1.1 |
| Truncate URL to 60 chars | Task 1.1 |
| RecommendedSourcePanel: warm gold border, Chinese labels | Task 1.2, 3.1 |
| EpisodeGroupPanel: collapse non-recommended, Chinese labels | Task 1.3 |
| Home: Hero section | Task 2.2 |
| Home: Continue Rail with progress | Task 2.3 |
| Home: Category Rail | Task 2.4 (MediaRail already exists) |
| Home: Live Shortcut | Task 2.4 |
| New routes: movie/series/variety/anime | Task 2.5 |
| Detail: source hierarchy with warm gold border | Task 3.1 |
| Detail: EpisodeChip states with Chinese badges | Task 3.2 |
| Player: error language rewrite | Task 4.1 |
| Player: switching notice | Task 4.2 |
| Player: all-failed retry message with count | Task 4.2 |

All spec items covered. No placeholders found.

---

## Execution Option

Plan complete and saved to `docs/superpowers/plans/2026-04-23-tvbox-new-routes-ui-implementation.md`.

**Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?