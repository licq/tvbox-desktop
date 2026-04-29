# Episode Deduplication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge duplicate episodes across sources in the catalog flow, showing 40 buttons for a 40-episode series instead of 80. Auto-switch sources on playback failure.

**Architecture:** Add episode label normalization utilities (`extractEpisodeIndex`, `mergeEpisodes`), new `UnifiedEpisode` type, modify `EpisodeGroupPanel` and `PlaybackDrawer` to show deduplicated episodes, and update `PlayerPage` to try multiple sources sequentially on failure.

**Tech Stack:** Vue 3, TypeScript, Pinia, Vitest

---

## File Map

| File | Action | Responsibility |
|------|--------|--------------|
| `src/types/index.ts` | Modify | Add `UnifiedEpisode`, `UnifiedEpisodeSource` interfaces |
| `src/utils/episode.ts` | Create | `extractEpisodeIndex`, `mergeEpisodes`, `formatDisplayLabel` |
| `src/utils/__tests__/episode.spec.ts` | Create | Unit tests for episode utilities |
| `src/components/media/EpisodeChip.vue` | Modify | Add optional `sourceCount` prop + badge |
| `src/components/detail/EpisodeGroupPanel.vue` | Modify | Accept `groups[]`, compute `unifiedEpisodes`, emit `UnifiedEpisode` |
| `src/components/detail/__tests__/EpisodeGroupPanel.spec.ts` | Modify | Update tests for deduplicated rendering |
| `src/components/player/PlaybackDrawer.vue` | Modify | Accept `unifiedEpisodes`, emit `selectUnifiedEpisode` |
| `src/views/VodDetail.vue` | Modify | `handlePlay` accepts `UnifiedEpisode`, store in `playerStore` |
| `src/stores/player.ts` | Modify | Add `pendingUnifiedEpisode` ref + setter |
| `src/views/PlayerPage.vue` | Modify | Auto-source-switching across `UnifiedEpisode.sources` |

---

### Task 1: Episode Label Normalization Utilities

**Files:**
- Create: `src/utils/episode.ts`
- Modify: `src/types/index.ts`
- Test: `src/utils/__tests__/episode.spec.ts`

- [ ] **Step 1: Add `UnifiedEpisode` types to `src/types/index.ts`**

Add after `CatalogEpisodeGroup`:

```ts
export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  episode: CatalogEpisode
}

export interface UnifiedEpisode {
  normalizedIndex: number
  displayLabel: string
  sources: UnifiedEpisodeSource[]
}
```

- [ ] **Step 2: Create `src/utils/__tests__/episode.spec.ts` with failing tests**

```ts
import { describe, it, expect } from 'vitest'
import { extractEpisodeIndex, mergeEpisodes, formatDisplayLabel } from '../episode'
import type { CatalogEpisodeGroup, CatalogItemType } from '@/types'

describe('extractEpisodeIndex', () => {
  it('extracts from 第1集', () => expect(extractEpisodeIndex('第1集')).toBe(1))
  it('extracts from 第01集', () => expect(extractEpisodeIndex('第01集')).toBe(1))
  it('extracts from 第1期', () => expect(extractEpisodeIndex('第1期')).toBe(1))
  it('extracts from S01E01', () => expect(extractEpisodeIndex('S01E01')).toBe(1))
  it('extracts from E01', () => expect(extractEpisodeIndex('E01')).toBe(1))
  it('extracts from pure number 01', () => expect(extractEpisodeIndex('01')).toBe(1))
  it('returns null for HD', () => expect(extractEpisodeIndex('HD')).toBeNull())
  it('returns null for 蓝光', () => expect(extractEpisodeIndex('蓝光')).toBeNull())
})

describe('mergeEpisodes', () => {
  const groups: CatalogEpisodeGroup[] = [
    {
      source_name: 'A',
      episodes: [
        { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
        { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
      ],
    },
    {
      source_name: 'B',
      episodes: [
        { id: 3, episode_label: '第01集', play_url: 'http://b/1', order_index: 1 },
        { id: 4, episode_label: '第02集', play_url: 'http://b/2', order_index: 2 },
      ],
    },
  ]

  it('merges duplicate episodes for series', () => {
    const result = mergeEpisodes(groups, 'series')
    expect(result).toHaveLength(2)
    expect(result[0].normalizedIndex).toBe(1)
    expect(result[0].sources).toHaveLength(2)
    expect(result[1].normalizedIndex).toBe(2)
  })

  it('sorts by normalizedIndex ascending', () => {
    const shuffled: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
          { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
        ],
      },
    ]
    const result = mergeEpisodes(shuffled, 'series')
    expect(result[0].normalizedIndex).toBe(1)
    expect(result[1].normalizedIndex).toBe(2)
  })

  it('does not merge for movies', () => {
    const movieGroups: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 1, episode_label: 'HD', play_url: 'http://a/hd', order_index: 1 },
          { id: 2, episode_label: '1080P', play_url: 'http://a/1080', order_index: 2 },
        ],
      },
    ]
    const result = mergeEpisodes(movieGroups, 'movie')
    expect(result).toHaveLength(2)
    expect(result[0].sources).toHaveLength(1)
  })

  it('treats unnormalizable labels as independent items', () => {
    const g: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 1, episode_label: '预告片', play_url: 'http://a/trailer', order_index: 1 },
        ],
      },
    ]
    const result = mergeEpisodes(g, 'series')
    expect(result).toHaveLength(1)
    expect(result[0].displayLabel).toBe('预告片')
  })
})

describe('formatDisplayLabel', () => {
  it('formats 第1集', () => expect(formatDisplayLabel('第1集')).toBe('第1集'))
  it('formats 第01集', () => expect(formatDisplayLabel('第01集')).toBe('第1集'))
  it('formats S01E01', () => expect(formatDisplayLabel('S01E01')).toBe('第1集'))
  it('formats 第1期 for variety', () => expect(formatDisplayLabel('第1期', 'variety')).toBe('第1期'))
  it('returns original for HD', () => expect(formatDisplayLabel('HD')).toBe('HD'))
})
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
vitest run src/utils/__tests__/episode.spec.ts
```

Expected: FAIL — functions not defined.

- [ ] **Step 4: Create `src/utils/episode.ts`**

```ts
import type { CatalogEpisodeGroup, CatalogItemType, UnifiedEpisode } from '@/types'

export function extractEpisodeIndex(label: string): number | null {
  const trimmed = label.trim()

  const chineseMatch = trimmed.match(/第\s*(\d+)\s*[集期]/)
  if (chineseMatch) return parseInt(chineseMatch[1], 10)

  const seasonMatch = trimmed.match(/S\d+E(\d+)/i)
  if (seasonMatch) return parseInt(seasonMatch[1], 10)

  const epMatch = trimmed.match(/^E(\d+)$/i)
  if (epMatch) return parseInt(epMatch[1], 10)

  const pureNum = trimmed.match(/^(\d+)$/)
  if (pureNum) return parseInt(pureNum[1], 10)

  return null
}

export function formatDisplayLabel(original: string, itemType?: CatalogItemType): string {
  const idx = extractEpisodeIndex(original)
  if (idx === null) return original
  const unit = itemType === 'variety' ? '期' : '集'
  return `第${idx}${unit}`
}

export function mergeEpisodes(
  groups: CatalogEpisodeGroup[],
  itemType: CatalogItemType
): UnifiedEpisode[] {
  if (itemType === 'movie') {
    return groups.flatMap(g =>
      g.episodes.map(ep => ({
        normalizedIndex: ep.id,
        displayLabel: ep.episode_label,
        sources: [{
          sourceKey: g.source_name,
          sourceName: g.source_name,
          episode: ep,
        }],
      }))
    )
  }

  const map = new Map<number, UnifiedEpisode>()

  for (const group of groups) {
    for (const ep of group.episodes) {
      const idx = extractEpisodeIndex(ep.episode_label)
      if (idx === null) {
        map.set(ep.id, {
          normalizedIndex: ep.id,
          displayLabel: ep.episode_label,
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            episode: ep,
          }],
        })
        continue
      }

      const existing = map.get(idx)
      if (existing) {
        existing.sources.push({
          sourceKey: group.source_name,
          sourceName: group.source_name,
          episode: ep,
        })
      } else {
        map.set(idx, {
          normalizedIndex: idx,
          displayLabel: formatDisplayLabel(ep.episode_label, itemType),
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            episode: ep,
          }],
        })
      }
    }
  }

  return Array.from(map.values()).sort((a, b) => a.normalizedIndex - b.normalizedIndex)
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
vitest run src/utils/__tests__/episode.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/types/index.ts src/utils/episode.ts src/utils/__tests__/episode.spec.ts
git commit -m "feat: add episode label normalization and merge utilities"
```

---

### Task 2: EpisodeChip Badge Support

**Files:**
- Modify: `src/components/media/EpisodeChip.vue`

- [ ] **Step 1: Add `sourceCount` prop and badge**

Modify `EpisodeChip.vue` props:

```ts
const props = defineProps<{
  label: string
  state: EpisodeAvailabilityState
  sourceCount?: number
}>()
```

Update template:

```vue
<template>
  <button class="episode-chip" :class="`episode-chip-${state}`">
    {{ label }}
    <span v-if="sourceCount && sourceCount > 1" class="episode-chip-badge">
      {{ sourceCount }}源
    </span>
  </button>
</template>
```

Add style (in `<style scoped>`):

```css
.episode-chip-badge {
  font-size: 0.6rem;
  background: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.05rem 0.3rem;
  border-radius: 0.2rem;
  margin-left: 0.25rem;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/media/EpisodeChip.vue
git commit -m "feat: add source-count badge to EpisodeChip"
```

---

### Task 3: EpisodeGroupPanel Deduplication

**Files:**
- Modify: `src/components/detail/EpisodeGroupPanel.vue`
- Test: `src/components/detail/__tests__/EpisodeGroupPanel.spec.ts`

- [ ] **Step 1: Update props and add unified episodes computed**

Replace the entire `<script setup>` with:

```ts
<script setup lang="ts">
import { computed, ref } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import { mergeEpisodes } from '@/utils/episode'
import type { CatalogEpisodeGroup, CatalogItemType, UnifiedEpisode } from '@/types'

const props = defineProps<{
  groups: CatalogEpisodeGroup[]
  item_type?: CatalogItemType
}>()

const emit = defineEmits<{
  play: [episode: UnifiedEpisode]
}>()

const isMovie = computed(() => props.item_type === 'movie')

const typeLabel = computed(() => {
  switch (props.item_type) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
})

const unifiedEpisodes = computed(() => {
  const itemType = props.item_type ?? 'series'
  return mergeEpisodes(props.groups, itemType)
})

const EXPAND_THRESHOLD = 24
const needsExpand = computed(() => {
  return !isMovie.value && unifiedEpisodes.value.length > EXPAND_THRESHOLD
})
const expanded = ref(false)

const visibleEpisodes = computed(() => {
  if (isMovie.value || !needsExpand.value || expanded.value) {
    return unifiedEpisodes.value
  }
  return unifiedEpisodes.value.slice(0, EXPAND_THRESHOLD)
})

const remainingCount = computed(() => {
  return unifiedEpisodes.value.length - EXPAND_THRESHOLD
})
</script>
```

- [ ] **Step 2: Update template**

Replace the template section with:

```vue
<template>
  <section class="source-group-card">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <span class="source-group-name">全部播放源</span>
        <span class="source-group-count-badge">
          {{ isMovie ? `${unifiedEpisodes.length} 个播放源` : `${unifiedEpisodes.length} 集` }}
        </span>
      </div>
      <span class="source-group-type-tag">{{ typeLabel }}</span>
    </div>

    <div class="source-group-body">
      <div v-if="isMovie" class="play-button-row">
        <button
          v-for="ue in unifiedEpisodes"
          :key="ue.sources[0].episode.id"
          class="play-button"
          @click="emit('play', ue)"
        >
          <span class="play-icon">▶</span>
          <span class="play-label">{{ ue.displayLabel }}</span>
        </button>
      </div>

      <template v-else>
        <div class="episode-chip-grid">
          <EpisodeChip
            v-for="ue in visibleEpisodes"
            :key="ue.normalizedIndex"
            :label="ue.displayLabel"
            :source-count="ue.sources.length"
            state="playable"
            @click="emit('play', ue)"
          />
        </div>

        <button
          v-if="needsExpand && !expanded"
          class="expand-toggle-button"
          @click="expanded = true"
        >
          <span>展开剩余 {{ remainingCount }} 集</span>
          <svg class="expand-chevron" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>

        <button
          v-else-if="needsExpand && expanded"
          class="expand-toggle-button"
          @click="expanded = false"
        >
          <span>收起</span>
          <svg class="expand-chevron expanded" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 10L8 6L12 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </template>
    </div>
  </section>
</template>
```

Keep existing `<style scoped>` unchanged.

- [ ] **Step 3: Update `VodDetail.vue` template to pass `groups`**

In `VodDetail.vue`, find the `EpisodeGroupPanel` usage in the normal catalog section and change from:

```vue
<EpisodeGroupPanel
  v-for="group in detailStore.episodeGroups"
  :key="group.source_name"
  :group="group"
  :item_type="detailStore.item?.item_type"
  @play="handlePlay"
/>
```

To:

```vue
<EpisodeGroupPanel
  :groups="detailStore.episodeGroups"
  :item_type="detailStore.item?.item_type"
  @play="handlePlay"
/>
```

- [ ] **Step 4: Update `EpisodeGroupPanel.spec.ts`**

Replace the entire test file with:

```ts
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGroupPanel from '../EpisodeGroupPanel.vue'
import type { CatalogEpisodeGroup } from '@/types'

const mockGroups: CatalogEpisodeGroup[] = [
  {
    source_name: 'Source A',
    episodes: [
      { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
      { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
    ],
  },
  {
    source_name: 'Source B',
    episodes: [
      { id: 3, episode_label: '第01集', play_url: 'http://b/1', order_index: 1 },
      { id: 4, episode_label: '第02集', play_url: 'http://b/2', order_index: 2 },
    ],
  },
]

describe('EpisodeGroupPanel', () => {
  it('merges duplicate episodes across sources for series', () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    const chips = wrapper.findAll('.episode-chip')
    expect(chips.length).toBe(2)
  })

  it('shows source count badge when episode has multiple sources', () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    const badges = wrapper.findAll('.episode-chip-badge')
    expect(badges.length).toBe(2)
    expect(badges[0].text()).toBe('2源')
  })

  it('emits unified episode on chip click', async () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    await wrapper.find('.episode-chip').trigger('click')
    expect(wrapper.emitted('play')).toHaveLength(1)
    const emitted = wrapper.emitted('play')![0][0] as { normalizedIndex: number; sources: unknown[] }
    expect(emitted.normalizedIndex).toBe(1)
    expect(emitted.sources).toHaveLength(2)
  })

  it('does not merge episodes for movies', () => {
    const movieGroups: CatalogEpisodeGroup[] = [
      {
        source_name: 'Source A',
        episodes: [
          { id: 1, episode_label: 'HD', play_url: 'http://a/hd', order_index: 1 },
          { id: 2, episode_label: '1080P', play_url: 'http://a/1080', order_index: 2 },
        ],
      },
    ]
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: movieGroups, item_type: 'movie' }
    })
    const buttons = wrapper.findAll('.play-button')
    expect(buttons.length).toBe(2)
  })
})
```

- [ ] **Step 5: Run component tests**

```bash
vitest run src/components/detail/__tests__/EpisodeGroupPanel.spec.ts
```

Expected: All tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/components/detail/EpisodeGroupPanel.vue src/components/detail/__tests__/EpisodeGroupPanel.spec.ts src/views/VodDetail.vue
git commit -m "feat: deduplicate episodes in EpisodeGroupPanel"
```

---

### Task 4: PlaybackDrawer Deduplication

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue`

- [ ] **Step 1: Update props, emits, and template**

Replace `<script setup>`:

```ts
<script setup lang="ts">
import { ref } from 'vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { PlayerSource, UnifiedEpisode } from '@/types'

const props = defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
  unifiedEpisodes?: UnifiedEpisode[]
  currentNormalizedIndex?: number
  activeTab?: 'sources' | 'episodes'
}>()

defineEmits<{
  select: [index: number]
  selectUnifiedEpisode: [episode: UnifiedEpisode]
  tabChange: [tab: 'sources' | 'episodes']
}>()

function sourceTone(kind: PlayerSource['kind']) {
  if (kind === 'external' || kind === 'embed') return 'danger'
  if (kind === 'hls') return 'cool'
  return 'neutral'
}

const innerTab = ref<'sources' | 'episodes'>(props.activeTab ?? 'sources')
</script>
```

Update the episodes tab section in template (replace the `v-else-if="innerTab === 'episodes' && episodes?.length"` block):

```vue
<!-- Episodes grid — shown when on episodes tab with unified episodes -->
<div v-else-if="innerTab === 'episodes' && unifiedEpisodes?.length" class="episode-grid">
  <button
    v-for="ue in unifiedEpisodes"
    :key="ue.normalizedIndex"
    :class="[
      'episode-chip',
      ue.normalizedIndex === currentNormalizedIndex ? 'episode-chip-active' : ''
    ]"
    type="button"
    @click="$emit('selectUnifiedEpisode', ue)"
  >
    {{ ue.displayLabel }}
    <span v-if="ue.sources.length > 1" class="source-count-badge">{{ ue.sources.length }}源</span>
  </button>
</div>
```

Add the badge style to `<style scoped>`:

```css
.source-count-badge {
  font-size: 0.6rem;
  background: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.05rem 0.3rem;
  border-radius: 0.2rem;
  margin-left: 0.25rem;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue
git commit -m "feat: deduplicate episodes in PlaybackDrawer"
```

---

### Task 5: VodDetail Integration

**Files:**
- Modify: `src/views/VodDetail.vue`, `src/stores/player.ts`

- [ ] **Step 1: Add `pendingUnifiedEpisode` to `playerStore.ts`**

Add imports and state:

```ts
import { ref } from 'vue'
import type { UnifiedEpisode } from '@/types'

// Inside the store definition, before return:
const pendingUnifiedEpisode = ref<UnifiedEpisode | null>(null)

function setPendingUnifiedEpisode(ep: UnifiedEpisode | null) {
  pendingUnifiedEpisode.value = ep
}

// In return object:
return {
  pendingUnifiedEpisode,
  setPendingUnifiedEpisode,
  saveHistory,
  getHistory,
}
```

- [ ] **Step 2: Update `handlePlay` in `VodDetail.vue`**

Replace:

```ts
function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`)
}
```

With:

```ts
import { usePlayerStore } from '@/stores/player'
import type { UnifiedEpisode } from '@/types'

// Add inside setup, after detailStore/libraryStore:
const playerStore = usePlayerStore()

function handlePlay(unifiedEpisode: UnifiedEpisode) {
  playerStore.setPendingUnifiedEpisode(unifiedEpisode)
  const firstSource = unifiedEpisode.sources[0]
  if (!firstSource) return
  router.push(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(firstSource.episode.play_url)}&episodeId=${firstSource.episode.id}`
  )
}
```

- [ ] **Step 3: Commit**

```bash
git add src/stores/player.ts src/views/VodDetail.vue
git commit -m "feat: wire UnifiedEpisode through VodDetail to playerStore"
```

---

### Task 6: PlayerPage Auto-Source Switching

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Add unified episode tracking state**

Add to the existing refs in `<script setup>`:

```ts
const currentUnifiedEpisode = ref<UnifiedEpisode | null>(null)
const currentUnifiedSourceIndex = ref(0)
```

Add imports:

```ts
import { usePlayerStore } from '@/stores/player'
import { mergeEpisodes } from '@/utils/episode'
import type { UnifiedEpisode } from '@/types'
```

Add store usage:

```ts
const playerStore = usePlayerStore()
```

- [ ] **Step 2: Compute `unifiedEpisodes` for the drawer**

After `activeGroup` ref, add:

```ts
const allGroups = computed(() => {
  if (detailStore.episodeGroups.length > 0) {
    return detailStore.episodeGroups
  }
  if (activeGroup.value) {
    return [activeGroup.value]
  }
  return []
})

const unifiedEpisodes = computed(() => {
  const groups = allGroups.value
  if (groups.length === 0) return []
  const itemType = detailStore.item?.item_type ?? 'series'
  return mergeEpisodes(groups, itemType)
})

const currentNormalizedIndex = computed(() => {
  if (!episodeId.value) return undefined
  const ue = unifiedEpisodes.value.find(u =>
    u.sources.some(s => s.episode.id === episodeId.value)
  )
  return ue?.normalizedIndex
})
```

- [ ] **Step 3: Implement `playUnifiedEpisode`**

Add after `initVodPlayback`:

```ts
async function playUnifiedEpisode(unifiedEpisode: UnifiedEpisode, sourceIndex = 0) {
  currentUnifiedEpisode.value = unifiedEpisode
  currentUnifiedSourceIndex.value = sourceIndex

  if (sourceIndex >= unifiedEpisode.sources.length) {
    errorMsg.value = '该集所有线路均不可用'
    return
  }

  const source = unifiedEpisode.sources[sourceIndex]

  if (itemId.value > 0) {
    await initVodPlayback(source.episode.play_url, source.episode.id)
  } else if (sourceName.value) {
    try {
      const targets = await invoke<PlaybackTarget[]>('provider_play', {
        source: sourceName.value,
        flag: 'auto',
        playUrl: source.episode.play_url,
      })
      if (targets.length > 0) {
        await initVodPlayback(targets[0].target_url, source.episode.id)
      } else {
        await playUnifiedEpisode(unifiedEpisode, sourceIndex + 1)
      }
    } catch (e) {
      console.error('[PlayerPage] provider_play failed:', e)
      await playUnifiedEpisode(unifiedEpisode, sourceIndex + 1)
    }
  }
}
```

- [ ] **Step 4: Update `switchToEpisode` to accept `UnifiedEpisode`**

Replace the existing `switchToEpisode` with:

```ts
async function switchToEpisode(unifiedEpisode: UnifiedEpisode) {
  const firstSource = unifiedEpisode.sources[0]
  if (!firstSource) return

  router.replace(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(firstSource.episode.play_url)}&episodeId=${firstSource.episode.id}`
  )

  await playUnifiedEpisode(unifiedEpisode)
}
```

- [ ] **Step 5: Update error handlers for cross-source switching**

In `initHlsPlayer`, update the HLS error handler:

```ts
hls.on(Hls.Events.ERROR, (_event, data) => {
  if (!data.fatal) return
  markCurrentSourceFailed()

  if (currentSourceIndex.value < sources.value.length - 1) {
    void switchToSource(currentSourceIndex.value + 1)
  } else if (
    currentUnifiedEpisode.value &&
    currentUnifiedSourceIndex.value < currentUnifiedEpisode.value.sources.length - 1
  ) {
    void playUnifiedEpisode(currentUnifiedEpisode.value, currentUnifiedSourceIndex.value + 1)
  } else {
    errorMsg.value = data.error?.message || '所有线路均不可用'
  }
})
```

Update `handleVideoError`:

```ts
function handleVideoError() {
  pendingAutoplay.value = false
  const mediaError = videoRef.value?.error
  const message = describeMediaErrorCode(mediaError?.code)
  markCurrentSourceFailed()

  if (currentSourceIndex.value < sources.value.length - 1) {
    errorMsg.value = `${message}，正在切换下一条线路`
    void switchToSource(currentSourceIndex.value + 1)
  } else if (
    currentUnifiedEpisode.value &&
    currentUnifiedSourceIndex.value < currentUnifiedEpisode.value.sources.length - 1
  ) {
    errorMsg.value = `${message}，正在切换下一个源`
    void playUnifiedEpisode(currentUnifiedEpisode.value, currentUnifiedSourceIndex.value + 1)
  } else {
    errorMsg.value = message
  }
}
```

- [ ] **Step 6: Handle pending unified episode on mount**

In the existing `onMounted` callback for vod mode, update the vod initialization block. Find:

```ts
} else if (mode.value === 'vod' && episodeUrl.value) {
  await initVodPlayback(episodeUrl.value, episodeId.value)
  // ...
}
```

Replace with:

```ts
} else if (mode.value === 'vod') {
  const pending = playerStore.pendingUnifiedEpisode
  if (pending) {
    playerStore.setPendingUnifiedEpisode(null)
    await playUnifiedEpisode(pending)
  } else if (episodeUrl.value) {
    await initVodPlayback(episodeUrl.value, episodeId.value)
  }
  // ... rest of detailStore.fetchDetail / loadProviderEpisodes ...
}
```

- [ ] **Step 7: Update PlaybackDrawer binding in template**

In the `<PlaybackDrawer>` usage, replace:

```vue
:episodes="activeGroup?.episodes"
:current-episode-id="currentEpisodeIdForDrawer"
@select-episode="switchToEpisode"
```

With:

```vue
:unified-episodes="unifiedEpisodes"
:current-normalized-index="currentNormalizedIndex"
@select-unified-episode="switchToEpisode"
```

- [ ] **Step 8: Remove obsolete `currentEpisodeIdForDrawer` computed**

The `currentEpisodeIdForDrawer` computed property is no longer needed. Remove it:

```ts
// REMOVE this entire computed:
const currentEpisodeIdForDrawer = computed(() => {
  if (episodeId.value) return episodeId.value
  if (episodeLabelFromQuery.value && activeGroup.value) {
    const ep = activeGroup.value.episodes.find(
      e => e.episode_label === episodeLabelFromQuery.value
    )
    if (ep) return ep.id
  }
  return undefined
})
```

- [ ] **Step 9: Run type check**

```bash
npm run build
```

Expected: Build succeeds with no TypeScript errors.

- [ ] **Step 10: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: auto-source-switching across UnifiedEpisode sources"
```

---

## Self-Review

### Spec Coverage Check

| Spec Requirement | Implementing Task |
|-----------------|-------------------|
| `UnifiedEpisode` type | Task 1 |
| `extractEpisodeIndex` normalization | Task 1 |
| `mergeEpisodes` for series/movies | Task 1 |
| `formatDisplayLabel` with 集/期 | Task 1 |
| EpisodeGroupPanel shows merged chips | Task 3 |
| Source count badge on chips | Task 2 + Task 3 |
| PlaybackDrawer shows merged episodes | Task 4 |
| VodDetail stores pending unified episode | Task 5 |
| PlayerPage auto-source-switching | Task 6 |
| Cross-source fallback on HLS error | Task 6 |
| Cross-source fallback on video error | Task 6 |
| Movies remain unmerged | Task 1 + Task 3 |
| SearchResultCard out of scope | Not in plan (correct) |

### Placeholder Scan

- No TBD, TODO, or "implement later" found.
- All steps include concrete code.
- No vague instructions like "add appropriate error handling".

### Type Consistency

- `UnifiedEpisode` defined in Task 1, used consistently in Tasks 3–6.
- `mergeEpisodes` signature uses `CatalogEpisodeGroup[]` and `CatalogItemType` as in spec.
- `EpisodeGroupPanel` emits `play: [UnifiedEpisode]` in Task 3.
- `PlaybackDrawer` emits `selectUnifiedEpisode: [UnifiedEpisode]` in Task 4.

---

## Execution Handoff

**Plan complete and saved to `docs/superpowers/plans/2026-04-29-episode-deduplication.md`. Two execution options:**

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

**Which approach?**
