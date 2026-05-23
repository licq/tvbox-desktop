# 播放页右边栏无 Tab 改造实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 PlaybackDrawer 从双标签页改为上下分区布局（剧集/来源 + 链路信息固定底部）

**Architecture:** 仅修改 PlaybackDrawer.vue 的模板/逻辑/样式，删除标签页切换代码，新增 PlaybackHeader、EpisodeSection（根据 itemType 条件渲染剧集网格或来源列表）、LinkInfoPanel（固定底部，含线路切换、URL 复制、错误显示）。PlayerPage 新增一个 `itemType` prop 传入。

**Tech Stack:** Vue 3 `<script setup>` + CSS Variables + Clipboard API

---

### 涉及文件

- **Modify:** `src/components/player/PlaybackDrawer.vue` — 完整重写
- **Modify:** `src/views/PlayerPage.vue` — 新增 `:item-type` prop

---

### Task 1: 重写 PlaybackDrawer.vue — 结构和 Props

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue`

删除标签页相关代码，改为新的上下分区布局。

- [ ] **Step 1: 更新 script 部分**

移除 `innerTab`、`activeTab` prop、`tabChange` emit。更新 props 和 emits：

```vue
<script setup lang="ts">
import { ref } from 'vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { PlayerSource, UnifiedEpisode } from '@/types'

const props = defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
  unifiedEpisodes?: UnifiedEpisode[]
  currentNormalizedIndex?: number
  itemType: string
}>()

const emit = defineEmits<{
  selectEpisode: [unifiedEpisode: UnifiedEpisode]
  switchLine: [index: number]
}>()

const copied = ref(false)

function sourceTone(kind: PlayerSource['kind']) {
  if (kind === 'external' || kind === 'embed') return 'danger'
  if (kind === 'hls') return 'cool'
  return 'neutral'
}

async function copyUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    // Clipboard API not available — silently fail
  }
}

const isSeries = computed(() =>
  props.itemType === 'series' || props.itemType === 'variety' || props.itemType === 'anime'
)
</script>
```

Add the missing `computed` import:
```vue
import { ref, computed } from 'vue'
```

- [ ] **Step 2: 写模板 — 整体布局**

替换整个 `<template>`，删除 tab 结构，用 flex column 布局固定底部的链路面板：

```vue
<template>
  <aside class="playback-drawer">
    <!-- PlaybackHeader -->
    <div class="playback-header">
      <div class="playback-header-title">
        <template v-if="isSeries && currentNormalizedIndex !== undefined && unifiedEpisodes?.length">
          <span class="eyebrow">正在播放</span>
          <h2>{{ unifiedEpisodes.find(e => e.normalizedIndex === currentNormalizedIndex)?.displayLabel || '选集' }}</h2>
        </template>
        <template v-else>
          <span class="eyebrow">播放线路</span>
          <h2>{{ sources[currentIndex]?.label || '选择线路' }}</h2>
        </template>
      </div>
      <SourceBadge :label="status" :tone="status === 'failed' ? 'danger' : 'warm'" />
    </div>

    <!-- EpisodeSection (scrollable) -->
    <div class="episode-section">
      <!-- Series mode: episode grid -->
      <div v-if="isSeries && unifiedEpisodes?.length" class="episode-grid">
        <button
          v-for="ue in unifiedEpisodes"
          :key="ue.normalizedIndex"
          :class="[
            'episode-chip',
            ue.normalizedIndex === currentNormalizedIndex ? 'episode-chip-active' : ''
          ]"
          type="button"
          @click="emit('selectEpisode', ue)"
        >
          {{ ue.displayLabel }}
          <span v-if="ue.sources.length > 1" class="source-count-badge">{{ ue.sources.length }}源</span>
        </button>
      </div>

      <!-- Movie mode: source list -->
      <div v-else-if="sources.length" class="source-list">
        <button
          v-for="(source, index) in sources"
          :key="`${source.url}-${index}`"
          :class="[
            'source-row',
            index === currentIndex ? 'source-row-active' : '',
            failedIndexes.includes(index) ? 'source-row-failed' : ''
          ]"
          type="button"
          @click="emit('switchLine', index)"
        >
          <span class="source-row-label">{{ source.label }}</span>
          <SourceBadge :label="source.kind" :tone="sourceTone(source.kind)" />
        </button>
      </div>

      <!-- Empty state -->
      <div v-else class="playback-empty">没有可用线路</div>
    </div>

    <!-- LinkInfoPanel (fixed bottom) -->
    <div class="link-info-panel">
      <!-- LineSwitcher -->
      <div v-if="sources.length > 1" class="line-switcher">
        <button
          v-for="(source, index) in sources"
          :key="index"
          :class="[
            'line-btn',
            index === currentIndex ? 'line-btn-active' : '',
            failedIndexes.includes(index) ? 'line-btn-failed' : ''
          ]"
          type="button"
          @click="emit('switchLine', index)"
        >
          线路{{ index + 1 }}
        </button>
      </div>

      <!-- UrlDisplay -->
      <div
        v-if="sources[currentIndex]?.url"
        :class="['url-display', { 'url-display-copied': copied }]"
        @click="copyUrl(sources[currentIndex].url)"
        title="点击复制 URL"
      >
        <span class="url-text">{{ sources[currentIndex].url }}</span>
        <span class="url-copy-hint">{{ copied ? '✓ 已复制' : '复制' }}</span>
      </div>

      <!-- ErrorDisplay -->
      <div v-if="errorMessage" class="error-display">
        {{ errorMessage }}
      </div>
    </div>
  </aside>
</template>
```

- [ ] **Step 3: 写样式 — 删除旧的 tab 样式，添加新布局样式**

替换整个 `<style scoped>` 块：

```css
.playback-drawer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.playback-empty {
  padding: 1rem;
  color: var(--text-muted);
  text-align: center;
  font-size: 0.875rem;
}

/* PlaybackHeader */
.playback-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem;
  border-bottom: 1px solid var(--stroke);
}

.playback-header-title .eyebrow {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 0.15rem;
}

.playback-header-title h2 {
  font-size: 0.95rem;
  font-weight: 600;
  margin: 0;
  line-height: 1.3;
}

/* EpisodeSection — scrollable */
.episode-section {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 0.75rem;
}

/* Episode grid (series/variety/anime) */
.episode-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 0.5rem;
}

.episode-chip {
  aspect-ratio: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  font-size: 0.8125rem;
  border-radius: 0.375rem;
  background: var(--bg-elevated);
  border: 1px solid var(--stroke);
  color: var(--text);
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s;
  position: relative;
  line-height: 1.2;
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

.source-count-badge {
  position: absolute;
  top: -4px;
  right: -4px;
  font-size: 0.6rem;
  background: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.05rem 0.3rem;
  border-radius: 0.2rem;
  line-height: 1.3;
}

/* Source list (movie mode) */
.source-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.source-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.65rem 0.75rem;
  border-radius: 0.5rem;
  background: transparent;
  border: 1px solid var(--stroke);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  transition: background 0.15s, border-color 0.15s;
}

.source-row:hover {
  border-color: var(--accent);
}

.source-row-active {
  background: var(--accent);
  color: #000;
  border-color: var(--accent);
  font-weight: 600;
}

.source-row-failed {
  border-color: var(--danger);
}

.source-row-label {
  font-weight: 500;
  font-size: 0.875rem;
}

/* LinkInfoPanel — fixed bottom */
.link-info-panel {
  border-top: 1px solid var(--stroke);
  padding: 0.75rem;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* LineSwitcher */
.line-switcher {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.line-btn {
  flex: 1;
  min-width: 55px;
  padding: 0.3rem 0.5rem;
  border-radius: 0.375rem;
  border: 1px solid var(--stroke);
  background: transparent;
  color: var(--text);
  font-size: 0.75rem;
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.line-btn:hover {
  border-color: var(--accent);
}

.line-btn-active {
  background: var(--accent);
  color: #000;
  font-weight: 500;
  border-color: var(--accent);
}

.line-btn-failed {
  border-color: var(--danger);
  color: var(--danger);
}

/* UrlDisplay */
.url-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.35rem;
  padding: 0.35rem 0.5rem;
  border-radius: 0.25rem;
  border: 1px solid var(--stroke);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  font-size: 0.7rem;
  font-family: monospace;
}

.url-display:hover {
  border-color: var(--accent);
  background: rgba(255, 255, 255, 0.03);
}

.url-display-copied {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}

.url-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
  min-width: 0;
}

.url-copy-hint {
  flex-shrink: 0;
  font-size: 0.65rem;
  color: var(--text-muted);
  white-space: nowrap;
}

.url-display-copied .url-copy-hint {
  color: var(--accent);
}

/* ErrorDisplay */
.error-display {
  font-size: 0.75rem;
  color: var(--danger);
  padding: 0.4rem 0.5rem;
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  border-radius: 0.25rem;
  line-height: 1.4;
}
```

- [ ] **Step 4: 验证完整文件结构正确**

确认以下移除：
- `activeTab` prop
- `tabChange` emit
- `innerTab` ref
- `.drawer-tabs` 样式

确认以下新增：
- `itemType` prop
- `selectEpisode` / `switchLine` emits
- `copied` ref + `copyUrl()` 方法
- `isSeries` computed
- `.playback-header` / `.episode-section` / `.link-info-panel` CSS 类

- [ ] **Step 5: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue
git commit -m "feat: rewrite PlaybackDrawer with tabless sidebar layout
- Remove source/episodes tab switching
- Add top section: episode grid (series) or source list (movie)
- Add fixed-bottom LinkInfoPanel with line switcher, URL copy, error display
- Add itemType prop for conditional rendering
- Click URL to copy to clipboard with 2s feedback"
```

---

### Task 2: 更新 PlayerPage.vue — 传递 itemType 和新 emits

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: 新增 itemType 计算属性**

在 `playUnifiedEpisode` 函数之前（约第 483 行），添加：

```ts
const itemType = computed(() => detailStore.item?.item_type ?? 'movie')
```

需要在 import 中确认 `computed` 已导入（第 2 行已有 `import { computed, ... }`）。

- [ ] **Step 2: 更新 PlaybackDrawer 模板绑定**

找到 `<PlaybackDrawer>` 模板（约第 789-799 行），更新为：

```vue
        <PlaybackDrawer
          :sources="sources"
          :current-index="currentSourceIndex"
          :failed-indexes="failedSourceIndexes"
          :status="playerStatusText"
          :error-message="errorMsg || playbackStore.errorMessage"
          :unified-episodes="unifiedEpisodes"
          :current-normalized-index="currentNormalizedIndex"
          :item-type="itemType"
          @select-episode="switchToEpisode"
          @switch-line="switchToSource"
        />
```

注意 emit 名称变化：
- `@select` → `@switch-line`
- `@select-unified-episode` → `@select-episode`

- [ ] **Step 3: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: pass itemType to PlaybackDrawer, update emit bindings"
```
