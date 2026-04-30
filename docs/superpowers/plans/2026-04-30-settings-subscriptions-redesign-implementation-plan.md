# Settings and Subscriptions Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the Settings and Subscriptions pages into a unified cinematic control surface with a polished settings hub and a task-oriented subscription panel.

**Architecture:** Add a small set of shared page chrome and task-panel styles in `src/style.css`, then rebuild `Settings.vue` and `Subscriptions.vue` around the same hero/header/card grammar. Keep stores and backend commands unchanged; the only logic additions are derived summary counts and refresh-state binding on the subscriptions page.

**Tech Stack:** Vue 3 SFCs, Pinia store state, Tailwind utilities, existing CSS custom properties in `src/style.css`.

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/style.css` | Add reusable page chrome, summary strip, field-row, task-row, and empty-state utilities for both pages. |
| `src/views/Settings.vue` | Rebuild the settings page as a two-column hub with a hero header, polished cards, and consistent control rows. |
| `src/views/Subscriptions.vue` | Rebuild the subscriptions page as a task panel with a summary strip, add form, and subscription task cards. |

No new files are required for this redesign.

---

### Task 1: Add Shared Chrome and Task-Panel Styles

**Files:**
- Modify: `src/style.css`

- [ ] **Step 1: Add page-hero, panel, and task-row utilities**

Append a small group of reusable layout utilities inside the existing `@layer components` block so both pages can share the same structure:

```css
.page-hero {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

@media (min-width: 1024px) {
  .page-hero {
    flex-direction: row;
    align-items: end;
    justify-content: space-between;
  }
}

.page-hero-copy {
  max-width: 48rem;
  display: grid;
  gap: 0.45rem;
}

.page-hero-title {
  font-size: clamp(1.8rem, 3vw, 2.6rem);
  line-height: 1.1;
  font-weight: 700;
  letter-spacing: 0.02em;
  color: var(--text-strong);
}

.page-hero-subtitle {
  max-width: 44rem;
  font-size: 0.95rem;
  line-height: 1.7;
  color: var(--text-soft);
}

.page-hero-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.panel-grid {
  display: grid;
  gap: 1rem;
}

@media (min-width: 1024px) {
  .panel-grid {
    grid-template-columns: minmax(0, 1.2fr) minmax(320px, 0.8fr);
    align-items: start;
  }
}

.panel-stack {
  display: grid;
  gap: 1rem;
}

.panel-header {
  display: flex;
  align-items: start;
  justify-content: space-between;
  gap: 1rem;
  margin-bottom: 1rem;
}

.panel-header-copy {
  display: grid;
  gap: 0.35rem;
}

.panel-header-title {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--text-strong);
}

.panel-header-subtitle {
  font-size: 0.88rem;
  line-height: 1.6;
  color: var(--text-soft);
}

.panel-kicker {
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.24em;
  text-transform: uppercase;
  color: var(--accent);
}

.field-row {
  display: grid;
  gap: 0.5rem;
}

@media (min-width: 768px) {
  .field-row {
    grid-template-columns: 180px minmax(0, 1fr);
    align-items: center;
  }
}

.field-label {
  font-size: 0.95rem;
  font-weight: 500;
  color: rgba(244, 239, 232, 0.82);
}

.field-help {
  font-size: 0.82rem;
  line-height: 1.6;
  color: var(--text-soft);
}

.field-control {
  width: 100%;
  min-height: 2.75rem;
  border-radius: 1rem;
  border: 1px solid rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.05);
  padding: 0.65rem 0.9rem;
  color: var(--text-strong);
  outline: none;
}

.field-control:focus {
  border-color: rgba(216, 154, 87, 0.5);
  box-shadow: 0 0 0 3px rgba(216, 154, 87, 0.16);
}

.task-summary-strip {
  display: grid;
  gap: 0.75rem;
}

@media (min-width: 768px) {
  .task-summary-strip {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}

.task-summary-card {
  padding: 1rem 1.1rem;
  border-radius: 1.4rem;
  background: rgba(255, 255, 255, 0.035);
  border: 1px solid rgba(255, 255, 255, 0.08);
}

.task-summary-value {
  margin-top: 0.45rem;
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--text-strong);
}

.task-row {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  padding: 1rem 1.1rem;
  border-radius: 1.45rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.035);
}

@media (min-width: 768px) {
  .task-row {
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
  }
}

.task-row-main {
  min-width: 0;
  flex: 1;
  display: grid;
  gap: 0.4rem;
}

.task-row-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text-strong);
}

.task-row-subtitle {
  min-width: 0;
  font-size: 0.86rem;
  line-height: 1.55;
  color: var(--text-soft);
  word-break: break-all;
}

.task-row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 0.6rem;
}

.empty-panel {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 0.9rem;
  padding: 2rem 1.25rem;
  border-radius: 1.75rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.03);
  text-align: center;
}

.danger-button {
  color: #2a0909;
  background: linear-gradient(135deg, #ffadad, #ffd0d0);
  border: 1px solid rgba(255, 173, 173, 0.42);
}
```

- [ ] **Step 2: Keep existing component tokens intact**

Do not replace `surface-panel`, `surface-muted`, `action-button`, or the warm accent variables. The new utilities should sit beside the current system so the home page and detail pages still look like they belong to the same product.

- [ ] **Step 3: Commit the shared style layer**

```bash
git add src/style.css
git commit -m "refactor: add shared settings panel styles"
```

---

### Task 2: Redesign `Settings.vue` Into a Settings Hub

**Files:**
- Modify: `src/views/Settings.vue`

- [ ] **Step 1: Keep the existing cache-clear methods and router wiring**

Leave the `useRouter()` and `invoke()` calls in place. The only script-side change should be cosmetic data if the template benefits from a small helper array; otherwise keep the current methods unchanged:

```ts
async function clearSourceSearchCache() {
  try {
    const count = await invoke<number>('clear_source_search_cache')
    alert(`已清除 ${count} 条源搜索缓存`)
  } catch (e) {
    alert('清除失败: ' + e)
  }
}

async function clearDoubanSearchCache() {
  try {
    const count = await invoke<number>('clear_douban_search_cache')
    alert(`已清除 ${count} 条豆瓣搜索缓存`)
  } catch (e) {
    alert('清除失败: ' + e)
  }
}
```

- [ ] **Step 2: Replace the flat page header with a hero header**

Change the top of the template to a hero that matches Home/VodDetail:

```vue
<div class="app-shell">
  <div class="mx-auto max-w-6xl">
    <header class="page-hero">
      <div class="page-hero-copy">
        <div class="eyebrow">TVBox Desktop</div>
        <h1 class="page-hero-title">设置中心</h1>
        <p class="page-hero-subtitle">
          管理播放、界面和缓存策略。这里保持和首页一致的深色电影感，但把设置做成更清晰的控制面板。
        </p>
      </div>

      <div class="page-hero-actions">
        <button class="action-button action-button-secondary" type="button" @click="router.back()">
          ← 返回
        </button>
      </div>
    </header>
  </div>
</div>
```

- [ ] **Step 3: Rebuild the content area into a two-column grid**

Use the shared `panel-grid` / `panel-stack` pattern. Left column should contain playback and appearance; right column should contain cache and about:

```vue
<main class="mx-auto grid max-w-6xl gap-4 lg:grid-cols-2">
  <div class="panel-stack">
    <section class="surface-panel rounded-[2rem] p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <h2 class="panel-header-title">播放设置</h2>
          <p class="panel-header-subtitle">控制默认播放画质和硬解行为。</p>
        </div>
      </div>

      <div class="space-y-4">
        <label class="field-row">
          <div>
            <div class="field-label">默认播放画质</div>
            <div class="field-help">决定播放器优先尝试的清晰度。</div>
          </div>
          <select class="field-control">
            <option>自动</option>
            <option>1080P</option>
            <option>720P</option>
            <option>480P</option>
          </select>
        </label>

        <label class="field-row">
          <div>
            <div class="field-label">启用硬解</div>
            <div class="field-help">优先使用硬件解码播放高码率内容。</div>
          </div>
          <input type="checkbox" class="w-5 h-5 accent-[var(--accent)]" checked />
        </label>
      </div>
    </section>

    <section class="surface-panel rounded-[2rem] p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <h2 class="panel-header-title">界面设置</h2>
          <p class="panel-header-subtitle">保持当前深色视觉系统，并预留主题切换入口。</p>
        </div>
      </div>

      <label class="field-row">
        <div>
          <div class="field-label">主题</div>
          <div class="field-help">当前只保留深色、浅色和自动三种选项。</div>
        </div>
        <select class="field-control">
          <option>深色</option>
          <option>浅色</option>
          <option>自动</option>
        </select>
      </label>
    </section>
  </div>

  <div class="panel-stack">
    <section class="surface-panel rounded-[2rem] p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <h2 class="panel-header-title">缓存管理</h2>
          <p class="panel-header-subtitle">按任务清理搜索缓存，保留状态提示和危险操作样式。</p>
        </div>
      </div>

      <div class="space-y-3">
        <div class="task-row">
          <div class="task-row-main">
            <div class="task-row-title">源搜索缓存</div>
            <div class="task-row-subtitle">清理通过各个内容源检索得到的本地缓存数据。</div>
          </div>
          <div class="task-row-actions">
            <button class="action-button danger-button" type="button" @click="clearSourceSearchCache">清除</button>
          </div>
        </div>

        <div class="task-row">
          <div class="task-row-main">
            <div class="task-row-title">豆瓣搜索缓存</div>
            <div class="task-row-subtitle">清理豆瓣搜索与详情抓取缓存，避免旧结果影响页面刷新。</div>
          </div>
          <div class="task-row-actions">
            <button class="action-button danger-button" type="button" @click="clearDoubanSearchCache">清除</button>
          </div>
        </div>
      </div>
    </section>

    <section class="surface-panel rounded-[2rem] p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <h2 class="panel-header-title">关于</h2>
          <p class="panel-header-subtitle">把版本信息做成更像产品信息卡，而不是普通文本。</p>
        </div>
      </div>

      <div class="rounded-[1.4rem] bg-white/5 p-4">
        <div class="text-sm text-white/70">TVBox 影视仓 v0.1.0</div>
        <div class="mt-2 text-sm leading-6 text-white/55">
          基于 Rust + Tauri + Vue 构建。当前页面只展示和设置相关的核心控制项，不引入额外状态。
        </div>
      </div>
    </section>
  </div>
</main>
```

- [ ] **Step 4: Keep the template responsive and polished**

Make sure the hero collapses cleanly on narrow widths, the cards keep rounded corners, and the controls reuse the same `action-button` language as the rest of the app.

- [ ] **Step 5: Commit the settings redesign**

```bash
git add src/views/Settings.vue
git commit -m "refactor(settings): redesign settings hub layout"
```

---

### Task 3: Redesign `Subscriptions.vue` Into a Task Panel

**Files:**
- Modify: `src/views/Subscriptions.vue`

- [ ] **Step 1: Add derived counts and fix per-item refresh state**

Extend the script so the header can show real summary numbers and the refresh button can reflect the active row:

```ts
import { computed, ref, onMounted } from 'vue'

const totalSubscriptions = computed(() => subStore.subscriptions.length)
const enabledSubscriptions = computed(() => subStore.subscriptions.filter(s => s.enabled).length)
const disabledSubscriptions = computed(() => totalSubscriptions.value - enabledSubscriptions.value)
const activeRefreshLabel = computed(() => {
  if (!subStore.isRefreshing) return '空闲'
  return `${subStore.refreshingName} 刷新中`
})

async function handleRefresh(sub: SourceSubscription) {
  refreshing.value = sub.id
  subStore.setRefreshing(sub.name, 1, 1)
  try {
    await subStore.refreshSubscription(sub.id)
  } catch (e) {
    alert('刷新失败: ' + e)
  } finally {
    refreshing.value = null
    subStore.clearRefreshing()
  }
}
```

- [ ] **Step 2: Replace the old flat header with a hero + action area**

Rework the top of the template so the page looks like a task panel, not a plain CRUD list:

```vue
<div class="app-shell">
  <div class="mx-auto max-w-6xl">
    <header class="page-hero">
      <div class="page-hero-copy">
        <div class="eyebrow">TVBox Desktop</div>
        <h1 class="page-hero-title">订阅任务面板</h1>
        <p class="page-hero-subtitle">
          在这里管理订阅源的添加、刷新、启用和删除。顶部保留汇总状态，下面按任务卡片呈现每个订阅。
        </p>
      </div>

      <div class="page-hero-actions">
        <RouterLink to="/library/live" class="action-button action-button-secondary">
          ← 返回主页
        </RouterLink>
        <button class="action-button action-button-primary" type="button" @click="showAddForm = !showAddForm">
          {{ showAddForm ? '收起' : '+ 添加订阅' }}
        </button>
      </div>
    </header>
  </div>
</div>
```

- [ ] **Step 3: Add a summary strip above the list**

Introduce a dashboard-style strip that shows counts and current refresh state:

```vue
<section class="task-summary-strip">
  <article class="task-summary-card">
    <div class="panel-kicker">总数</div>
    <div class="task-summary-value">{{ totalSubscriptions }}</div>
  </article>
  <article class="task-summary-card">
    <div class="panel-kicker">启用</div>
    <div class="task-summary-value">{{ enabledSubscriptions }}</div>
  </article>
  <article class="task-summary-card">
    <div class="panel-kicker">停用</div>
    <div class="task-summary-value">{{ disabledSubscriptions }}</div>
  </article>
  <article class="task-summary-card">
    <div class="panel-kicker">刷新状态</div>
    <div class="task-summary-value text-[1rem] leading-6">{{ activeRefreshLabel }}</div>
  </article>
</section>
```

- [ ] **Step 4: Turn the add form into a focused task panel**

Keep the same add behavior, but render it as a surfaced card instead of a simple block:

```vue
<section v-if="showAddForm" class="surface-panel rounded-[2rem] p-6">
  <div class="panel-header">
    <div class="panel-header-copy">
      <h2 class="panel-header-title">添加订阅</h2>
      <p class="panel-header-subtitle">填写名称和 JSON 地址后即可加入当前订阅列表。</p>
    </div>
  </div>

  <div class="space-y-4">
    <label class="field-row">
      <div>
        <div class="field-label">名称</div>
        <div class="field-help">用于在列表和刷新提示中识别这个订阅。</div>
      </div>
      <input v-model="newName" type="text" class="field-control" placeholder="例如: 我的收藏" />
    </label>

    <label class="field-row">
      <div>
        <div class="field-label">订阅地址 (JSON)</div>
        <div class="field-help">一个指向订阅配置 JSON 的可访问地址。</div>
      </div>
      <input
        v-model="newUrl"
        type="text"
        class="field-control"
        placeholder="https://example.com/subscription.json"
      />
    </label>

    <div class="flex justify-end">
      <button class="action-button action-button-primary" type="button" @click="handleAdd">添加订阅</button>
    </div>
  </div>
</section>
```

- [ ] **Step 5: Rebuild the list into wide task cards**

Render each subscription as a card with clear state, a strong title, muted URL, and action group:

```vue
<div v-else class="space-y-3">
  <article v-for="sub in subStore.subscriptions" :key="sub.id" class="task-row">
    <div class="task-row-main">
      <div class="flex items-center gap-3">
        <button
          :class="['w-12 h-6 rounded-full transition', sub.enabled ? 'bg-primary' : 'bg-gray-600']"
          @click="handleToggle(sub)"
        >
          <div
            :class="['w-5 h-5 bg-white rounded-full transition transform', sub.enabled ? 'translate-x-6' : 'translate-x-0.5']"
          ></div>
        </button>
        <div class="min-w-0">
          <div class="task-row-title">{{ sub.name }}</div>
          <div class="task-row-subtitle">{{ sub.url }}</div>
        </div>
      </div>
    </div>

    <div class="task-row-actions">
      <button
        :disabled="refreshing === sub.id"
        class="action-button action-button-secondary"
        type="button"
        @click="handleRefresh(sub)"
      >
        {{ refreshing === sub.id ? '刷新中...' : '🔄 刷新' }}
      </button>
      <button class="action-button danger-button" type="button" @click="handleDelete(sub)">🗑️ 删除</button>
    </div>
  </article>
</div>
```

- [ ] **Step 6: Add polished empty and loading states**

Use the shared `empty-panel` style so the page does not collapse into plain text:

```vue
<div v-if="subStore.loading" class="flex justify-center py-10">
  <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
</div>

<div v-else-if="subStore.subscriptions.length === 0" class="empty-panel">
  <div class="text-lg font-semibold">暂无订阅</div>
  <div class="max-w-md text-sm leading-6 text-white/55">
    先添加一个订阅源，再通过刷新任务把内容同步到本地数据库。
  </div>
  <button class="action-button action-button-primary" type="button" @click="showAddForm = true">
    + 添加订阅
  </button>
</div>
```

- [ ] **Step 7: Commit the subscriptions redesign**

```bash
git add src/views/Subscriptions.vue
git commit -m "refactor(subscriptions): redesign task panel layout"
```

---

### Task 4: Verify Build and Visual Behavior

**Files:**
- No code changes; verification only.

- [ ] **Step 1: Run the production build**

```bash
npm run build
```

Expected:
- TypeScript check passes.
- Vite build finishes without template or style errors.

- [ ] **Step 2: Run the desktop app and inspect the two redesigned pages**

```bash
npm run tauri -- dev
```

Expected manual checks:
- Settings page shows a hero header, two-column card layout, and consistent control rows.
- Subscriptions page shows the summary strip, add panel, task cards, and visible refresh state.
- The back buttons still return to the prior area.
- Destructive cache and delete actions still prompt or alert as before.

- [ ] **Step 3: Check the working tree for accidental formatting noise**

```bash
git diff --check
git status --short
```

Expected:
- No whitespace errors.
- Only the intended UI files and supporting CSS are modified.

- [ ] **Step 4: Commit verification notes if needed**

If the implementation required a follow-up fix, keep the commit focused and reference the exact page that needed correction.
