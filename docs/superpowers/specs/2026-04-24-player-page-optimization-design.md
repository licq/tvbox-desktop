# TVBox 播放页面优化设计

Date: 2026-04-24
Status: Approved

## 变更 1：全屏功能修复

### 问题

`toggleFullscreen()` 使用 `document.documentElement.requestFullscreen()` 将整个页面全屏，导致视频区域无法获得独立的控制能力，overlay（vignette、controls）表现异常。

### 解决方案

使用已有的 `src/utils/fullscreen.ts` 工具函数，将 `.player-video-wrap` 元素作为全屏目标。

**改动文件**：`src/views/PlayerPage.vue`

```typescript
// 引入
import { enterFullscreen, exitFullscreen } from '@/utils/fullscreen'

// 改动 toggleFullscreen
const videoWrapRef = ref<HTMLElement | null>(null)

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

```html
<!-- template 中绑定 ref -->
<div class="player-video-wrap" ref="videoWrapRef">
```

**效果**：
- 视频 + vignette + controls 整体全屏
- drawer 在全屏时保持可见（被视频区域遮挡，按 ESC 可恢复）
- 监听 `document.fullscreenElement` 变化自动同步 `fullscreen.value`（如果已有监听则跳过）

---

## 变更 2：选集功能

### 问题

播放页面对于剧情/综艺只有单集 URL，无法切换集数。用户需要返回详情页才能选择其他剧集。

### 解决方案

在 `PlaybackDrawer` 中增加「线路」/「选集」Tab 切换。在 VOD 模式下，PlayerPage 加载 `detailStore.episodeGroups`，用户可直接在播放页切换集数。

### 数据流

```
VodDetailPage
  └─> router.push('/player/vod/{itemId}?episode=url&episodeId=id')

PlayerPage (onMounted for VOD)
  └─> detailStore.fetchDetail(itemId)
        └─> detailStore.episodeGroups[]
              └─> CatalogEpisodeGroup { source_name, episodes[] }

Drawer (选集 Tab)
  └─> 当前 group 的 episodes 网格
        └─> 点击集数 -> router.push 切换 episode
```

### 改动文件

**`src/views/PlayerPage.vue`**：

```typescript
import { useDetailStore } from '@/stores/detail'

const detailStore = useDetailStore()
const activeGroup = ref<CatalogEpisodeGroup | null>(null)

onMounted(async () => {
  if (mode.value === 'vod' && itemId.value) {
    await detailStore.fetchDetail(itemId.value)
    const group = detailStore.episodeGroups.find(g =>
      g.episodes.some(e => e.id === episodeId.value)
    )
    activeGroup.value = group ?? null
  }
})

function switchToEpisode(episode: CatalogEpisode) {
  router.push(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`
  )
}
```

**`src/components/player/PlaybackDrawer.vue`**：

```typescript
// 新增 props
defineProps<{
  // ... existing
  episodes?: CatalogEpisode[]
  currentEpisodeId?: number
  activeTab?: 'sources' | 'episodes'
}>()

// 新增 emits
defineEmits<{
  selectSource: [index: number]
  selectEpisode: [episode: CatalogEpisode]
  tabChange: [tab: 'sources' | 'episodes']
}>()

// Tab 内部状态（可由外部控制或内部管理）
const innerTab = ref<'sources' | 'episodes'>('sources')
```

```html
<!-- Drawer 模板改动 -->
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

<!-- 选集 Tab 内容 -->
<div v-if="innerTab === 'episodes' && episodes?.length" class="episode-grid">
  <button
    v-for="ep in episodes"
    :key="ep.id"
    :class="['episode-chip', ep.id === currentEpisodeId ? 'episode-chip-active' : '']"
    @click="$emit('selectEpisode', ep)"
  >
    {{ ep.episode_label }}
  </button>
</div>

<div v-else-if="innerTab === 'episodes'" class="playback-empty">
  当前无可用选集
</div>
```

**CSS 样式**（添加到 `style.css`）：

```css
.drawer-tabs {
  display: flex;
  gap: 0;
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

## 实现顺序

1. **全屏修复**（PlayerPage.vue — toggleFullscreen 改动，引入 fullscreen.ts）
2. **选集功能**（PlayerPage.vue — detailStore.fetchDetail + activeGroup；PlaybackDrawer.vue — Tab + episode grid + emits）

## 验收标准

- 全屏时视频区域（video + vignette + controls）独立全屏，不受页面其他元素干扰
- 选集 Tab 在 VOD 模式下可用，点击集数切换播放
- Tab 切换状态正确，episodes 为空时显示"当前无可用选集"
- 不影响现有直播模式的播放功能
