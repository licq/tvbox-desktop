# 播放页面优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 全屏功能修复（使用 fullscreen.ts）+ 选集功能（Drawer Tab 切换）

**Architecture:** 两个独立的 UI 改动：(1) PlayerPage 的 fullscreen 改用 video-wrap 元素，(2) PlaybackDrawer 增加线路/选集 Tab，VOD 模式下 PlayerPage 加载 episodeGroups 并传给 Drawer。

**Tech Stack:** Vue 3 + Pinia + TypeScript + Tailwind CSS

---

## Task 1: 全屏功能修复

**Files:**
- Modify: `src/views/PlayerPage.vue:1-12` (imports)
- Modify: `src/views/PlayerPage.vue:25-36` (refs)
- Modify: `src/views/PlayerPage.vue:155-164` (toggleFullscreen function)
- Modify: `src/views/PlayerPage.vue:348` (template ref binding)

### 改动详解

**Step 1: 添加 import**

在 `PlayerPage.vue` 的 import 区块添加：

```typescript
import { useDetailStore } from '@/stores/detail'
```

在现有 import 之后（确认 `enterFullscreen` 和 `exitFullscreen` 已从 `@/utils/fullscreen` 引入，如果还没有就添加）：

```typescript
import { enterFullscreen, exitFullscreen } from '@/utils/fullscreen'
```

**Step 2: 添加 videoWrapRef**

在现有 `ref` 区块（大约第 25-36 行附近）添加：

```typescript
const videoWrapRef = ref<HTMLElement | null>(null)
```

**Step 3: 改写 toggleFullscreen 函数**

找到当前的 `toggleFullscreen` 函数（约在 155-164 行），替换为：

```typescript
async function toggleFullscreen() {
  if (!document.fullscreenElement) {
    await enterFullscreen(
      videoWrapRef.value,
      () => document.documentElement.requestFullscreen()
    )
    fullscreen.value = true
  } else {
    await exitFullscreen(document, () => document.exitFullscreen())
    fullscreen.value = false
  }
}
```

**Step 4: 绑定 videoWrapRef**

在 template 中找到 `.player-video-wrap` 的 div（约第 348 行），添加 ref：

```html
<div class="player-video-wrap" ref="videoWrapRef">
```

---

## Task 2: 选集功能 — PlaybackDrawer Tab + Episode Grid

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue` (props, emits, template)
- Create: CSS additions in `src/style.css` (or scoped `<style>` in PlaybackDrawer)

### 改动详解

**Step 1: 更新 import 和类型**

在 `PlaybackDrawer.vue` 的 `<script setup>` 中，确认已有 `SourceBadge` import（保持不变）。不需要引入新类型。

**Step 2: 更新 defineProps**

找到当前的 `defineProps`，替换为：

```typescript
defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
  episodes?: import('@/types').CatalogEpisode[]
  currentEpisodeId?: number
  activeTab?: 'sources' | 'episodes'
}>()
```

**Step 3: 更新 defineEmits**

找到当前的 `defineEmits`，追加新的 emit：

```typescript
defineEmits<{
  select: [index: number]
  selectEpisode: [episode: import('@/types').CatalogEpisode]
  tabChange: [tab: 'sources' | 'episodes']
}>()
```

**Step 4: 添加 innerTab ref**

在 `sourceTone` 函数之后、`</script>` 之前添加：

```typescript
const innerTab = ref<'sources' | 'episodes'>('sources')
```

**Step 5: 更新 template — 添加 Tab Bar**

在 `.playback-drawer-header` 之后、`.playback-drawer-copy` 之前添加：

```html
<div class="drawer-tabs">
  <button
    :class="{ active: innerTab === 'sources' }"
    @click="innerTab = 'sources'; $emit('tabChange', 'sources')"
  >线路</button>
  <button
    :class="{ active: innerTab === 'episodes' }"
    @click="innerTab = 'episodes'; $emit('tabChange', 'episodes')"
  >选集</button>
</div>
```

**Step 6: 更新 template — 条件渲染 Source/Episode 内容**

将 `.playback-source-list` 用 `<div v-if="innerTab === 'sources'">` 包裹：

```html
<div v-if="innerTab === 'sources' && sources.length" class="playback-source-list">
  <!-- existing v-for code -->
</div>

<div v-if="innerTab === 'sources' && !sources.length" class="playback-empty">
  没有解析出可展示线路。
</div>

<!-- Episode grid — new -->
<div v-if="innerTab === 'episodes' && episodes?.length" class="episode-grid">
  <button
    v-for="ep in episodes"
    :key="ep.id"
    :class="['episode-chip', ep.id === currentEpisodeId ? 'episode-chip-active' : '']"
    type="button"
    @click="$emit('selectEpisode', ep)"
  >
    {{ ep.episode_label }}
  </button>
</div>

<div v-else-if="innerTab === 'episodes'" class="playback-empty">
  当前无可用选集
</div>
```

**Step 7: 添加 CSS（scoped style）**

在 `PlaybackDrawer.vue` 的 `<style scoped>` 区块末尾添加：

```css
.drawer-tabs {
  display: flex;
  border-bottom: 1px solid var(--stroke);
  margin-bottom: 0.75rem;
}

.drawer-tabs button {
  flex: 1;
  padding: 0.5rem;
  font-size: 0.875rem;
  color: var(--text-muted);
  border-bottom: 2px solid transparent;
  background: none;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.drawer-tabs button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.episode-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 0.5rem;
  max-height: 280px;
  overflow-y: auto;
}

.episode-chip {
  padding: 0.5rem 0.25rem;
  font-size: 0.8125rem;
  border-radius: 0.375rem;
  background: var(--bg-elevated);
  border: 1px solid var(--stroke);
  color: var(--text);
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s;
}

.episode-chip:hover {
  border-color: var(--accent);
}

.episode-chip-active {
  background: var(--accent);
  color: #000;
  font-weight: 600;
  border-color: var(--accent);
}
```

---

## Task 3: 选集功能 — PlayerPage Episode 集成

**Files:**
- Modify: `src/views/PlayerPage.vue` (imports, refs, onMounted, new functions, template)

### 改动详解

**Step 1: 添加 import**

在 `PlayerPage.vue` script setup 顶部添加：

```typescript
import { useDetailStore } from '@/stores/detail'
import type { CatalogEpisode, CatalogEpisodeGroup } from '@/types'
```

**Step 2: 添加 store 和 refs**

在现有 `const` 声明区块添加：

```typescript
const detailStore = useDetailStore()
const activeGroup = ref<CatalogEpisodeGroup | null>(null)
```

**Step 3: 更新 onMounted**

找到 `onMounted` 的 VOD 分支，在 `playbackStore.resolve` 之后添加 episodeGroups 获取：

```typescript
} else if (mode.value === 'vod' && episodeUrl.value) {
  const url = decodeURIComponent(episodeUrl.value)
  const resolved = await playbackStore.resolve(url, episodeId.value)
  sources.value = resolved.candidates.map(candidate => ({
    url: candidate.url,
    label: candidate.label,
    kind: candidate.kind
  }))
  currentSourceIndex.value = 0

  if (itemId.value) {
    await detailStore.fetchDetail(itemId.value)
    const group = detailStore.episodeGroups.find(g =>
      g.episodes.some(e => e.id === episodeId.value)
    )
    activeGroup.value = group ?? null
  }

  if (resolved.status === 'ready' && sources.value.length > 0) {
    await playSource(sources.value[0])
  } else if (resolved.status === 'external_required' && sources.value.length > 0) {
    errorMsg.value = resolved.errorMessage ?? '当前资源需要外部处理'
    await playSource(sources.value[0])
  } else {
    errorMsg.value = resolved.errorMessage ?? '当前条目没有可用线路'
  }
}
```

**Step 4: 添加 switchToEpisode 函数**

在 `switchToSource` 函数之后添加：

```typescript
function switchToEpisode(episode: CatalogEpisode) {
  router.push(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`
  )
}
```

**Step 5: 更新 template — PlaybackDrawer props 和事件**

找到 `<PlaybackDrawer>` 组件（约第 416-423 行），替换为：

```html
<PlaybackDrawer
  :sources="sources"
  :current-index="currentSourceIndex"
  :failed-indexes="failedSourceIndexes"
  :status="playerStatusText"
  :error-message="errorMsg || playbackStore.errorMessage"
  :episodes="activeGroup?.episodes"
  :current-episode-id="episodeId"
  @select="switchToSource"
  @select-episode="switchToEpisode"
/>
```

---

## 验证计划

1. **全屏验证**：播放一个直播/HLS 视频，点击全屏按钮，确认视频 + vignette + controls 全屏，ESC 退出正常
2. **选集验证**：播放一个剧情/综艺，进入选集 Tab，确认集数列表显示，点击集数可切换播放
3. **Tab 切换验证**：在线路和选集 Tab 间切换，状态保持
4. **回退验证**：播放直播源，确认没有选集 Tab（episodes 为空时显示"当前无可用选集"）
