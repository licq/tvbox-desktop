# TVBox Media Center UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the app’s core UI surfaces so Home, Detail, and Player feel like a desktop media center with playback-first information hierarchy instead of a source operations dashboard.

**Architecture:** Keep the existing playback runtime and source pipeline intact, but refactor the frontend into a clearer content-first structure with shared media-center primitives. The work is centered on view composition, reusable components, stronger type modeling for UI state, and a unified visual system layered over current stores and commands.

**Tech Stack:** Vue 3, Pinia, Vue Router, TypeScript, Tailwind CSS, Tauri invoke layer, Vitest

---

## File Structure

### Existing files to modify

- `src/views/Home.vue`
  - Rebuild into hero + continue watching + rails + live-now + source-health composition.
- `src/views/VodDetail.vue`
  - Rebuild into content header + recommended source + grouped episode matrix.
- `src/views/PlayerPage.vue`
  - Reframe as cinematic player with secondary source drawer and clearer runtime messaging.
- `src/style.css`
  - Replace current loose visual tokens with a clearer media-center design system and page-level primitives.
- `src/stores/library.ts`
  - Support home sections and rail-friendly data access without overloading one generic catalog view.
- `src/stores/detail.ts`
  - Support recommended-source state and richer detail page grouping.
- `src/types/index.ts`
  - Add explicit UI types for hero cards, source badges, episode states, and player drawer status.
- `src/stores/__tests__/library.spec.ts`
  - Extend tests for new home payload shaping.
- `src/stores/__tests__/playback.spec.ts`
  - Extend tests for player status behavior after UI changes.

### New files to create

- `src/components/home/HomeHero.vue`
  - Hero surface with featured content and compact status stack.
- `src/components/home/ContinueRail.vue`
  - Continue-watching rail.
- `src/components/home/MediaRail.vue`
  - Reusable horizontal rail for movie/series/variety/anime slices.
- `src/components/home/LiveNowPanel.vue`
  - Compact live-entry section for high-frequency channels and grouped entry points.
- `src/components/home/SourceHealthPanel.vue`
  - Secondary source health presentation.
- `src/components/detail/DetailHero.vue`
  - Poster + title + metadata + primary CTA layout.
- `src/components/detail/RecommendedSourcePanel.vue`
  - Recommended source summary and recommendation rationale.
- `src/components/detail/EpisodeGroupPanel.vue`
  - Episode matrix with group-level expansion rules and per-episode state styling.
- `src/components/player/PlaybackDrawer.vue`
  - Secondary source drawer for current line and fallback candidates.
- `src/components/player/PlaybackNotice.vue`
  - Player-facing failure/recovery surface.
- `src/components/media/MediaCard.vue`
  - Shared content card with media-first presentation.
- `src/components/media/SourceBadge.vue`
  - Shared badge for source confidence / health messaging.
- `src/components/media/EpisodeChip.vue`
  - Shared episode selection chip.

### Boundary notes

- Do not add backend commands in this plan unless a missing UI data field is absolutely required.
- Prefer deriving UI state in stores or view helpers over introducing new backend coupling.
- Keep page responsibility strict:
  - Home decides what to watch
  - Detail decides where to enter
  - Player handles playback and recovery

---

### Task 1: Expand frontend UI types for media-center surfaces

**Files:**
- Modify: `src/types/index.ts`
- Test: `src/stores/__tests__/library.spec.ts`

- [ ] **Step 1: Write the failing type-driven test for new home/detail shape**

Add this test block to `src/stores/__tests__/library.spec.ts`:

```ts
it('normalizes home payload into hero and rail-friendly card fields', () => {
  const payload = {
    continue_watching: [
      {
        id: 1,
        title: 'Arcane',
        item_type: 'series' as const,
        poster: 'https://img.test/arcane.jpg',
        progress: 52,
        source_badge: '荐片',
        update_badge: '继续观看'
      }
    ],
    latest_updates: [],
    featured: [
      {
        id: 2,
        title: 'Dune',
        item_type: 'movie' as const,
        poster: 'https://img.test/dune.jpg',
        source_badge: 'Auete',
        update_badge: '推荐'
      }
    ]
  }

  expect(payload.featured[0].item_type).toBe('movie')
  expect(payload.continue_watching[0].progress).toBe(52)
})
```

- [ ] **Step 2: Run test to verify current baseline still only models generic cards**

Run:

```bash
npx vitest run src/stores/__tests__/library.spec.ts
```

Expected:

- PASS or near-pass on existing generic card normalization
- No explicit type support yet for richer hero/source semantics

- [ ] **Step 3: Add explicit UI types for hero, badges, and episode state**

Update `src/types/index.ts` with these additions:

```ts
export type SourceConfidence = 'high' | 'medium' | 'low' | 'unknown'

export interface SourceBadge {
  label: string
  confidence?: SourceConfidence
  tone?: 'warm' | 'cool' | 'neutral' | 'danger'
}

export interface HeroMetric {
  label: string
  value: string
}

export interface HomeHeroCard extends CatalogCard {
  summary?: string
  primary_badge?: string
}

export type EpisodeAvailabilityState = 'playable' | 'resolving' | 'unavailable'

export interface DetailEpisodeView extends CatalogEpisode {
  availability?: EpisodeAvailabilityState
  source_badge?: string
}
```

- [ ] **Step 4: Run tests to verify type additions do not break store tests**

Run:

```bash
npx vitest run src/stores/__tests__/library.spec.ts src/stores/__tests__/playback.spec.ts
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/types/index.ts src/stores/__tests__/library.spec.ts
git commit -m "feat: add media center ui types"
```

---

### Task 2: Reshape library and detail stores for UI-first composition

**Files:**
- Modify: `src/stores/library.ts`
- Modify: `src/stores/detail.ts`
- Test: `src/stores/__tests__/library.spec.ts`

- [ ] **Step 1: Write failing library store tests for hero and rail usage**

Add this block to `src/stores/__tests__/library.spec.ts`:

```ts
it('keeps featured card available as hero source and continue watching as separate rail', async () => {
  const payload = {
    continue_watching: [
      { id: 1, title: 'Arcane', item_type: 'series' as const, progress: 40 }
    ],
    latest_updates: [
      { id: 2, title: 'The Bear', item_type: 'series' as const }
    ],
    featured: [
      { id: 3, title: 'Dune', item_type: 'movie' as const }
    ]
  }

  const store = useLibraryStore()
  store.applyHomePayload(payload)

  expect(store.featured[0].title).toBe('Dune')
  expect(store.continueWatching[0].title).toBe('Arcane')
  expect(store.latestUpdates[0].title).toBe('The Bear')
})
```

- [ ] **Step 2: Run test to verify current store baseline**

Run:

```bash
npx vitest run src/stores/__tests__/library.spec.ts
```

Expected:

- PASS on generic arrays, but no dedicated helper structure yet for home sections

- [ ] **Step 3: Add UI-facing helpers in `library.ts` and `detail.ts`**

Add these helpers in `src/stores/library.ts`:

```ts
function sliceRail(items: CatalogCard[], limit = 12) {
  return items.slice(0, limit)
}
```

And expose derived state:

```ts
const hero = computed(() => featured.value[0] ?? latestUpdates.value[0] ?? continueWatching.value[0] ?? null)

function getRail(itemType: CatalogItemType) {
  return sliceRail(catalogItems.value.filter(card => card.item_type === itemType))
}
```

In `src/stores/detail.ts`, add a recommended source helper:

```ts
const recommendedGroup = computed(() => episodeGroups.value[0] ?? null)
```

- [ ] **Step 4: Run store tests again**

Run:

```bash
npx vitest run src/stores/__tests__/library.spec.ts
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/stores/library.ts src/stores/detail.ts src/stores/__tests__/library.spec.ts
git commit -m "feat: shape stores for media center ui"
```

---

### Task 3: Introduce shared media-center components

**Files:**
- Create: `src/components/media/MediaCard.vue`
- Create: `src/components/media/SourceBadge.vue`
- Create: `src/components/media/EpisodeChip.vue`
- Modify: `src/style.css`

- [ ] **Step 1: Add minimal presentational component contracts**

Create `src/components/media/SourceBadge.vue`:

```vue
<script setup lang="ts">
defineProps<{
  label: string
  tone?: 'warm' | 'cool' | 'neutral' | 'danger'
}>()
</script>

<template>
  <span :class="['source-badge', tone ? `source-badge-${tone}` : 'source-badge-neutral']">
    {{ label }}
  </span>
</template>
```

Create `src/components/media/EpisodeChip.vue`:

```vue
<script setup lang="ts">
import type { EpisodeAvailabilityState } from '@/types'

defineProps<{
  label: string
  state?: EpisodeAvailabilityState
}>()
</script>

<template>
  <button :class="['episode-chip', state ? `episode-chip-${state}` : 'episode-chip-playable']">
    <span class="episode-chip-label">{{ label }}</span>
  </button>
</template>
```

- [ ] **Step 2: Add the reusable media card**

Create `src/components/media/MediaCard.vue`:

```vue
<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'

defineProps<{
  title: string
  poster?: string
  subtitle?: string
  sourceBadge?: string
}>()
</script>

<template>
  <article class="media-card">
    <div class="media-card-poster">
      <img v-if="poster" :src="poster" :alt="title" class="media-card-image" />
      <div v-else class="media-card-fallback">No Poster</div>
    </div>
    <div class="media-card-body">
      <SourceBadge v-if="sourceBadge" :label="sourceBadge" tone="cool" />
      <h3 class="media-card-title">{{ title }}</h3>
      <p v-if="subtitle" class="media-card-subtitle">{{ subtitle }}</p>
    </div>
  </article>
```

- [ ] **Step 3: Add visual-system classes in `src/style.css`**

Add these component rules:

```css
.media-card {
  display: grid;
  gap: 0.9rem;
}

.media-card-poster {
  overflow: hidden;
  border-radius: 1.4rem;
  background: rgba(255, 255, 255, 0.04);
}

.media-card-image {
  width: 100%;
  aspect-ratio: 2 / 3;
  object-fit: cover;
  display: block;
}

.source-badge {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 0.35rem 0.7rem;
  font-size: 0.68rem;
  letter-spacing: 0.18em;
  text-transform: uppercase;
}
```

- [ ] **Step 4: Run frontend build to verify new shared components compile**

Run:

```bash
npm run build
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/media/MediaCard.vue src/components/media/SourceBadge.vue src/components/media/EpisodeChip.vue src/style.css
git commit -m "feat: add media center shared components"
```

---

### Task 4: Rebuild Home into a media-center landing page

**Files:**
- Modify: `src/views/Home.vue`
- Create: `src/components/home/HomeHero.vue`
- Create: `src/components/home/ContinueRail.vue`
- Create: `src/components/home/MediaRail.vue`
- Create: `src/components/home/LiveNowPanel.vue`
- Create: `src/components/home/SourceHealthPanel.vue`
- Modify: `src/style.css`
- Test: `src/stores/__tests__/library.spec.ts`

- [ ] **Step 1: Create home section primitives**

Create `src/components/home/HomeHero.vue`:

```vue
<script setup lang="ts">
import type { CatalogCard, HeroMetric } from '@/types'

defineProps<{
  item: CatalogCard | null
  metrics: HeroMetric[]
  title: string
  summary: string
}>()
</script>

<template>
  <section class="home-hero">
    <div class="home-hero-copy">
      <p class="eyebrow">Featured</p>
      <h1 class="home-hero-title">{{ title }}</h1>
      <p class="home-hero-summary">{{ summary }}</p>
    </div>
    <div class="home-hero-metrics">
      <div v-for="metric in metrics" :key="metric.label" class="hero-metric">
        <div class="hero-metric-label">{{ metric.label }}</div>
        <div class="hero-metric-value">{{ metric.value }}</div>
      </div>
    </div>
  </section>
```

Create `src/components/home/MediaRail.vue`:

```vue
<script setup lang="ts">
import MediaCard from '@/components/media/MediaCard.vue'
import type { CatalogCard } from '@/types'

defineProps<{
  title: string
  items: CatalogCard[]
}>()
</script>

<template>
  <section class="media-rail">
    <header class="media-rail-header">
      <h2 class="section-title">{{ title }}</h2>
    </header>
    <div class="media-rail-track">
      <MediaCard
        v-for="item in items"
        :key="item.id"
        :title="item.title"
        :poster="item.poster"
        :source-badge="item.source_badge"
      />
    </div>
  </section>
```

- [ ] **Step 2: Rewrite `Home.vue` to use five-section layout**

Refactor `src/views/Home.vue` so its top-level rendering order is:

```vue
<HomeHero ... />
<ContinueRail ... />
<MediaRail title="电影" ... />
<MediaRail title="剧集" ... />
<MediaRail title="综艺" ... />
<MediaRail title="动漫" ... />
<LiveNowPanel ... />
<SourceHealthPanel ... />
```

Remove the current “tab strip as dominant navigation” layout from the hero region. Keep route-level category navigation, but demote its visual prominence.

- [ ] **Step 3: Add styling for rails and hero**

Add these rules to `src/style.css`:

```css
.home-hero {
  display: grid;
  gap: 2rem;
  grid-template-columns: minmax(0, 1.4fr) 320px;
}

.media-rail-track {
  display: grid;
  grid-auto-flow: column;
  grid-auto-columns: minmax(180px, 220px);
  gap: 1rem;
  overflow-x: auto;
  padding-bottom: 0.5rem;
}
```

- [ ] **Step 4: Run frontend build and library tests**

Run:

```bash
npm run build
npx vitest run src/stores/__tests__/library.spec.ts
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/views/Home.vue src/components/home/HomeHero.vue src/components/home/ContinueRail.vue src/components/home/MediaRail.vue src/components/home/LiveNowPanel.vue src/components/home/SourceHealthPanel.vue src/style.css
git commit -m "feat: rebuild home as media center landing page"
```

---

### Task 5: Rebuild Detail into guided source selection

**Files:**
- Modify: `src/views/VodDetail.vue`
- Create: `src/components/detail/DetailHero.vue`
- Create: `src/components/detail/RecommendedSourcePanel.vue`
- Create: `src/components/detail/EpisodeGroupPanel.vue`
- Modify: `src/style.css`

- [ ] **Step 1: Create detail section components**

Create `src/components/detail/DetailHero.vue`:

```vue
<script setup lang="ts">
import type { CatalogDetailItem } from '@/types'

defineProps<{
  item: CatalogDetailItem
}>()
</script>

<template>
  <section class="detail-hero">
    <div class="detail-hero-poster">
      <img v-if="item.poster" :src="item.poster" :alt="item.title" class="detail-hero-image" />
    </div>
    <div class="detail-hero-copy">
      <p class="eyebrow">{{ item.item_type }}</p>
      <h1 class="detail-hero-title">{{ item.title }}</h1>
      <p v-if="item.summary" class="detail-hero-summary">{{ item.summary }}</p>
    </div>
  </section>
```

Create `src/components/detail/RecommendedSourcePanel.vue`:

```vue
<script setup lang="ts">
import type { CatalogEpisodeGroup } from '@/types'

defineProps<{
  group: CatalogEpisodeGroup | null
}>()
</script>

<template>
  <section class="recommended-source-panel">
    <p class="eyebrow">Recommended Source</p>
    <h2 class="section-title">{{ group?.source_name ?? '暂无推荐线路' }}</h2>
    <p class="recommended-source-copy">
      {{ group ? '优先显示当前建议进入的来源，减少盲选。' : '当前没有可推荐的来源。' }}
    </p>
  </section>
```

- [ ] **Step 2: Rewrite `VodDetail.vue` around header + recommendation + grouped episodes**

Update page composition to:

```vue
<DetailHero ... />
<RecommendedSourcePanel :group="detailStore.recommendedGroup" />
<EpisodeGroupPanel
  v-for="group in detailStore.episodeGroups"
  :key="group.source_name"
  :group="group"
/>
```

Default visual emphasis must go to the recommended group, not equal-weight grids for all groups.

- [ ] **Step 3: Add detail-specific CSS**

Add to `src/style.css`:

```css
.detail-hero {
  display: grid;
  gap: 2rem;
  grid-template-columns: 280px minmax(0, 1fr);
}

.recommended-source-panel {
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 1.6rem;
  background: rgba(255, 255, 255, 0.04);
  padding: 1.25rem;
}
```

- [ ] **Step 4: Run frontend build**

Run:

```bash
npm run build
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/views/VodDetail.vue src/components/detail/DetailHero.vue src/components/detail/RecommendedSourcePanel.vue src/components/detail/EpisodeGroupPanel.vue src/style.css
git commit -m "feat: rebuild detail page around guided source selection"
```

---

### Task 6: Rebuild Player into cinematic playback + source drawer

**Files:**
- Modify: `src/views/PlayerPage.vue`
- Create: `src/components/player/PlaybackDrawer.vue`
- Create: `src/components/player/PlaybackNotice.vue`
- Modify: `src/stores/playback.ts`
- Modify: `src/style.css`
- Test: `src/stores/__tests__/playback.spec.ts`
- Test: `src/utils/__tests__/player.spec.ts`

- [ ] **Step 1: Add failing playback store regression for preserved failure messaging**

Add to `src/stores/__tests__/playback.spec.ts`:

```ts
it('preserves external-required and failed messaging for player notice rendering', () => {
  const store = usePlaybackStore()

  store.applyResolved({
    status: 'external_required',
    candidates: [],
    errorMessage: '当前集只有外部工具线路，桌面端未直接展示'
  })

  expect(store.status).toBe('external_required')
  expect(store.errorMessage).toContain('外部工具线路')
})
```

- [ ] **Step 2: Create player surface helpers**

Create `src/components/player/PlaybackNotice.vue`:

```vue
<script setup lang="ts">
defineProps<{
  title: string
  message?: string | null
}>()
</script>

<template>
  <div class="playback-notice">
    <div class="playback-notice-title">{{ title }}</div>
    <div v-if="message" class="playback-notice-copy">{{ message }}</div>
  </div>
```

Create `src/components/player/PlaybackDrawer.vue`:

```vue
<script setup lang="ts">
import type { PlaybackCandidate } from '@/types'

defineProps<{
  sources: PlaybackCandidate[]
  currentIndex: number
}>()

const emit = defineEmits<{
  select: [index: number]
}>()
</script>

<template>
  <aside class="playback-drawer">
    <button
      v-for="(source, index) in sources"
      :key="`${source.label}-${index}`"
      :class="['playback-drawer-item', currentIndex === index ? 'playback-drawer-item-active' : '']"
      @click="emit('select', index)"
    >
      {{ source.label }}
    </button>
  </aside>
```

- [ ] **Step 3: Recompose `PlayerPage.vue`**

Make these structural changes:

- keep video area as dominant
- move source list into `PlaybackDrawer`
- render `PlaybackNotice` for failure and fallback states
- keep auto-switch logic, but make the on-screen message player-facing

Required state mapping:

```ts
const noticeTitle = computed(() => {
  if (playbackStore.status === 'external_required') return '需要外部工具'
  if (errorMsg.value) return '当前线路不可用'
  return ''
})
```

- [ ] **Step 4: Add player CSS and run tests**

Add to `src/style.css`:

```css
.playback-drawer {
  display: grid;
  gap: 0.75rem;
  align-content: start;
}

.playback-notice {
  border-radius: 1.2rem;
  background: rgba(10, 17, 24, 0.84);
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 1rem 1.1rem;
}
```

Run:

```bash
npx vitest run src/stores/__tests__/playback.spec.ts src/utils/__tests__/player.spec.ts
npm run build
```

Expected:

- PASS

- [ ] **Step 5: Commit**

```bash
git add src/views/PlayerPage.vue src/components/player/PlaybackDrawer.vue src/components/player/PlaybackNotice.vue src/stores/playback.ts src/stores/__tests__/playback.spec.ts src/utils/__tests__/player.spec.ts src/style.css
git commit -m "feat: rebuild player ui around cinematic playback flow"
```

---

### Task 7: Final visual-system pass and responsive cleanup

**Files:**
- Modify: `src/style.css`
- Modify: `src/views/Home.vue`
- Modify: `src/views/VodDetail.vue`
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Add responsive media-center layout rules**

Add responsive adjustments to `src/style.css`:

```css
@media (max-width: 1100px) {
  .home-hero,
  .detail-hero {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 768px) {
  .app-shell {
    padding-inline: 1rem;
  }
}
```

- [ ] **Step 2: Remove leftover tool-dashboard copy and hierarchy**

Audit these files and replace any remaining console-like wording:

- `src/views/Home.vue`
- `src/views/VodDetail.vue`
- `src/views/PlayerPage.vue`

Specifically remove or rewrite phrases that still sound like:

- control panel
- source console
- internal debugging
- raw technical runtime labels shown as primary text

- [ ] **Step 3: Run final verification**

Run:

```bash
npx vitest run src/stores/__tests__/library.spec.ts src/stores/__tests__/playback.spec.ts src/utils/__tests__/player.spec.ts
npm run build
```

Expected:

- PASS

- [ ] **Step 4: Commit**

```bash
git add src/style.css src/views/Home.vue src/views/VodDetail.vue src/views/PlayerPage.vue
git commit -m "feat: polish media center ui system and responsive behavior"
```

---

## Spec Coverage Check

- Home as five-part media-center surface: covered by Task 4.
- Detail as guided source-selection page: covered by Task 5.
- Player as cinematic playback with secondary source drawer and clearer notice language: covered by Task 6.
- Shared visual system and reusable components: covered by Tasks 1, 3, and 7.
- Responsive behavior and hierarchy cleanup: covered by Task 7.

## Placeholder Scan

Checked for:

- `TBD`
- `TODO`
- “implement later”
- vague “add tests” without code

No unresolved placeholders remain.

## Type Consistency Check

- Shared types are introduced in Task 1 before they are consumed in Tasks 2-6.
- Home/detail/player components refer only to names introduced earlier in the plan.
- Playback-facing states remain aligned with existing runtime values: `ready`, `failed`, `external_required`.
