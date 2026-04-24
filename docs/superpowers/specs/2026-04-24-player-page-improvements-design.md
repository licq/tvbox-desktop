# 播放页面改进设计

日期: 2026-04-24

## 概述

修复播放页面的三个问题：控制按钮自动隐藏、选集点击、全屏按钮。

---

## Issue 1: 控制按钮自动隐藏

### 行为
- 播放时：鼠标静默 3 秒后自动隐藏 `.player-controls`
- 鼠标移入 `.player-video-wrap` 区域时显示控制栏
- 鼠标离开或用户操作（如点击、拖动进度条）后重新计时 3 秒
- 暂停状态：始终显示控制栏（不受计时器影响）

### 实现
- 添加 `controlsVisible` 响应式状态，默认为 `true`
- 添加 `hideTimer` 变量，播放时开始倒计时
- 使用 `@mouseenter` 和 `@mouseleave` 事件监听
- 使用 `watch` 监听 `playing` 状态，暂停时清除计时器并显示控制栏

---

## Issue 2: 选集功能点击

### 问题
- 当前使用 `router.push` 切换 episode，会污染浏览器历史
- 返回按钮会回到上一个 episode URL，而非详情页

### 解决方案
- 将 `router.push` 改为 `router.replace`
- 切换 episode 仅更新 URL 参数（`?episode=...&episodeId=...`），不生成新历史记录
- 用户按返回键会回到详情页，而非上一个 episode

### 实现
- 在 `switchToEpisode` 函数中使用 `router.replace` 代替 `router.push`

---

## Issue 3: 全屏按钮

### 行为
- 优先对 `<video>` 元素调用 `webkitEnterFullscreen()`（macOS Safari 原生视频全屏）
- 如果 `webkitEnterFullscreen` 不可用，导航到全屏专用页面

### 全屏专用页面 (`/player/fullscreen/:id`)
- 新建 `FullscreenPlayer.vue` 页面
- 使用 Tauri 窗口 API `setFullscreen(true)` 实现整页全屏
- 控制栏同样自动隐藏（参考 Issue 1 的实现）
- 退出全屏时返回到普通播放页面

### 实现
- `toggleFullscreen` 函数优先尝试 `video.webkitEnterFullscreen()`
- 失败时使用 `router.push('/player/fullscreen/...')` 导航到全屏页面
- `FullscreenPlayer.vue` 在 `onMounted` 调用 Tauri 全屏 API

---

## 组件变更

| 文件 | 变更 |
|------|------|
| `src/views/PlayerPage.vue` | 控制栏显示/隐藏逻辑、router.push → router.replace |
| `src/views/FullscreenPlayer.vue` | 新建，全屏播放页面 |
| `src/stores/playback.ts` | 无变更 |
| `src/components/player/PlaybackDrawer.vue` | 无变更 |

---

## 测试要点

1. 播放视频后 3 秒控制栏自动隐藏
2. 鼠标移入视频区域控制栏显示
3. 点击选集后 URL 替换，视频切换，返回键回到详情页
4. 全屏按钮点击后 video 元素全屏（macOS WKWebView）