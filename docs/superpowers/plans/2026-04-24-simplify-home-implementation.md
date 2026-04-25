# 首页精简实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended)

**Goal:** 删除非核心区块，精简首页布局

**Architecture:** 删除 HomeHero、ContinueRail、SourceHealthPanel、MediaRail[]，简化 Tab 内容区

**Tech Stack:** Vue 3, Tailwind CSS

---

## Task 1: 删除 HomeHero

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 删除 HomeHero 组件调用**

删除 `<HomeHero ...>` 组件（约第 240-245 行）

- [ ] **Step 2: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 2: 删除 ContinueRail

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 删除 ContinueRail 组件调用**

删除 `<ContinueRail ...>` 组件（约第 247 行）

- [ ] **Step 2: 删除相关导入**

删除 `import ContinueRail from '@/components/home/ContinueRail.vue'`

- [ ] **Step 3: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 3: 删除 SourceHealthPanel

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 删除 SourceHealthPanel 组件调用**

删除 `<SourceHealthPanel ...>` 组件（约第 260 行）

- [ ] **Step 2: 删除相关导入**

删除 `import SourceHealthPanel from '@/components/home/SourceHealthPanel.vue'`

- [ ] **Step 3: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 4: 删除 MediaRail[]

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 删除 MediaRail 组件调用**

删除 `<MediaRail ...>` 组件（约第 249-256 行）

- [ ] **Step 2: 删除相关导入**

删除 `import MediaRail from '@/components/home/MediaRail.vue'`

- [ ] **Step 3: 删除 railSummaries 相关代码**

删除 `railSummaries` 对象和 `rails` computed（约第 91-109 行）

- [ ] **Step 4: 删除 catalogTypes 常量**

删除 `const catalogTypes`（约第 51 行）

- [ ] **Step 5: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 5: 简化 home-secondary-browser

**Files:**
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 简化 home-secondary-browser 结构**

保留搜索栏和 Tab 内容，直接显示：
- live: 频道列表
- 其他: VodCard 网格

删除不必要的层级结构。

- [ ] **Step 2: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## 自检清单

- [ ] HomeHero 已删除
- [ ] ContinueRail 已删除
- [ ] SourceHealthPanel 已删除
- [ ] MediaRail[] 已删除
- [ ] home-secondary-browser 已简化
- [ ] 所有构建通过
