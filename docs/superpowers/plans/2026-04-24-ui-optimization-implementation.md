# UI 优化实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一应用界面设计，删除孤儿代码

**Architecture:**
- 删除孤儿页面 Live.vue, Vod.vue, FullscreenPlayer.vue
- 重构 Settings.vue 使用设计系统组件类
- 清理 router/index.ts 中未使用的路由

**Tech Stack:** Vue 3, Tailwind CSS, Tauri

---

## Task 1: 删除 Live.vue 和 Vod.vue

**Files:**
- Delete: `src/views/Live.vue`
- Delete: `src/views/Vod.vue`

- [ ] **Step 1: 验证 Live.vue 和 Vod.vue 未被 router 引用**

检查 `src/router/index.ts`：
- `/live` 和 `/vod` 路由使用 `redirect`，不引用任何组件
- Live.vue 和 Vod.vue 不在 import 语句中

Run: `grep -n "Live.vue\|Vod.vue" src/router/index.ts`
Expected: 无结果

- [ ] **Step 2: 删除 Live.vue**

Run: `rm src/views/Live.vue`

- [ ] **Step 3: 删除 Vod.vue**

Run: `rm src/views/Vod.vue`

- [ ] **Step 4: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 2: 删除 FullscreenPlayer.vue

**Files:**
- Delete: `src/views/FullscreenPlayer.vue`
- Modify: `src/router/index.ts:34-37`

- [ ] **Step 1: 验证 FullscreenPlayer.vue 未被使用**

检查 `src/views/PlayerPage.vue` 中的全屏实现：
- PlayerPage 使用 CSS 全屏（position: fixed）
- 不引用 FullscreenPlayer.vue

Run: `grep -n "FullscreenPlayer" src/views/PlayerPage.vue`
Expected: 无结果

- [ ] **Step 2: 删除 FullscreenPlayer.vue**

Run: `rm src/views/FullscreenPlayer.vue`

- [ ] **Step 3: 从 router 中移除 FullscreenPlayer 路由**

将 `src/router/index.ts` 中的：
```typescript
{
  path: '/player/fullscreen/:id',
  name: 'fullscreen-player',
  component: () => import('@/views/FullscreenPlayer.vue')
}
```

改为：
```typescript
// FullscreenPlayer 已删除，使用 CSS 全屏
```

- [ ] **Step 4: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 3: 重构 Settings.vue 使用设计系统

**Files:**
- Modify: `src/views/Settings.vue`

**当前样式：**
```html
<div class="settings-page min-h-screen bg-gray-900 text-white p-4">
```

**目标样式：**
```html
<div class="app-shell">
```

**修改步骤：**

- [ ] **Step 1: 读取当前 Settings.vue 内容**

- [ ] **Step 2: 将页面容器从 `min-h-screen bg-gray-900 text-white p-4` 改为 `app-shell`**

- [ ] **Step 3: 将 `bg-gray-800 p-4 rounded-lg` 替换为 `surface-panel`**

- [ ] **Step 4: 将 `bg-gray-700` 表单元素保持或替换为统一风格**

- [ ] **Step 5: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 4: 清理 router/index.ts 中的冗余路由

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: 检查并删除冗余路由**

当前 router 中存在问题：
1. `/live` → redirect，但 Live.vue 已删除
2. `/vod` → redirect，但 Vod.vue 已删除
3. `/library/live` 和 `/library/:type` 使用同一个 Home 组件（重复）

简化后的路由：
```typescript
routes: [
  { path: '/', redirect: '/library/live' },
  { path: '/library/:type', name: 'library', component: Home },
  { path: '/player/:mode/:id', name: 'player', component: () => import('@/views/PlayerPage.vue') },
  { path: '/subscriptions', name: 'subscriptions', component: () => import('@/views/Subscriptions.vue') },
  { path: '/detail/:itemId', name: 'detail', component: () => import('@/views/VodDetail.vue') },
  { path: '/vod/:id', redirect: to => `/detail/${to.params.id}` },
  { path: '/settings', name: 'settings', component: () => import('@/views/Settings.vue') }
]
```

删除：
- `/live` 路由（已重定向到 /library/live）
- `/vod` 路由（已重定向到 /library/movie）
- `/library/live` 重复路由

- [ ] **Step 2: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## 自检清单

- [ ] Live.vue 和 Vod.vue 已删除
- [ ] FullscreenPlayer.vue 已删除
- [ ] router 中的 FullscreenPlayer 路由已移除
- [ ] Settings.vue 使用设计系统样式
- [ ] router 中的冗余路由已清理
- [ ] 所有构建通过