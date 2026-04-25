# 播放页面改进实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复播放页面的三个问题：控制按钮自动隐藏、选集点击、全屏按钮

**Architecture:**
- Issue 1: 在 PlayerPage.vue 中添加 `controlsVisible` 状态和 hideTimer，实现播放时控制栏 3 秒无操作自动隐藏
- Issue 2: 将 `switchToEpisode` 中的 `router.push` 改为 `router.replace`，避免污染浏览器历史
- Issue 3: 全屏优先尝试 video.webkitEnterFullscreen()，失败时导航到 FullscreenPlayer.vue 全屏页面

**Tech Stack:** Vue 3, TypeScript, Vue Router, Tauri Window API

---

## Task 1: 控制按钮自动隐藏

**Files:**
- Modify: `src/views/PlayerPage.vue:1-520`

**Change:** 在 PlayerPage.vue 中添加控制栏显示/隐藏逻辑

- [ ] **Step 1: 添加 `controlsVisible` 状态和 `hideTimer` 变量**

在 `onUnmounted` 之后、`let hlsInstance` 之前添加：

```typescript
const controlsVisible = ref(true)
let hideTimer: number | null = null

function startHideTimer() {
  if (hideTimer) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
  if (playing.value) {
    hideTimer = window.setTimeout(() => {
      controlsVisible.value = false
    }, 3000)
  }
}

function showControls() {
  controlsVisible.value = true
  startHideTimer()
}

function handleUserInteraction() {
  showControls()
}
```

- [ ] **Step 2: 修改 `handleVideoPlay` 函数，添加计时器**

将 `handleVideoPlay` 函数改为：

```typescript
function handleVideoPlay() {
  playing.value = true
  errorMsg.value = ''
  showControls()
}
```

- [ ] **Step 3: 修改 `handleVideoPause` 函数，停止计时器并显示控制栏**

将 `handleVideoPause` 函数改为：

```typescript
function handleVideoPause() {
  playing.value = false
  if (hideTimer) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
  controlsVisible.value = true
}
```

- [ ] **Step 4: 在 `togglePlay` 和 `seek` 函数中添加用户交互处理**

在 `togglePlay` 函数末尾添加：

```typescript
function togglePlay() {
  if (!videoRef.value) return

  if (playing.value) {
    videoRef.value.pause()
    playing.value = false
    return
  }

  void attemptPlayback(true)
  handleUserInteraction()
}
```

在 `seek` 函数中添加：

```typescript
function seek(time: number) {
  if (!videoRef.value) return
  videoRef.value.currentTime = time
  handleUserInteraction()
}
```

- [ ] **Step 5: 在模板中添加鼠标事件和条件类**

找到 `.player-controls` 所在 div（第 461 行附近），改为：

```html
<div
  class="player-controls"
  :class="{ 'controls-hidden': !controlsVisible }"
  @mouseenter="showControls"
  @mouseleave="startHideTimer"
>
```

- [ ] **Step 6: 在 `onUnmounted` 中清除计时器**

在 `onUnmounted` 函数中添加：

```typescript
if (hideTimer) {
  window.clearTimeout(hideTimer)
}
```

- [ ] **Step 7: 添加 CSS 样式**

在 `src/style.css` 或对应的样式文件中添加：

```css
.player-controls.controls-hidden {
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.3s ease;
}

.player-controls {
  transition: opacity 0.3s ease;
}
```

- [ ] **Step 8: 运行测试验证**

Run: `npm run build`
Expected: TypeScript 编译无错误

---

## Task 2: 选集功能 router.push → router.replace

**Files:**
- Modify: `src/views/PlayerPage.vue:290-294`

**Change:** 将 `switchToEpisode` 函数中的 `router.push` 改为 `router.replace`

- [ ] **Step 1: 修改 `switchToEpisode` 函数**

将：

```typescript
function switchToEpisode(episode: CatalogEpisode) {
  router.push(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`
  )
}
```

改为：

```typescript
function switchToEpisode(episode: CatalogEpisode) {
  router.replace(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`
  )
}
```

- [ ] **Step 2: 运行测试验证**

Run: `npm run build`
Expected: TypeScript 编译无错误

---

## Task 3: 全屏按钮 - webkitEnterFullscreen 优先 + Fallback 页面

**Files:**
- Modify: `src/views/PlayerPage.vue:179-244`
- Create: `src/views/FullscreenPlayer.vue`
- Modify: `src/router/index.ts` (如果需要添加路由)

**Change:** 简化全屏逻辑，优先使用 video.webkitEnterFullscreen()，失败时导航到全屏页面

- [ ] **Step 1: 简化 `toggleFullscreen` 函数**

将 `toggleFullscreen` 函数改为：

```typescript
async function toggleFullscreen() {
  const video = videoRef.value
  if (!video) return

  // 已在全屏状态 → 退出
  if (document.fullscreenElement) {
    try {
      await document.exitFullscreen()
    } catch (e) {
      console.error('[fullscreen] exit error:', e)
    }
    fullscreen.value = false
    fullscreenError.value = ''
    return
  }

  // 进入全屏 - 优先 webkitEnterFullscreen
  if (typeof (video as any).webkitEnterFullscreen === 'function') {
    console.log('[fullscreen] trying webkitEnterFullscreen')
    try {
      ;(video as any).webkitEnterFullscreen()
      fullscreen.value = true
      fullscreenError.value = ''
      console.log('[fullscreen] webkitEnterFullscreen ok')
      return
    } catch (e) {
      console.error('[fullscreen] webkitEnterFullscreen error:', e)
    }
  }

  // Fallback: 导航到全屏页面
  console.log('[fullscreen] navigating to fullscreen page')
  const episodeParam = episodeUrl.value
    ? `?episode=${encodeURIComponent(episodeUrl.value)}&episodeId=${episodeId.value ?? ''}`
    : ''
  router.push(`/player/fullscreen/${itemId.value}${episodeParam}`)
}
```

- [ ] **Step 2: 创建 FullscreenPlayer.vue**

创建 `src/views/FullscreenPlayer.vue`：

```vue
<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { usePlaybackStore } from '@/stores/playback'
import { useDetailStore } from '@/stores/detail'
import type Hls from 'hls.js'

const route = useRoute()
const router = useRouter()
const playbackStore = usePlaybackStore()
const detailStore = useDetailStore()

const videoRef = ref<HTMLVideoElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const controlsVisible = ref(true)
const errorMsg = ref('')

let hlsInstance: Hls | null = null
let hlsConstructorPromise: Promise<typeof import('hls.js').default> | null = null
let progressUpdateInterval: number | null = null
let hideTimer: number | null = null

const episodeUrl = computed(() => {
  const value = route.query.episode
  return typeof value === 'string' ? value : null
})
const episodeId = computed(() => {
  const value = route.query.episodeId
  const numeric = typeof value === 'string' ? Number(value) : NaN
  return Number.isFinite(numeric) && numeric > 0 ? numeric : undefined
})
const itemId = computed(() => Number(route.params.id))

onMounted(async () => {
  const win = getCurrentWindow()
  await win.setFullscreen(true)

  if (episodeUrl.value) {
    const url = decodeURIComponent(episodeUrl.value)
    const resolved = await playbackStore.resolve(url, episodeId.value)
    if (resolved.status === 'ready' && resolved.candidates.length > 0) {
      await playSource(resolved.candidates[0])
    }
  }

  progressUpdateInterval = window.setInterval(() => {
    if (!videoRef.value) return
    currentTime.value = videoRef.value.currentTime
    duration.value = videoRef.value.duration || 0
  }, 1000)

  startHideTimer()
})

onUnmounted(() => {
  if (progressUpdateInterval) {
    window.clearInterval(progressUpdateInterval)
  }
  if (hideTimer) {
    window.clearTimeout(hideTimer)
  }
  if (hlsInstance) {
    hlsInstance.destroy()
  }
  const win = getCurrentWindow()
  void win.setFullscreen(false)
})

function startHideTimer() {
  if (hideTimer) {
    window.clearTimeout(hideTimer)
  }
  if (playing.value) {
    hideTimer = window.setTimeout(() => {
      controlsVisible.value = false
    }, 3000)
  }
}

function showControls() {
  controlsVisible.value = true
  startHideTimer()
}

async function playSource(candidate: { url: string; label: string; kind: string }) {
  if (!videoRef.value) return
  const url = candidate.url

  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  if (url.includes('.m3u8')) {
    const Hls = await getHlsConstructor()
    if (Hls.isSupported()) {
      const hls = new Hls()
      hlsInstance = hls
      hls.loadSource(url)
      hls.attachMedia(videoRef.value)
      hls.on(Hls.Events.ERROR, () => {
        errorMsg.value = '播放错误'
      })
      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        void videoRef.value?.play()
        playing.value = true
      })
      return
    }
  }

  videoRef.value.src = url
  videoRef.value.load()
  void videoRef.value.play()
  playing.value = true
}

async function getHlsConstructor() {
  if (!hlsConstructorPromise) {
    hlsConstructorPromise = import('hls.js').then(module => module.default)
  }
  return hlsConstructorPromise
}

function formatTime(seconds: number): string {
  const m = Math.floor(seconds / 60)
  const s = Math.floor(seconds % 60)
  return `${m}:${s.toString().padStart(2, '0')}`
}

function handleExitFullscreen() {
  const win = getCurrentWindow()
  void win.setFullscreen(false)
  router.back()
}
</script>

<template>
  <div class="fullscreen-player">
    <video
      ref="videoRef"
      class="fullscreen-video"
      playsinline
      @click="playing = !playing; showControls()"
      @play="playing = true; startHideTimer()"
      @pause="playing = false; controlsVisible = true"
    ></video>

    <div
      class="fullscreen-controls"
      :class="{ 'controls-hidden': !controlsVisible }"
      @mouseenter="showControls"
    >
      <button type="button" @click="handleExitFullscreen">退出全屏</button>
      <span>{{ formatTime(currentTime) }} / {{ formatTime(duration) }}</span>
    </div>
  </div>
</template>

<style scoped>
.fullscreen-player {
  position: fixed;
  inset: 0;
  background: #000;
  z-index: 9999;
}

.fullscreen-video {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.fullscreen-controls {
  position: absolute;
  bottom: 0;
  left: 0;
  right: 0;
  padding: 1rem;
  background: linear-gradient(transparent, rgba(0,0,0,0.8));
  display: flex;
  align-items: center;
  gap: 1rem;
  transition: opacity 0.3s ease;
}

.fullscreen-controls.controls-hidden {
  opacity: 0;
  pointer-events: none;
}
</style>
```

- [ ] **Step 3: 添加路由**

在 `src/router/index.ts` 中添加新路由（如果尚不存在）：

```typescript
{
  path: '/player/fullscreen/:id',
  name: 'fullscreen-player',
  component: () => import('@/views/FullscreenPlayer.vue')
}
```

- [ ] **Step 4: 运行测试验证**

Run: `npm run build`
Expected: TypeScript 编译无错误

---

## Task 4: 完整测试

**Files:**
- 修改: `src/views/PlayerPage.vue`
- 创建: `src/views/FullscreenPlayer.vue`
- 修改: `src/router/index.ts`

- [ ] **Step 1: 运行构建测试**

Run: `npm run build`
Expected: 编译成功，无错误

- [ ] **Step 2: 运行类型检查**

Run: `npm run tauri build` (或 `npm run tauri dev` 测试)
Expected: 无编译错误

---

## 自检清单

- [ ] Issue 1: 播放视频后 3 秒控制栏自动隐藏 ✓
- [ ] Issue 1: 鼠标移入视频区域控制栏显示 ✓
- [ ] Issue 1: 暂停时控制栏始终显示 ✓
- [ ] Issue 2: router.replace 已替换 router.push ✓
- [ ] Issue 3: webkitEnterFullscreen 优先 ✓
- [ ] Issue 3: Fallback 全屏页面已创建 ✓