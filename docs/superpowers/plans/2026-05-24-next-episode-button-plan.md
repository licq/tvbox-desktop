# Next Episode Button — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add "下一集 →" button to player control bar that auto-plays the next episode when clicked.

**Architecture:** Add two computed properties (`hasNextEpisode`, `nextEpisode`) and one handler function (`playNextEpisode`) to PlayerPage.vue. Add a single button to the control bar template.

**Tech Stack:** Vue 3 Composition API, TypeScript, no new dependencies

---

## Task 1: Add hasNextEpisode and nextEpisode computed properties

**Files:**
- Modify: `src/views/PlayerPage.vue` — add computed properties after `unifiedEpisodes` and `currentNormalizedIndex`

- [ ] **Step 1: Read PlayerPage.vue lines 163-195 to find existing computed properties**

- [ ] **Step 2: Add hasNextEpisode and nextEpisode computed properties**

After `currentNormalizedIndex` computed (around line 189), add:

```ts
const hasNextEpisode = computed(() => {
  if (itemType.value === 'movie') return false
  const nextIdx = (currentNormalizedIndex.value ?? -1) + 1
  return nextIdx < unifiedEpisodes.value.length
})

const nextEpisode = computed(() => {
  if (!hasNextEpisode.value) return null
  return unifiedEpisodes.value.find(e => e.normalizedIndex === currentNormalizedIndex.value + 1) ?? null
})
```

- [ ] **Step 3: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add hasNextEpisode and nextEpisode computed properties"
```

---

## Task 2: Add playNextEpisode handler

**Files:**
- Modify: `src/views/PlayerPage.vue` — add handler function near `switchToEpisode` function

- [ ] **Step 1: Read PlayerPage.vue to find `switchToEpisode` function (around line 1491)**

- [ ] **Step 2: Add playNextEpisode handler before switchToEpisode**

```ts
function playNextEpisode() {
  if (nextEpisode.value) {
    playUnifiedEpisode(nextEpisode.value, undefined, true)
  }
}
```

- [ ] **Step 3: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add playNextEpisode handler"
```

---

## Task 3: Add next episode button to control bar template

**Files:**
- Modify: `src/views/PlayerPage.vue` — add button in the control bar, next to fullscreen button

- [ ] **Step 1: Read PlayerPage.vue around lines 2140-2165 to find the control bar structure**

- [ ] **Step 2: Add button after fullscreen button (line 2144)**

Find:
```vue
<button class="action-button action-button-secondary" type="button" @click="toggleFullscreen">
  {{ fullscreen ? '退出全屏' : '全屏' }}
</button>
```

Add after it:
```vue
<button
  v-if="hasNextEpisode"
  class="action-button action-button-secondary"
  type="button"
  @click="playNextEpisode"
>
  下一集 →
</button>
```

- [ ] **Step 3: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add next episode button to control bar"
```

---

## Verification

1. Run `npm run build` — should compile without errors
2. Navigate to a series with multiple episodes
3. Play an episode that is NOT the last one — button should be visible with "下一集 →"
4. Click the button — should auto-play the next episode
5. Play the last episode — button should be hidden
6. Navigate to a movie — button should be hidden