# UI 优化设计

日期: 2026-04-24

## 概述

统一应用界面设计，删除孤儿代码，提升整体一致性。

---

## Issue 1: 统一旧页面样式

### 现状
- `Live.vue`, `Vod.vue`, `Settings.vue` 使用内联 Tailwind 类（如 `bg-gray-900`, `p-4`）
- 与新设计系统（`.app-shell`, `.surface-panel`, `.media-card`）不一致

### 目标
重构这三个页面使用统一的设计系统：

| 页面 | 路由 | 重构目标 |
|------|------|----------|
| `Live.vue` | `/live` → `/library/live` | 使用 `.app-shell`, `.surface-panel` 布局 |
| `Vod.vue` | `/vod` → `/library/movie` | 同上 |
| `Settings.vue` | `/settings` | 同上 |

### 实现
- 将内联 Tailwind 类替换为设计系统组件类
- 使用 `.app-shell` 作为页面容器
- 使用 `.surface-panel` / `.surface-muted` 作为面板背景
- 保持功能不变，仅改样式

---

## Issue 2: 删除孤儿代码

### 删除的文件
- `src/views/Live.vue` — 路由已重定向到 `/library/live`
- `src/views/Vod.vue` — 路由已重定向到 `/library/movie`
- `src/views/FullscreenPlayer.vue` — CSS 全屏已替代原实现

### 影响
- 路由保持不变（已是重定向）
- `FullscreenPlayer.vue` 从未被主动导航（PlayerPage 使用 CSS 全屏）

---

## 变更清单

| 文件 | 操作 |
|------|------|
| `src/views/Live.vue` | 删除 |
| `src/views/Vod.vue` | 删除 |
| `src/views/FullscreenPlayer.vue` | 删除 |
| `src/views/Settings.vue` | 重构样式 |
| `src/views/Live.vue` | 重构样式（如需保留） |
| `src/views/Vod.vue` | 重构样式（如需保留） |
| `src/router/index.ts` | 移除孤儿路由（如有） |
| `src/style.css` | 无变更 |
| `src/components/` | 无变更 |

---

## 测试要点

1. `/live` 和 `/vod` 仍然重定向到正确的页面
2. `/settings` 页面样式与其他页面一致
3. 全屏功能仍然正常工作（不依赖 FullscreenPlayer.vue）