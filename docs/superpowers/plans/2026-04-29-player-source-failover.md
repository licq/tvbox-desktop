# Player Source Failover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make same-episode multi-source playback automatically fail over to the next usable source while exposing manual source switching and current title context.

**Architecture:** Add a focused frontend playback session module that owns source attempt state, candidate advancement, and in-memory health. `PlayerPage.vue` will use that module as the single authority for "what to try next" while continuing to own HLS/video wiring. `PlaybackDrawer.vue` will render current episode sources and emit source-switch/retry actions.

**Tech Stack:** Vue 3 Composition API, Pinia, Vue Router, Vitest, Tauri invoke APIs, hls.js.

---

## File Structure

- Create: `src/utils/playbackSession.ts` - pure TypeScript playback orchestration, source/candidate state, in-memory health cache helpers.
- Create: `src/utils/__tests__/playbackSession.spec.ts` - TDD coverage for ordering, failover, manual retry, and autoplay handling decisions.
- Modify: `src/types/index.ts` - shared `PlaybackSourceAttempt` type used by drawer and player.
- Modify: `src/views/PlayerPage.vue` - integrate session orchestration, title computation, source retry/switch events.
- Modify: `src/components/player/PlaybackDrawer.vue` - show same-episode source list in series mode and emit source selection/retry.
- Create or modify: `src/components/player/__tests__/PlaybackDrawer.spec.ts` - component coverage for episode source states and emitted events.

## Task 1: Playback Session Model

**Files:**
- Create: `src/utils/playbackSession.ts`
- Create: `src/utils/__tests__/playbackSession.spec.ts`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Write failing tests for source ordering and state initialization**

Create `src/utils/__tests__/playbackSession.spec.ts`:

```ts
import { describe, expect, it, beforeEach } from 'vitest'
import type { PlaybackCandidate, UnifiedEpisode } from '@/types'
import {
  createEpisodePlaybackSession,
  clearPlaybackHealth,
  markPlaybackHealth,
} from '@/utils/playbackSession'

function episode(): UnifiedEpisode {
  return {
    normalizedIndex: 3,
    displayLabel: '第3集',
    sources: [
      {
        sourceKey: 'slow',
        sourceName: '慢线路',
        episode: { id: 31, episode_label: '第03集', play_url: 'https://slow.example/play', order_index: 0 },
      },
      {
        sourceKey: 'fast',
        sourceName: '快线路',
        episode: { id: 32, episode_label: '第03集', play_url: 'https://fast.example/play', order_index: 1 },
      },
      {
        sourceKey: 'bad',
        sourceName: '坏线路',
        episode: { id: 33, episode_label: '第03集', play_url: 'https://bad.example/play', order_index: 2 },
      },
    ],
  }
}

describe('playback session', () => {
  beforeEach(() => clearPlaybackHealth())

  it('orders recently successful sources before unknown and failed sources', () => {
    const ep = episode()
    markPlaybackHealth({ scope: 'source', key: 'fast|https://fast.example/play', status: 'success' })
    markPlaybackHealth({ scope: 'source', key: 'bad|https://bad.example/play', status: 'failed', reason: 'manifest failed' })

    const session = createEpisodePlaybackSession(ep)

    expect(session.sourceAttempts.map(attempt => attempt.source.sourceKey)).toEqual(['fast', 'slow', 'bad'])
    expect(session.sourceAttempts[0]?.status).toBe('idle')
    expect(session.sourceAttempts[2]?.status).toBe('skipped')
  })

  it('keeps original source order when no health is known', () => {
    const session = createEpisodePlaybackSession(episode())

    expect(session.sourceAttempts.map(attempt => attempt.source.sourceName)).toEqual(['慢线路', '快线路', '坏线路'])
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- src/utils/__tests__/playbackSession.spec.ts`

Expected: FAIL because `@/utils/playbackSession` does not exist.

- [ ] **Step 3: Add shared types**

Modify `src/types/index.ts` near playback types:

```ts
export type PlaybackSourceAttemptStatus =
  | 'idle'
  | 'resolving'
  | 'playable'
  | 'playing'
  | 'failed'
  | 'skipped'

export interface PlaybackSourceAttempt {
  source: UnifiedEpisodeSource
  status: PlaybackSourceAttemptStatus
  candidates: PlaybackCandidate[]
  failedCandidateIndexes: number[]
  failureReason?: string
  lastTriedAt?: number
}
```

- [ ] **Step 4: Implement minimal session creation and health cache**

Create `src/utils/playbackSession.ts`:

```ts
import type {
  PlaybackCandidate,
  PlaybackSourceAttempt,
  PlaybackSourceAttemptStatus,
  UnifiedEpisode,
} from '@/types'

type HealthStatus = 'success' | 'failed'
type HealthScope = 'source' | 'candidate'

interface PlaybackHealthEntry {
  scope: HealthScope
  key: string
  status: HealthStatus
  reason?: string
  checkedAt: number
}

export interface PlaybackHealthInput {
  scope: HealthScope
  key: string
  status: HealthStatus
  reason?: string
}

export interface EpisodePlaybackSession {
  episode: UnifiedEpisode
  sourceAttempts: PlaybackSourceAttempt[]
  activeSourceIndex: number
  activeCandidateIndex: number
  status: 'idle' | 'resolving' | 'playing' | 'failed'
  lastError?: string
}

const playbackHealth = new Map<string, PlaybackHealthEntry>()

function healthMapKey(scope: HealthScope, key: string) {
  return `${scope}:${key}`
}

export function sourceHealthKey(sourceKey: string, playUrl: string) {
  return `${sourceKey}|${playUrl}`
}

export function candidateHealthKey(candidate: PlaybackCandidate) {
  const headers = candidate.headers ? JSON.stringify(candidate.headers) : ''
  return `${candidate.url}|${candidate.referer ?? ''}|${headers}`
}

export function markPlaybackHealth(input: PlaybackHealthInput) {
  playbackHealth.set(healthMapKey(input.scope, input.key), {
    ...input,
    checkedAt: Date.now(),
  })
}

export function getPlaybackHealth(scope: HealthScope, key: string) {
  return playbackHealth.get(healthMapKey(scope, key)) ?? null
}

export function clearPlaybackHealth() {
  playbackHealth.clear()
}

function sourceRank(attempt: PlaybackSourceAttempt) {
  const key = sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url)
  const health = getPlaybackHealth('source', key)
  if (health?.status === 'success') return 0
  if (health?.status === 'failed') return 2
  return 1
}

function statusForSource(sourceKey: string, playUrl: string): PlaybackSourceAttemptStatus {
  const health = getPlaybackHealth('source', sourceHealthKey(sourceKey, playUrl))
  return health?.status === 'failed' ? 'skipped' : 'idle'
}

export function createEpisodePlaybackSession(episode: UnifiedEpisode): EpisodePlaybackSession {
  const sourceAttempts = episode.sources
    .map<PlaybackSourceAttempt>(source => ({
      source,
      status: statusForSource(source.sourceKey, source.episode.play_url),
      candidates: [],
      failedCandidateIndexes: [],
      failureReason: getPlaybackHealth('source', sourceHealthKey(source.sourceKey, source.episode.play_url))?.reason,
    }))
    .map((attempt, originalIndex) => ({ attempt, originalIndex }))
    .sort((a, b) => sourceRank(a.attempt) - sourceRank(b.attempt) || a.originalIndex - b.originalIndex)
    .map(({ attempt }) => attempt)

  return {
    episode,
    sourceAttempts,
    activeSourceIndex: -1,
    activeCandidateIndex: -1,
    status: 'idle',
  }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `npm run test -- src/utils/__tests__/playbackSession.spec.ts`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/types/index.ts src/utils/playbackSession.ts src/utils/__tests__/playbackSession.spec.ts
git commit -m "feat: add playback source session model"
```

## Task 2: Candidate Advancement and Failure Decisions

**Files:**
- Modify: `src/utils/playbackSession.ts`
- Modify: `src/utils/__tests__/playbackSession.spec.ts`

- [ ] **Step 1: Add failing tests for candidate advancement**

Modify the existing import from `@/utils/playbackSession` in `src/utils/__tests__/playbackSession.spec.ts` so it includes the new helpers:

```ts
import {
  attachCandidatesToActiveSource,
  createEpisodePlaybackSession,
  clearPlaybackHealth,
  markCurrentCandidateFailed,
  markPlaybackHealth,
  nextCandidateToPlay,
  startNextSourceAttempt,
  shouldFailoverAfterPlaybackError,
} from '@/utils/playbackSession'
```

Then append the candidate fixtures and tests after the existing `describe('playback session', ...)` block:

```ts

const candidates: PlaybackCandidate[] = [
  { url: 'https://cdn.example/a.m3u8', label: '候选1', kind: 'hls' },
  { url: 'https://cdn.example/b.m3u8', label: '候选2', kind: 'hls' },
]

describe('playback session advancement', () => {
  beforeEach(() => clearPlaybackHealth())

  it('tries another candidate in the same source before moving to the next source', () => {
    const session = createEpisodePlaybackSession(episode())
    const firstAttempt = startNextSourceAttempt(session)
    expect(firstAttempt?.source.sourceKey).toBe('slow')

    attachCandidatesToActiveSource(session, candidates)
    expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/a.m3u8')

    markCurrentCandidateFailed(session, 'manifest failed')
    expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/b.m3u8')

    markCurrentCandidateFailed(session, 'segment failed')
    const nextAttempt = startNextSourceAttempt(session)
    expect(nextAttempt?.source.sourceKey).toBe('fast')
  })

  it('does not fail over for autoplay blocking', () => {
    expect(shouldFailoverAfterPlaybackError({ name: 'NotAllowedError' })).toBe(false)
    expect(shouldFailoverAfterPlaybackError(new Error('NotAllowedError: play() failed'))).toBe(false)
    expect(shouldFailoverAfterPlaybackError(new Error('media decode failed'))).toBe(true)
  })

  it('allows manual attempts for a skipped failed source', () => {
    const ep = episode()
    markPlaybackHealth({ scope: 'source', key: 'bad|https://bad.example/play', status: 'failed', reason: 'previous failure' })
    const session = createEpisodePlaybackSession(ep)

    const manual = startNextSourceAttempt(session, { sourceKey: 'bad', manual: true })

    expect(manual?.source.sourceKey).toBe('bad')
    expect(manual?.status).toBe('resolving')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- src/utils/__tests__/playbackSession.spec.ts`

Expected: FAIL because advancement helpers do not exist.

- [ ] **Step 3: Implement advancement helpers**

Append to `src/utils/playbackSession.ts`:

```ts
interface StartSourceOptions {
  sourceKey?: string
  manual?: boolean
}

export function startNextSourceAttempt(
  session: EpisodePlaybackSession,
  options: StartSourceOptions = {}
) {
  const index = options.sourceKey
    ? session.sourceAttempts.findIndex(attempt => attempt.source.sourceKey === options.sourceKey)
    : session.sourceAttempts.findIndex((attempt, attemptIndex) =>
        attemptIndex > session.activeSourceIndex &&
        (options.manual || attempt.status !== 'failed') &&
        (options.manual || attempt.status !== 'skipped')
      )

  if (index < 0) {
    session.status = 'failed'
    session.lastError = session.lastError ?? '该集所有播放源均不可用'
    return null
  }

  const attempt = session.sourceAttempts[index]
  if (!attempt) return null

  session.activeSourceIndex = index
  session.activeCandidateIndex = -1
  session.status = 'resolving'
  attempt.status = 'resolving'
  attempt.lastTriedAt = Date.now()
  return attempt
}

export function attachCandidatesToActiveSource(
  session: EpisodePlaybackSession,
  candidates: PlaybackCandidate[]
) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return

  attempt.candidates = candidates
  attempt.failedCandidateIndexes = []
  attempt.status = candidates.length > 0 ? 'playable' : 'failed'
  if (candidates.length === 0) {
    attempt.failureReason = '当前源没有可用候选线路'
  }
}

export function nextCandidateToPlay(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return null

  const nextIndex = attempt.candidates.findIndex((_, index) =>
    index > session.activeCandidateIndex && !attempt.failedCandidateIndexes.includes(index)
  )

  if (nextIndex < 0) return null

  session.activeCandidateIndex = nextIndex
  session.status = 'playing'
  attempt.status = 'playing'
  return attempt.candidates[nextIndex] ?? null
}

export function markCurrentCandidateFailed(session: EpisodePlaybackSession, reason: string) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return

  if (
    session.activeCandidateIndex >= 0 &&
    !attempt.failedCandidateIndexes.includes(session.activeCandidateIndex)
  ) {
    attempt.failedCandidateIndexes = [...attempt.failedCandidateIndexes, session.activeCandidateIndex]
    const candidate = attempt.candidates[session.activeCandidateIndex]
    if (candidate) {
      markPlaybackHealth({
        scope: 'candidate',
        key: candidateHealthKey(candidate),
        status: 'failed',
        reason,
      })
    }
  }

  const hasRemainingCandidate = attempt.candidates.some((_, index) =>
    !attempt.failedCandidateIndexes.includes(index)
  )

  if (!hasRemainingCandidate) {
    attempt.status = 'failed'
    attempt.failureReason = reason
    session.lastError = reason
    markPlaybackHealth({
      scope: 'source',
      key: sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url),
      status: 'failed',
      reason,
    })
  }
}

export function markCurrentCandidatePlaying(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  const candidate = attempt?.candidates[session.activeCandidateIndex]
  if (!attempt || !candidate) return

  session.status = 'playing'
  attempt.status = 'playing'
  markPlaybackHealth({
    scope: 'source',
    key: sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url),
    status: 'success',
  })
  markPlaybackHealth({
    scope: 'candidate',
    key: candidateHealthKey(candidate),
    status: 'success',
  })
}

export function shouldFailoverAfterPlaybackError(error: unknown) {
  const name = typeof error === 'object' && error !== null && 'name' in error
    ? String((error as { name?: unknown }).name)
    : ''
  const message = error instanceof Error ? error.message : String(error)
  return !name.includes('NotAllowedError') && !message.includes('NotAllowedError')
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- src/utils/__tests__/playbackSession.spec.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/utils/playbackSession.ts src/utils/__tests__/playbackSession.spec.ts
git commit -m "feat: add playback failover decisions"
```

## Task 3: Player Page Integration

**Files:**
- Modify: `src/views/PlayerPage.vue`
- Test: `src/utils/__tests__/playbackSession.spec.ts`

- [ ] **Step 1: Add helper imports and session state**

Modify imports in `src/views/PlayerPage.vue`:

```ts
import {
  attachCandidatesToActiveSource,
  createEpisodePlaybackSession,
  markCurrentCandidateFailed,
  markCurrentCandidatePlaying,
  nextCandidateToPlay,
  shouldFailoverAfterPlaybackError,
  startNextSourceAttempt,
  type EpisodePlaybackSession,
} from '@/utils/playbackSession'
```

Add near existing refs:

```ts
const playbackSession = ref<EpisodePlaybackSession | null>(null)
const currentEpisodeSourceAttempts = computed(() => playbackSession.value?.sourceAttempts ?? [])
```

- [ ] **Step 2: Add session-driven playback helpers**

Add in `src/views/PlayerPage.vue` near `playUnifiedEpisode`:

```ts
async function resolveActiveAttempt(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return false

  try {
    if (itemId.value > 0) {
      const resolved = await playbackStore.resolve(attempt.source.episode.play_url, attempt.source.episode.id)
      attachCandidatesToActiveSource(session, resolved.candidates.map(candidate => ({
        url: candidate.url,
        label: candidate.label,
        kind: candidate.kind,
        headers: candidate.headers,
        referer: candidate.referer,
      })))
    } else if (sourceName.value) {
      const targets = await invoke<PlaybackTarget[]>('provider_play', {
        source: sourceName.value,
        flag: 'auto',
        playUrl: attempt.source.episode.play_url,
      })
      const target = targets[0]
      if (!target) {
        attachCandidatesToActiveSource(session, [])
      } else if (target.target_kind === 'Direct') {
        attachCandidatesToActiveSource(session, [{
          url: target.target_url,
          label: attempt.source.sourceName,
          kind: target.target_url.includes('.m3u8') ? 'hls' : 'http',
          headers: target.headers ?? undefined,
          referer: target.referer ?? undefined,
        }])
      } else {
        const resolved = await playbackStore.resolve(target.target_url, attempt.source.episode.id)
        attachCandidatesToActiveSource(session, resolved.candidates.map(candidate => ({
          url: candidate.url,
          label: candidate.label,
          kind: candidate.kind,
          headers: candidate.headers,
          referer: candidate.referer,
        })))
      }
    }
    return true
  } catch (error) {
    attempt.status = 'failed'
    attempt.failureReason = String(error)
    return false
  }
}

async function playNextFromSession(reason?: string) {
  const session = playbackSession.value
  if (!session) return

  if (reason) {
    markCurrentCandidateFailed(session, reason)
  }

  const sameSourceCandidate = nextCandidateToPlay(session)
  if (sameSourceCandidate) {
    sources.value = session.sourceAttempts[session.activeSourceIndex]?.candidates ?? []
    currentSourceIndex.value = session.activeCandidateIndex
    await playSource(sameSourceCandidate)
    return
  }

  const nextAttempt = startNextSourceAttempt(session)
  if (!nextAttempt) {
    errorMsg.value = session.lastError ?? '该集所有播放源均不可用'
    return
  }

  const resolved = await resolveActiveAttempt(session)
  if (!resolved) {
    await playNextFromSession(nextAttempt.failureReason ?? '解析失败')
    return
  }

  const candidate = nextCandidateToPlay(session)
  if (!candidate) {
    await playNextFromSession('当前源没有可用候选线路')
    return
  }

  sources.value = nextAttempt.candidates
  currentSourceIndex.value = session.activeCandidateIndex
  errorMsg.value = ''
  await playSource(candidate)
}
```

- [ ] **Step 3: Replace `playUnifiedEpisode` with session startup**

Replace the body of `playUnifiedEpisode` in `src/views/PlayerPage.vue`:

```ts
async function playUnifiedEpisode(unifiedEpisode: UnifiedEpisode, sourceIndex = 0) {
  currentUnifiedEpisode.value = unifiedEpisode
  currentUnifiedSourceIndex.value = sourceIndex
  playbackSession.value = createEpisodePlaybackSession(unifiedEpisode)

  const preferredSource = unifiedEpisode.sources[sourceIndex]
  const attempt = preferredSource
    ? startNextSourceAttempt(playbackSession.value, { sourceKey: preferredSource.sourceKey, manual: sourceIndex > 0 })
    : startNextSourceAttempt(playbackSession.value)

  if (!attempt) {
    errorMsg.value = '该集所有线路均不可用'
    return
  }

  const resolved = await resolveActiveAttempt(playbackSession.value)
  if (!resolved) {
    await playNextFromSession(attempt.failureReason ?? '解析失败')
    return
  }

  const candidate = nextCandidateToPlay(playbackSession.value)
  if (!candidate) {
    await playNextFromSession('当前源没有可用候选线路')
    return
  }

  sources.value = attempt.candidates
  currentSourceIndex.value = playbackSession.value.activeCandidateIndex
  failedSourceIndexes.value = []
  await playSource(candidate)
}
```

- [ ] **Step 4: Update playback success and failure hooks**

In `attemptPlayback`, replace the catch branch source failure behavior with:

```ts
  } catch (error) {
    playing.value = false
    pendingAutoplay.value = false
    errorMsg.value = describePlaybackFailure(error)
    if (!manual && isAutoplayBlocked(error)) {
      return
    }
    if (shouldFailoverAfterPlaybackError(error)) {
      await playNextFromSession(describePlaybackFailure(error))
    }
  }
```

In `handleVideoPlay`, add:

```ts
  if (playbackSession.value) {
    markCurrentCandidatePlaying(playbackSession.value)
  }
```

In HLS fatal error and `handleVideoError`, replace direct `switchToSource` / `playUnifiedEpisode` branches with:

```ts
void playNextFromSession(data.error?.message || 'HLS 播放失败')
```

and:

```ts
void playNextFromSession(message)
```

- [ ] **Step 5: Run focused tests and typecheck**

Run: `npm run test -- src/utils/__tests__/playbackSession.spec.ts`

Expected: PASS.

Run: `npm run build`

Expected: PASS TypeScript check and Vite build.

- [ ] **Step 6: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: route vod playback through source session"
```

## Task 4: Playback Drawer Source Controls

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue`
- Create: `src/components/player/__tests__/PlaybackDrawer.spec.ts`
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Write failing drawer tests**

Create `src/components/player/__tests__/PlaybackDrawer.spec.ts`:

```ts
import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PlaybackDrawer from '@/components/player/PlaybackDrawer.vue'
import type { PlaybackSourceAttempt, UnifiedEpisode } from '@/types'

const unifiedEpisodes: UnifiedEpisode[] = [{
  normalizedIndex: 3,
  displayLabel: '第3集',
  sources: [
    { sourceKey: 'a', sourceName: '非凡线路', episode: { id: 1, episode_label: '第03集', play_url: 'a', order_index: 0 } },
    { sourceKey: 'b', sourceName: '量子线路', episode: { id: 2, episode_label: '第03集', play_url: 'b', order_index: 1 } },
  ],
}]

const attempts: PlaybackSourceAttempt[] = [
  {
    source: unifiedEpisodes[0]!.sources[0]!,
    status: 'playing',
    candidates: [{ url: 'https://a.example/1.m3u8', label: 'HLS', kind: 'hls' }],
    failedCandidateIndexes: [],
  },
  {
    source: unifiedEpisodes[0]!.sources[1]!,
    status: 'failed',
    candidates: [],
    failedCandidateIndexes: [],
    failureReason: 'manifest failed',
  },
]

describe('PlaybackDrawer', () => {
  it('renders current episode source attempts in series mode', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放中',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: attempts,
      },
    })

    expect(wrapper.text()).toContain('本集播放源')
    expect(wrapper.text()).toContain('非凡线路')
    expect(wrapper.text()).toContain('当前播放')
    expect(wrapper.text()).toContain('量子线路')
    expect(wrapper.text()).toContain('manifest failed')
  })

  it('emits switchEpisodeSource when clicking an episode source', async () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放中',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: attempts,
      },
    })

    await wrapper.find('[data-testid="episode-source-b"]').trigger('click')

    expect(wrapper.emitted('switchEpisodeSource')?.[0]).toEqual(['b'])
  })
})
```

- [ ] **Step 2: Run drawer tests to verify they fail**

Run: `npm run test -- src/components/player/__tests__/PlaybackDrawer.spec.ts`

Expected: FAIL because the prop and emit do not exist.

- [ ] **Step 3: Update drawer props and emits**

Modify `src/components/player/PlaybackDrawer.vue` imports and props:

```ts
import type { CatalogItemType, PlaybackSourceAttempt, PlayerSource, UnifiedEpisode } from '@/types'
```

Add prop:

```ts
  episodeSourceAttempts?: PlaybackSourceAttempt[]
```

Add emit:

```ts
  switchEpisodeSource: [sourceKey: string]
```

Add helper:

```ts
function attemptStatusLabel(status: PlaybackSourceAttempt['status']) {
  if (status === 'playing') return '当前播放'
  if (status === 'resolving') return '解析中'
  if (status === 'failed') return '本次失败'
  if (status === 'skipped') return '最近失败'
  if (status === 'playable') return '可播放'
  return '待探测'
}
```

- [ ] **Step 4: Render episode source list in series mode**

In `PlaybackDrawer.vue`, after the episode grid in series mode, add:

```vue
      <div v-if="isSeries && episodeSourceAttempts?.length" class="episode-source-list">
        <div class="episode-source-title">本集播放源</div>
        <button
          v-for="attempt in episodeSourceAttempts"
          :key="attempt.source.sourceKey"
          :data-testid="`episode-source-${attempt.source.sourceKey}`"
          :class="[
            'source-row',
            attempt.status === 'playing' ? 'source-row-active' : '',
            attempt.status === 'failed' || attempt.status === 'skipped' ? 'source-row-failed' : ''
          ]"
          type="button"
          @click="emit('switchEpisodeSource', attempt.source.sourceKey)"
        >
          <span class="source-row-label">{{ attempt.source.sourceName }}</span>
          <span class="source-row-meta">
            {{ attempt.failureReason || attemptStatusLabel(attempt.status) }}
          </span>
        </button>
      </div>
```

Add scoped styles:

```css
.episode-source-list {
  margin-top: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.episode-source-title {
  font-size: 0.72rem;
  color: var(--text-muted);
  letter-spacing: 0.04em;
}

.source-row-meta {
  color: var(--text-muted);
  font-size: 0.7rem;
  text-align: right;
}
```

- [ ] **Step 5: Wire drawer to player**

In `src/views/PlayerPage.vue`, add prop and event:

```vue
        <PlaybackDrawer
          :sources="sources"
          :current-index="currentSourceIndex"
          :failed-indexes="failedSourceIndexes"
          :status="playerStatusText"
          :error-message="errorMsg || playbackStore.errorMessage"
          :unified-episodes="unifiedEpisodes"
          :current-normalized-index="currentNormalizedIndex"
          :item-type="itemType"
          :episode-source-attempts="currentEpisodeSourceAttempts"
          @select-episode="switchToEpisode"
          @switch-line="switchToSource"
          @switch-episode-source="switchEpisodeSource"
        />
```

Add handler:

```ts
async function switchEpisodeSource(sourceKey: string) {
  const session = playbackSession.value
  if (!session) return

  const attempt = startNextSourceAttempt(session, { sourceKey, manual: true })
  if (!attempt) return

  const resolved = await resolveActiveAttempt(session)
  if (!resolved) {
    errorMsg.value = attempt.failureReason ?? '解析失败'
    return
  }

  const candidate = nextCandidateToPlay(session)
  if (!candidate) {
    errorMsg.value = '当前源没有可用候选线路'
    return
  }

  sources.value = attempt.candidates
  currentSourceIndex.value = session.activeCandidateIndex
  errorMsg.value = ''
  await playSource(candidate)
}
```

- [ ] **Step 6: Run drawer and build verification**

Run: `npm run test -- src/components/player/__tests__/PlaybackDrawer.spec.ts`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue src/components/player/__tests__/PlaybackDrawer.spec.ts src/views/PlayerPage.vue
git commit -m "feat: expose episode source switching"
```

## Task 5: Player Title Context

**Files:**
- Modify: `src/views/PlayerPage.vue`
- Test: `src/utils/__tests__/player.spec.ts`
- Modify: `src/utils/player.ts`

- [ ] **Step 1: Add failing title helper tests**

Append to `src/utils/__tests__/player.spec.ts`:

```ts
import { formatPlayerTitle } from '@/utils/player'

describe('player title formatting', () => {
  it('formats series title with episode label', () => {
    expect(formatPlayerTitle({ title: '庆余年', episodeLabel: '第03集' })).toBe('庆余年 · 第03集')
  })

  it('falls back to episode label when title is missing', () => {
    expect(formatPlayerTitle({ episodeLabel: '第03集' })).toBe('第03集')
  })

  it('uses source label only as a final fallback', () => {
    expect(formatPlayerTitle({ sourceLabel: '非凡线路' })).toBe('非凡线路')
  })
})
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `npm run test -- src/utils/__tests__/player.spec.ts`

Expected: FAIL because `formatPlayerTitle` does not exist.

- [ ] **Step 3: Implement title helper**

Append to `src/utils/player.ts`:

```ts
export interface PlayerTitleInput {
  title?: string | null
  episodeLabel?: string | null
  sourceLabel?: string | null
}

export function formatPlayerTitle(input: PlayerTitleInput) {
  const title = input.title?.trim()
  const episodeLabel = input.episodeLabel?.trim()
  const sourceLabel = input.sourceLabel?.trim()

  if (title && episodeLabel) return `${title} · ${episodeLabel}`
  if (title) return title
  if (episodeLabel) return episodeLabel
  if (sourceLabel) return sourceLabel
  return 'TVBox'
}
```

- [ ] **Step 4: Wire title into PlayerPage**

Modify `src/views/PlayerPage.vue` imports:

```ts
import { describeMediaErrorCode, describePlaybackFailure, formatPlayerTitle, isAutoplayBlocked } from '@/utils/player'
```

Add computed:

```ts
const currentEpisodeLabel = computed(() => {
  if (currentUnifiedEpisode.value?.displayLabel) return currentUnifiedEpisode.value.displayLabel
  return episodeLabelFromQuery.value ?? null
})

const pageTitle = computed(() =>
  formatPlayerTitle({
    title: detailStore.item?.title ?? sourceTitle.value ?? null,
    episodeLabel: currentEpisodeLabel.value,
    sourceLabel: currentSource.value?.label ?? null,
  })
)
```

Add watcher import and effect:

```ts
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
```

```ts
watch(pageTitle, title => {
  document.title = title
}, { immediate: true })
```

Update the topbar template to show the title:

```vue
        <div class="player-title">
          <strong>{{ pageTitle }}</strong>
        </div>
```

- [ ] **Step 5: Run tests and build**

Run: `npm run test -- src/utils/__tests__/player.spec.ts`

Expected: PASS.

Run: `npm run build`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/utils/player.ts src/utils/__tests__/player.spec.ts src/views/PlayerPage.vue
git commit -m "feat: show current media title in player"
```

## Task 6: Final Verification

**Files:**
- No planned source edits unless verification exposes defects.

- [ ] **Step 1: Run all frontend tests**

Run: `npm run test`

Expected: PASS.

- [ ] **Step 2: Run production build**

Run: `npm run build`

Expected: PASS TypeScript check and Vite build.

- [ ] **Step 3: Inspect git diff for accidental generated files**

Run: `git status --short`

Expected: only intentional source/test/doc changes remain, with no `.superpowers/`, `src-tauri/target`, or build output staged.

- [ ] **Step 4: Manual playback smoke test**

Run: `npm run tauri dev`

Expected:

- Open a VOD item with a same-episode multi-source list.
- Start playback for an episode with multiple sources.
- Force the current source to fail by choosing a known bad source or temporarily disconnecting the manifest URL.
- Player advances to the next source without getting stuck.
- Right drawer shows failed source state and lets the user retry it manually.
- Page title shows `片名 · 集数`.

- [ ] **Step 5: Commit final fixes if any**

If verification required fixes:

```bash
git add src
git commit -m "fix: stabilize player source failover"
```

If no fixes were needed, do not create an empty commit.
