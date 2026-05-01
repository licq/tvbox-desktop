# 播放页上下文保留与非阻塞自动切源 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the player always show the correct title and keep episode/source lists visible while it automatically fails over to the next playable source.

**Architecture:** Reintroduce a small playback-context handoff from detail pages into the player store so the player can render title and episode metadata before async detail fetches finish. Keep the existing playback-session failover logic, but make the drawer loading state non-blocking and centralize the "should show skeleton vs. show content" decision in a small UI helper so it is easy to test. No backend API changes are needed.

**Tech Stack:** Vue 3, Pinia, TypeScript, Vitest, hls.js, Tauri 2.x

---

### Task 1: Restore playback-context handoff from detail page

**Files:**
- Modify: `src/stores/player.ts`
- Modify: `src/views/VodDetail.vue`
- Test: `src/stores/__tests__/player.spec.ts`

- [ ] **Step 1: Write the failing store test**

```ts
import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it, beforeEach } from 'vitest'
import { usePlayerStore } from '@/stores/player'

beforeEach(() => {
  setActivePinia(createPinia())
})

it('stores and consumes pending playback detail exactly once', () => {
  const store = usePlayerStore()
  const snapshot = {
    item: { id: 7, title: '庆余年', item_type: 'series' as const },
    episode_groups: [{ source_name: '非凡线路', episodes: [] }],
  }

  store.setPendingPlaybackDetail(snapshot)

  expect(store.takePendingPlaybackDetail()).toEqual(snapshot)
  expect(store.takePendingPlaybackDetail()).toBeNull()
})
```

Expected: fail until `setPendingPlaybackDetail()` and `takePendingPlaybackDetail()` exist.

- [ ] **Step 2: Implement the smallest store changes**

```ts
const pendingPlaybackDetail = ref<CatalogDetail | null>(null)

function setPendingPlaybackDetail(detail: CatalogDetail | null) {
  pendingPlaybackDetail.value = detail
}

function takePendingPlaybackDetail() {
  const detail = pendingPlaybackDetail.value
  pendingPlaybackDetail.value = null
  return detail
}
```

Export both methods from the store and keep `pendingUnifiedEpisode` unchanged.

- [ ] **Step 3: Pass the snapshot from `VodDetail.vue` before routing**

```ts
function handlePlay(ue: UnifiedEpisode) {
  if (ue.sources.length === 0) return

  if (detailStore.item) {
    playerStore.setPendingPlaybackDetail({
      item: detailStore.item,
      episode_groups: detailStore.episodeGroups,
    })
  }

  playerStore.setPendingUnifiedEpisode(ue)
  const episode = ue.sources[0].episode
  router.push({
    path: `/player/vod/${itemId.value}`,
    query: {
      episode: episode.play_url,
      episodeId: String(episode.id),
      title: detailStore.item?.title ?? undefined,
    },
  })
}
```

- [ ] **Step 4: Run the targeted store test**

Run: `vitest run src/stores/__tests__/player.spec.ts -v`

Expected: PASS after the store methods and handoff are wired.

- [ ] **Step 5: Commit**

```bash
git add src/stores/player.ts src/views/VodDetail.vue src/stores/__tests__/player.spec.ts
git commit -m "feat: preserve playback context handoff"
```

### Task 2: Make the playback drawer non-blocking

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue`
- Test: `src/components/player/__tests__/PlaybackDrawer.spec.ts`

- [ ] **Step 1: Write the failing component test**

```ts
it('keeps series content visible while loading when content already exists', () => {
  const wrapper = mount(PlaybackDrawer, {
    props: {
      sources: attempts[0]!.candidates,
      currentIndex: 0,
      failedIndexes: [],
      status: '正在解析',
      unifiedEpisodes,
      currentNormalizedIndex: 3,
      itemType: 'series',
      episodeSourceAttempts: attempts,
      loading: true,
    },
  })

  expect(wrapper.find('.playback-loading').exists()).toBe(false)
  expect(wrapper.text()).toContain('本集播放源')
  expect(wrapper.text()).toContain('非凡线路')
  expect(wrapper.text()).toContain('量子线路')
}
```

Expected: fail because the current template still replaces the drawer with skeleton markup whenever `loading` is true.

- [ ] **Step 2: Relax the loading gate in `PlaybackDrawer.vue`**

```ts
const hasContent = computed(() =>
  props.sources.length > 0 ||
  (props.unifiedEpisodes?.length ?? 0) > 0 ||
  (props.episodeSourceAttempts?.length ?? 0) > 0
)
```

Then change the template so skeleton blocks only render when `loading && !hasContent`, while the episode grid, source list, URL panel, error text, and episode-source list remain visible whenever content already exists.

- [ ] **Step 3: Keep loading errors visible when content exists**

```vue
<div v-if="errorMessage && (!loading || hasContent)" class="error-display">
  {{ errorMessage }}
</div>
```

This keeps the actual failure reason visible even while the player is still resolving other sources.

- [ ] **Step 4: Run the drawer tests**

Run: `vitest run src/components/player/__tests__/PlaybackDrawer.spec.ts -v`

Expected: PASS after the skeleton gate is relaxed.

- [ ] **Step 5: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue src/components/player/__tests__/PlaybackDrawer.spec.ts
git commit -m "feat: keep playback drawer interactive during loading"
```

### Task 3: Centralize playback UI decisions and wire PlayerPage to the snapshot

**Files:**
- Create: `src/utils/playbackUi.ts`
- Modify: `src/views/PlayerPage.vue`
- Test: `src/utils/__tests__/playbackUi.spec.ts`

- [ ] **Step 1: Write the failing helper test**

```ts
import { describe, expect, it } from 'vitest'
import { resolvePlaybackPageTitle, shouldShowDrawerSkeleton } from '@/utils/playbackUi'

it('prefers the hydrated detail title over route fallback', () => {
  expect(resolvePlaybackPageTitle({
    detailTitle: '庆余年',
    routeTitle: '旧标题',
    episodeLabel: '第03集',
    sourceLabel: '非凡线路',
  })).toBe('庆余年 · 第03集')
})

it('does not show drawer skeleton when content already exists', () => {
  expect(shouldShowDrawerSkeleton({
    loading: true,
    hasContent: true,
  })).toBe(false)
})
```

Expected: fail until the helper exists.

- [ ] **Step 2: Add the helper module**

```ts
import { formatPlayerTitle } from '@/utils/player'

export function resolvePlaybackPageTitle(input: {
  detailTitle?: string | null
  routeTitle?: string | null
  episodeLabel?: string | null
  sourceLabel?: string | null
}) {
  return formatPlayerTitle({
    title: input.detailTitle ?? input.routeTitle ?? null,
    episodeLabel: input.episodeLabel ?? null,
    sourceLabel: input.sourceLabel ?? null,
  })
}

export function shouldShowDrawerSkeleton(input: { loading: boolean; hasContent: boolean }) {
  return input.loading && !input.hasContent
}
```

Keep the helper small and pure so the page logic is easy to reason about and test.

- [ ] **Step 3: Wire `PlayerPage.vue` to hydrate from the pending snapshot**

```ts
const pendingPlaybackDetail = playerStore.takePendingPlaybackDetail()
if (pendingPlaybackDetail) {
  detailStore.item = pendingPlaybackDetail.item
  detailStore.episodeGroups = pendingPlaybackDetail.episode_groups
  detailStore.error = null
  detailStore.loading = false
  activeGroup.value = pendingPlaybackDetail.episode_groups[0] ?? null
}
```

Use this before the async detail fetch so the header can render immediately on entry.

- [ ] **Step 4: Use the helper for title and loading decisions**

```ts
const pageTitle = computed(() =>
  resolvePlaybackPageTitle({
    detailTitle: detailStore.item?.title ?? null,
    routeTitle: sourceTitle.value ?? null,
    episodeLabel: currentEpisodeLabel.value,
    sourceLabel: currentSource.value?.label ?? null,
  })
)

const drawerLoading = computed(() =>
  shouldShowDrawerSkeleton({
    loading: isInitialLoading.value ||
      detailStore.loading ||
      playbackStore.status === 'resolving' ||
      currentEpisodeSourceAttempts.value.some(attempt => attempt.status === 'resolving'),
    hasContent:
      unifiedEpisodes.value.length > 0 ||
      sources.value.length > 0 ||
      currentEpisodeSourceAttempts.value.length > 0,
  })
)
```

The important part is that `hasContent` wins over background loading, so the drawer stays interactive during failover and detail refreshes.

- [ ] **Step 5: Run the helper tests**

Run: `vitest run src/utils/__tests__/playbackUi.spec.ts -v`

Expected: PASS after the helper and PlayerPage wiring are in place.

- [ ] **Step 6: Commit**

```bash
git add src/utils/playbackUi.ts src/views/PlayerPage.vue src/utils/__tests__/playbackUi.spec.ts
git commit -m "feat: stabilize player title and drawer state"
```

### Task 4: Verify failover behavior and run the full suite

**Files:**
- Test: `src/utils/__tests__/playbackSession.spec.ts`
- Test: `src/utils/__tests__/playbackUi.spec.ts`
- Test: `src/components/player/__tests__/PlaybackDrawer.spec.ts`
- Test: `src/stores/__tests__/player.spec.ts`

- [ ] **Step 1: Add one regression check for automatic failover**

```ts
import {
  attachCandidatesToActiveSource,
  createEpisodePlaybackSession,
  markCurrentCandidateFailed,
  nextCandidateToPlay,
  startNextSourceAttempt,
} from '@/utils/playbackSession'

it('keeps trying the next source after a playback failure', () => {
  const session = createEpisodePlaybackSession(episode())
  const firstAttempt = startNextSourceAttempt(session)
  expect(firstAttempt?.source.sourceKey).toBe('slow')

  attachCandidatesToActiveSource(session, [
    { url: 'https://cdn.example/a.m3u8', label: '候选1', kind: 'hls' },
    { url: 'https://cdn.example/b.m3u8', label: '候选2', kind: 'hls' },
  ])

  expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/a.m3u8')
  markCurrentCandidateFailed(session, 'manifest failed')
  expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/b.m3u8')
  markCurrentCandidateFailed(session, 'segment failed')
  expect(startNextSourceAttempt(session)?.source.sourceKey).toBe('fast')
})
```

This should match the existing playback-session test style and prove the source chain keeps advancing after failures.

- [ ] **Step 2: Run the targeted tests in sequence**

Run:

```bash
vitest run src/stores/__tests__/player.spec.ts src/components/player/__tests__/PlaybackDrawer.spec.ts src/utils/__tests__/playbackUi.spec.ts src/utils/__tests__/playbackSession.spec.ts -v
```

Expected: PASS.

- [ ] **Step 3: Run the project verification commands**

Run:

```bash
npm run test
npm run build
```

Expected: both commands succeed with no TypeScript or Vitest regressions.

- [ ] **Step 4: Commit**

```bash
git add src/stores/player.ts src/views/VodDetail.vue src/components/player/PlaybackDrawer.vue src/utils/playbackUi.ts src/stores/__tests__/player.spec.ts src/components/player/__tests__/PlaybackDrawer.spec.ts src/utils/__tests__/playbackUi.spec.ts src/utils/__tests__/playbackSession.spec.ts
git commit -m "feat: preserve playback context and non-blocking failover"
```
