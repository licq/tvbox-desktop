# VodDetail Movie Search Card Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make movie search results show playable episode links directly inside SearchResultCard instead of a "Play Now" button that expands to EpisodeGroupPanel.

**Architecture:** SearchResultCard renders episode buttons inline for movies by reading preloaded source details (same data flow as episodes). A small label-deduplication helper formats button text as `source_short · episode_label`. Episodes keep existing source-selector + EpisodeGrid pattern.

**Tech Stack:** Vue 3, TypeScript, vitest, @vue/test-utils

---

### File Structure

| File | Responsibility |
|------|---------------|
| `src/components/detail/SearchResultCard.vue` | Core change: movie renders episode buttons directly; adds label formatting helper |
| `src/components/detail/__tests__/SearchResultCard.spec.ts` | Updated tests for new movie rendering + label dedup logic |

---

### Task 1: Update tests for movie direct-episode rendering

**Files:**
- Modify: `src/components/detail/__tests__/SearchResultCard.spec.ts`

These tests validate the new behavior before implementation.

- [ ] **Step 1: Replace movie-renders-MovieActionPanel test**

Replace the existing test `'renders MovieActionPanel for movies'` with one that asserts episode buttons are rendered directly when sourceDetails are provided.

```typescript
  it('renders episode buttons for movies when source details provided', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    expect(wrapper.find('.movie-action-panel').exists()).toBe(false)
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })
```

- [ ] **Step 2: Add test for movie button label dedup**

When `episode_label` already contains `source_name`, the button should show only `episode_label` without repeating the source name.

```typescript
  it('deduplicates source name from movie episode label', () => {
    const detailsWithOverlap = {
      s1: {
        title: '测试影片',
        poster: null,
        summary: null,
        episodes: [
          { id: 1, episode_label: '文才HD', play_url: 'http://a/1', order_index: 1 },
        ],
      },
    }
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails: detailsWithOverlap,
      },
    })
    const btn = wrapper.find('.source-btn')
    expect(btn.text()).toBe('文才HD')
    expect(btn.text()).not.toContain('文才 ·')
  })
```

- [ ] **Step 3: Add test for movie button label with source prefix**

When `episode_label` does NOT contain `source_name`, the button should show `source_short · episode_label`.

```typescript
  it('prefixes source short name when episode label does not contain it', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    const btn = wrapper.find('.source-btn')
    expect(btn.text()).toBe('来源A · 01')
  })
```

- [ ] **Step 4: Add test for movie loading state**

When a source is in `loadingSources`, show a loading placeholder instead of buttons for that source.

```typescript
  it('shows loading placeholder for movie when a source is loading', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
        loadingSources: ['s1'],
      },
    })
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
  })
```

- [ ] **Step 5: Add test for movie empty-episode source**

A source with zero episodes should not render any buttons.

```typescript
  it('skips empty sources for movies', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    // s1 has 2 episodes, s2 has 0 episodes => 2 buttons
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })
```

- [ ] **Step 6: Update play-source emit test to play-episode emit for movies**

Movies now emit `play-episode` with the episode object and source key, same as episodes.

Replace the existing test `'emits play-source when MovieActionPanel primary button is clicked'` with:

```typescript
  it('emits play-episode when a movie episode button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    await wrapper.find('.source-btn').trigger('click')
    expect(wrapper.emitted('play-episode')).toHaveLength(1)
    expect(wrapper.emitted('play-episode')![0]).toEqual([episodes[0], 's1'])
  })
```

- [ ] **Step 7: Run tests to confirm they fail**

```bash
vitest run src/components/detail/__tests__/SearchResultCard.spec.ts
```

Expected: Several tests FAIL because movie no longer renders `.movie-action-panel` and doesn't yet emit `play-episode`.

---

### Task 2: Implement SearchResultCard.vue movie inline episodes

**Files:**
- Modify: `src/components/detail/SearchResultCard.vue`

- [ ] **Step 1: Remove MovieActionPanel import and usage**

Remove the import:
```typescript
import MovieActionPanel from './MovieActionPanel.vue'
```

In the template, replace the `<MovieActionPanel>` block inside `.card-right` with movie episode button rendering logic (see Step 2).

- [ ] **Step 2: Add movie episode rendering template**

Inside `.card-right`, change the movie branch to render episode buttons directly:

```vue
      <template v-if="isMovie">
        <div class="source-action-area">
          <div v-if="isLoadingAnyMovieSource" class="loading-placeholder">
            加载中…
          </div>
          <div v-else class="source-selector-row">
            <button
              v-for="btn in movieEpisodeButtons"
              :key="btn.key"
              type="button"
              class="source-btn"
              @click="emit('play-episode', btn.episode, btn.source)"
            >
              {{ btn.label }}
            </button>
          </div>
          <div
            v-if="!isLoadingAnyMovieSource && movieEpisodeButtons.length === 0"
            class="load-episodes-btn"
          >
            暂无播放链接
          </div>
        </div>
      </template>
```

- [ ] **Step 3: Add computed properties for movie button data**

Add these computed properties in `<script setup>`:

```typescript
const isLoadingAnyMovieSource = computed(() => {
  if (!isMovie.value) return false
  return props.sources.some(s =>
    props.loadingSources?.includes(s.source)
  )
})

interface MovieEpisodeButton {
  key: string
  source: string
  episode: CatalogEpisode
  label: string
}

const movieEpisodeButtons = computed<MovieEpisodeButton[]>(() => {
  if (!isMovie.value) return []
  const buttons: MovieEpisodeButton[] = []
  for (const src of props.sources) {
    const detail = props.sourceDetails?.[src.source]
    if (!detail) continue
    for (const ep of detail.episodes) {
      const label = formatEpisodeLabel(src.source_name, ep.episode_label)
      buttons.push({
        key: `${src.source}-${ep.id}`,
        source: src.source,
        episode: ep,
        label,
      })
    }
  }
  return buttons
})

function formatEpisodeLabel(sourceName: string, episodeLabel: string): string {
  // If episodeLabel already contains sourceName, return as-is
  if (episodeLabel.includes(sourceName)) {
    return episodeLabel
  }
  // Extract a short name (first 2-4 chars, preserving whole characters for CJK)
  const short = sourceName.slice(0, 4)
  return `${short} · ${episodeLabel}`
}
```

- [ ] **Step 4: Update emits to remove play-source for movie path**

`SearchResultCard` no longer emits `play-source` from the movie path (episodes never used it). Keep `play-source` in the emits declaration for backward compatibility, but it's only emitted by the now-removed MovieActionPanel.

Actually, verify: the parent `VodDetail.vue` still listens to `@play-source` on SearchResultCard for the old flow. Check `VodDetail.vue` line 538. Since we are removing MovieActionPanel, the `play-source` event will never fire from SearchResultCard. The parent handler `handleCardSourcePlay` should be removed or we should make SearchResultCard still emit `play-source` for movies that have no preloaded details yet.

Wait — per the spec, movies now ALWAYS show episode buttons directly. If no source details are preloaded, we show "加载中…". So `play-source` from SearchResultCard is truly dead. We can remove it from the emits and from the parent listener.

But to keep changes minimal and avoid touching VodDetail.vue, let's leave `play-source` in the emit declaration and just not emit it. The parent handler will simply never fire.

- [ ] **Step 5: Run tests**

```bash
vitest run src/components/detail/__tests__/SearchResultCard.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/detail/SearchResultCard.vue
 git add src/components/detail/__tests__/SearchResultCard.spec.ts
git commit -m "feat: show movie episodes directly in search result cards"
```

---

### Task 3: Self-review / Cleanup

- [ ] **Step 1: Verify MovieActionPanel is no longer imported**

Confirm `MovieActionPanel` import line is gone from `SearchResultCard.vue`.

- [ ] **Step 2: Verify no TypeScript errors**

```bash
npm run build
```

Expected: build completes with 0 errors.

- [ ] **Step 3: Commit if build fixes were needed**

If any type fixes were needed, commit them.

---

## Self-Review

**1. Spec coverage:**
- Movies show playback links directly in cards ✅ → Task 2
- Source name + episode_label combined with dedup ✅ → Task 1 Step 2,3 + Task 2 Step 3
- Episodes keep existing pattern ✅ → no changes to series branch
- Loading/error states ✅ → Task 1 Step 4,5 + Task 2 Step 2
- Tests ✅ → Task 1 covers all behaviors

**2. Placeholder scan:** No TBD, TODO, or vague steps found.

**3. Type consistency:** `CatalogEpisode`, `sourceDetails` types match existing definitions. `formatEpisodeLabel` takes `(string, string)` → returns `string` consistently.
