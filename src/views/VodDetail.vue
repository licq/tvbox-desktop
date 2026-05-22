<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import { useLibraryStore } from '@/stores/library'
import { usePlayerStore } from '@/stores/player'
import { invoke } from '@tauri-apps/api/core'
import DoubanMetaPanel from '@/components/detail/DoubanMetaPanel.vue'
import DetailMetaSkeleton from '@/components/detail/DetailMetaSkeleton.vue'
import EpisodeGroupPanel from '@/components/detail/EpisodeGroupPanel.vue'
import EpisodeGroupSkeleton from '@/components/detail/EpisodeGroupSkeleton.vue'
import SearchResultCard from '@/components/detail/SearchResultCard.vue'
import SearchResultCardSkeleton from '@/components/detail/SearchResultCardSkeleton.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { CatalogEpisode, CatalogItemType, DoubanHot, PlaybackTarget, SearchResult, SourceSearchResult, UnifiedEpisode } from '@/types'
import {
  getVodDetailSearchSnapshot,
  normalizeVodDetailSearchKey,
  setVodDetailSearchSnapshot,
  type VodDetailProviderDetail,
  type VodDetailSearchGroup,
} from '@/utils/vodDetailSearchCache'

interface DoubanSubjectMeta {
  doubanId: number
  title: string
  rating: number | null
  ratingCount: number | null
  director: string[]
  writer: string[]
  actors: string[]
  genre: string[]
  country: string[]
  language: string[]
  releaseDate: string[]
  runtime: string | null
  summary: string | null
  poster: string | null
}

interface DedupSearchItem {
  title: string
  poster?: string
  item_type: SearchResult['item_type']
  sources: Array<{ source: string; source_name: string; detail_url: string }>
}

const route = useRoute()
const router = useRouter()
const detailStore = useDetailStore()
const libraryStore = useLibraryStore()
const playerStore = usePlayerStore()

const itemId = computed(() => Number(route.params.itemId))
const isFromDouban = computed(() => route.query.douban === '1')
const isSearch = computed(() => route.query.search === '1')

// Clean common Chinese suffixes from search result titles (e.g. "生命树[全集]" → "生命树")
// Also strips trailing English/original title (e.g. "匹兹堡医护前线 The Pitt" → "匹兹堡医护前线")
function cleanTitle(title: string): string {
  return title
    .replace(/\[全集\]/g, '')
    .replace(/\[全\]/g, '')
    .replace(/\[完结\]/g, '')
    .replace(/\[更新至\d+集\]/g, '')
    .replace(/\[更新\d+集\]/g, '')
    .replace(/第\d+季/g, '')
    .replace(/第\d+部/g, '')
    .replace(/\[HD\]/gi, '')
    .replace(/\[高清\]/g, '')
    // Strip trailing English/original title (e.g. "The Pitt", "Avatar")
    .replace(/\s+[A-Za-z][A-Za-z\s\d\-\:\.]*$/, '')
    .trim()
}

/**
 * Check if a search result title is relevant to the search keyword.
 * We require at least one character from the keyword to appear in the title.
 * This filters out completely unrelated results from loose provider matches
 * (e.g. ypanso returning "梅尔特伊" for "危险关系").
 */
function isTitleRelevant(title: string, keyword: string): boolean {
  const normalizedTitle = title.toLowerCase()
  const normalizedKeyword = keyword.toLowerCase()

  // Extract Chinese characters and alphanumeric from keyword
  const keywordChars = Array.from(new Set(
    normalizedKeyword.match(/[\u4e00-\u9fa5a-z0-9]/g) || []
  ))

  if (keywordChars.length === 0) return true // keyword has no extractable chars, don't filter

  // Require at least one keyword character to appear in title
  return keywordChars.some(ch => normalizedTitle.includes(ch))
}

// Quick lookup from library store (already has DoubanHot data from home page)
const hotItem = ref<DoubanHot | null>(null)

async function loadHotItemFromDb() {
  if (!isFromDouban.value || !itemId.value) return
  // First try store (home page data)
  const found = libraryStore.doubanHot.find(h => h.id === itemId.value)
  if (found) {
    hotItem.value = found
    return
  }
  // Fallback 1: fetch from database by id (direct lookup, no LIMIT)
  try {
    const dbFound = await invoke<DoubanHot | null>('get_douban_hot_by_id', { id: itemId.value })
    if (dbFound) {
      hotItem.value = dbFound
      return
    }
  } catch {
    // ignore
  }
  // Fallback 2: legacy bulk fetch (LIMIT 100 may miss some items)
  try {
    const items = await invoke<DoubanHot[]>('get_douban_hot', {})
    const dbFound = items.find(h => h.id === itemId.value)
    if (dbFound) hotItem.value = dbFound
  } catch {
    // ignore
  }
}

// Preliminary meta from DoubanHot (available immediately, before scraper returns)
const preliminaryMeta = computed<DoubanSubjectMeta | null>(() => {
  const h = hotItem.value
  if (!h) return null
  return {
    doubanId: h.id,
    title: h.name,
    rating: h.rating,
    ratingCount: null,
    director: [],
    writer: [],
    actors: [],
    genre: [],
    country: [],
    language: [],
    releaseDate: [],
    runtime: null,
    summary: null,
    poster: h.poster,
  }
})

// Enriched meta from WebView scraper (arrives later)
const doubanMeta = ref<DoubanSubjectMeta | null>(null)

// Meta from source detail page (after clicking a source)
const sourceDetailMeta = ref<DoubanSubjectMeta | null>(null)

// Display meta: doubanMeta (richest) when available, else source detail from play sources, else hot list, else fallback
const displayMeta = computed(() => doubanMeta.value ?? sourceDetailMeta.value ?? preliminaryMeta.value ?? fallbackMeta.value)


const fallbackMeta = computed<DoubanSubjectMeta | null>(() => {
  const first = dedupSearchItems.value[0]
  if (!first) return null
  return {
    doubanId: 0,
    title: cleanTitle(first.title),
    rating: null,
    ratingCount: null,
    director: [],
    writer: [],
    actors: [],
    genre: [],
    country: [],
    language: [],
    releaseDate: [],
    runtime: null,
    summary: null,
    poster: first.poster ?? null,
  }
})

const loadingDouban = ref(false)
// Request memoization state to avoid duplicate fetches on route changes
const doubanFetchState = ref<'idle' | 'loading' | 'done'>('idle')
const searchResults = ref<VodDetailSearchGroup[]>([])
const loadingSearch = ref(false)
const searchError = ref<string | null>(null)
const searchRequestVersion = ref(0)

// Prefer more specific types over generic/movie when sources disagree on the same title.
function itemTypePriority(t: SearchResult['item_type']): number {
  switch (t) {
    case 'series': return 4
    case 'variety': return 3
    case 'anime': return 2
    case 'movie': return 1
    case 'generic': return 0
    default: return 0
  }
}

/**
 * Infer item type from episode labels.
 * Distinguishes series (sequential episodes) from movies (multi-lineage copies).
 */
function inferTypeFromEpisodes(episodes: CatalogEpisode[]): 'series' | 'movie' | 'unknown' {
  if (episodes.length === 0) return 'unknown'
  if (episodes.length === 1) return 'movie'

  const labels = episodes.map(e => e.episode_label)

  // Strong series signal: any label contains "第X集" or S01E01 format
  if (labels.some(l => /第\s*\d+\s*集/.test(l))) return 'series'
  if (labels.some(l => /S\d+E\d+/i.test(l))) return 'series'

  // Strong series signal: pure numeric labels (01, 02, 03...) when count > 1
  // Filter out "线路1" style lineage labels first
  const pureNumericLabels = labels.filter(l => /^\d+$/.test(l))
  if (pureNumericLabels.length > 1) return 'series'

  // Movie multi-lineage patterns: all labels are quality/lineage identifiers
  const moviePatterns = [
    /^HD$/i, /^高清$/, /^1080P$/i, /^720P$/i, /^480P$/i, /^SD$/i,
    /^蓝光$/, /^BD$/i, /^DVD$/i, /^线路\d+$/, /^备用$/, /^正片$/, /^全集$/,
    /^立即播放$/, /^播放$/, /^m3u8$/i,
    /^国语$/, /^粤语$/, /^英语$/, /^中字$/, /^中英双字$/,
    /^HD中字$/, /^高清中字$/, /^BD中字$/, /^HD国语$/, /^高清国语$/,
  ]
  const allMatchMovie = labels.every(l =>
    moviePatterns.some(p => p.test(l))
  )
  if (allMatchMovie) return 'movie'

  // Heuristic: very high count strongly suggests series (movies rarely have 10+ lineages)
  if (episodes.length > 10) return 'series'

  return 'unknown'
}

const dedupSearchItems = computed<DedupSearchItem[]>(() => {
  const map = new Map<string, DedupSearchItem>()
  for (const group of searchResults.value) {
    for (const r of group.results) {
      const key = r.title ?? ''
      let item = map.get(key)
      if (!item) {
        item = { title: r.title ?? '', poster: r.poster, item_type: r.item_type, sources: [] }
        map.set(key, item)
      }
      // If this source claims a more specific type, upgrade the deduped item.
      if (itemTypePriority(r.item_type) > itemTypePriority(item.item_type)) {
        item.item_type = r.item_type
      }
      if (!item.sources.some(s => s.source === r.source)) {
        item.sources.push({ source: r.source, source_name: group.source_name, detail_url: r.detail_url })
      }
    }
  }
  // Infer type from preloaded provider details using episode label patterns.
  for (const item of map.values()) {
    for (const src of item.sources) {
      const cacheKey = getCacheKey(item.title, src.source)
      const detail = providerDetailCache.value.get(cacheKey)
      if (!detail) continue
      const inferred = inferTypeFromEpisodes(detail.episodes)
      if (inferred === 'series' && (item.item_type === 'movie' || item.item_type === 'generic')) {
        item.item_type = 'series'
      } else if (inferred === 'movie' && item.item_type === 'generic') {
        item.item_type = 'movie'
      }
      // Don't downgrade (e.g. keep 'series' if already set)
    }
  }
  return Array.from(map.values())
})

// Filter out items that have no playable episodes across all loaded sources.
// Only hide when all sources have finished loading (none pending) and all have 0 episodes.
const visibleDedupSearchItems = computed<DedupSearchItem[]>(() => {
  return dedupSearchItems.value.filter(item => {
    let allLoadedEmpty = true
    let hasAnyLoaded = false
    let hasAnyPending = false
    for (const src of item.sources) {
      const cacheKey = getCacheKey(item.title, src.source)
      const isLoading = preloadingKeys.value.has(cacheKey)
      const detail = providerDetailCache.value.get(cacheKey)
      if (isLoading) {
        hasAnyPending = true
        allLoadedEmpty = false
        continue
      }
      if (detail) {
        hasAnyLoaded = true
        if (detail.episodes.length > 0) {
          allLoadedEmpty = false
        }
      }
    }
    // Keep if: at least one source has episodes, or some sources are still loading
    if (hasAnyPending) return true
    // If no sources were ever loaded, keep it (preload hasn't reached it yet)
    if (!hasAnyLoaded) return true
    // All loaded sources are empty → hide
    return !allLoadedEmpty
  })
})

const providerDetailCache = ref(new Map<string, VodDetailProviderDetail>())
const preloadingKeys = ref(new Set<string>())

function getCacheKey(title: string, source: string): string {
  return `${title}-${source}`
}

function restoreSearchSnapshot(snapshot: {
  searchResults: VodDetailSearchGroup[]
  providerDetailEntries: Array<[string, VodDetailProviderDetail]>
}) {
  searchResults.value = snapshot.searchResults.map(group => ({
    source_name: group.source_name,
    results: group.results.map(result => ({ ...result })),
  }))
  providerDetailCache.value = new Map(
    snapshot.providerDetailEntries.map(([key, detail]) => [
      key,
      {
        title: detail.title,
        poster: detail.poster,
        summary: detail.summary,
        episodes: detail.episodes.map(episode => ({ ...episode })),
      },
    ]),
  )
}

async function preloadAllSources(item: DedupSearchItem) {
  await Promise.all(
    item.sources.map(src => preloadSource(item, src.source))
  )
}

async function preloadSource(item: DedupSearchItem, sourceKey: string) {
  const key = getCacheKey(item.title, sourceKey)
  if (providerDetailCache.value.has(key) || preloadingKeys.value.has(key)) return

  const source = item.sources.find(s => s.source === sourceKey)
  if (!source) return

  preloadingKeys.value.add(key)
  try {
    const detail = await invoke<VodDetailProviderDetail>('provider_detail', {
      source: source.source,
      ids: source.detail_url,
    })
    providerDetailCache.value.set(key, detail)
  } catch (e) {
    console.error('[VodDetail] preload failed for', item.title, sourceKey, e)
  } finally {
    preloadingKeys.value.delete(key)
  }
}

async function handleCardEpisodePlay(episode: CatalogEpisode, sourceKey: string, item: DedupSearchItem) {
  const source = item.sources.find(s => s.source === sourceKey)
  if (!source) return

  try {
    const targets = await invoke<PlaybackTarget[]>('provider_play', {
      source: source.source,
      flag: 'auto',
      playUrl: episode.play_url,
    })
    if (targets.length > 0) {
      const target = targets[0]
      router.push({
        name: 'player',
        params: { mode: 'vod', id: 0 },
        query: {
          episode: target.target_url,
          source: source.source,
          detailUrl: source.detail_url,
          title: item.title,
          episodeLabel: episode.episode_label,
          episodeReferer: target.referer ?? undefined,
          episodeHeaders: target.headers ? JSON.stringify(target.headers) : undefined,
          episodeTargets: JSON.stringify(targets),
        },
      })
    } else {
      searchError.value = '播放地址获取失败'
    }
  } catch (e) {
    console.error('[VodDetail] provider_play failed:', e)
    searchError.value = '播放地址获取失败'
  }
}

function getSourceDetailsForItem(item: DedupSearchItem): Record<string, VodDetailProviderDetail> {
  const result: Record<string, VodDetailProviderDetail> = {}
  for (const src of item.sources) {
    const key = getCacheKey(item.title, src.source)
    const detail = providerDetailCache.value.get(key)
    if (detail) {
      result[src.source] = detail
    }
  }
  return result
}

function getLoadingSourcesForItem(item: DedupSearchItem): string[] {
  return item.sources
    .filter(src => preloadingKeys.value.has(getCacheKey(item.title, src.source)))
    .map(src => src.source)
}

async function loadDetail() {
  // Direct from Douban hot list - use itemId as douban_id directly
  if (isFromDouban.value && itemId.value) {
    // OPTIMIZATION 1: Load basic info from store/DB immediately
    await loadHotItemFromDb()

    // OPTIMIZATION 2: Start source search IMMEDIATELY (we have title now)
    if (hotItem.value?.name) {
      await searchSources(cleanTitle(hotItem.value.name))
    }

    // Also fetch enriched Douban metadata in background for director/actors/summary
    // Skip if already fetched or in progress (memoization)
    if (doubanFetchState.value === 'done' && doubanMeta.value) return
    if (doubanFetchState.value === 'loading') return
    doubanFetchState.value = 'loading'
    loadingDouban.value = true
    invoke<DoubanSubjectMeta | null>('fetch_douban_metadata_by_id', {
      douban_id: itemId.value,
    }).then(meta => {
      doubanMeta.value = meta
    }).catch(e => {
      console.error('[VodDetail] fetch_douban_metadata_by_id failed:', e)
    }).finally(() => {
      loadingDouban.value = false
      doubanFetchState.value = 'done'
    })
    return
  }

  if (isSearch.value) {
    const keyword = route.query.keyword as string
    if (keyword) {
      await searchSources(keyword)
      // Try to get Douban metadata for the top panel (use cleaned keyword)
      // Skip if already fetched or in progress (memoization)
      if (doubanFetchState.value === 'done' && doubanMeta.value) return
      if (doubanFetchState.value === 'loading') return
      doubanFetchState.value = 'loading'
      const cleanKeyword = cleanTitle(keyword)
      loadingDouban.value = true
      invoke<DoubanSubjectMeta | null>('search_douban_subject_by_keyword', { keyword: cleanKeyword })
        .then(meta => {
          doubanMeta.value = meta
        })
        .catch(e => {
          console.error('[VodDetail] search_douban_subject_by_keyword failed:', e)
        })
        .finally(() => {
          loadingDouban.value = false
          doubanFetchState.value = 'done'
        })
    }
    return
  }

  // Normal catalog item flow
  if (!Number.isFinite(itemId.value) || itemId.value <= 0) {
    return
  }

  await detailStore.fetchDetail(itemId.value)

  // Fetch Douban metadata on-demand
  // Skip if already fetched or in progress (memoization)
  if (doubanFetchState.value === 'done' && doubanMeta.value) return
  if (doubanFetchState.value === 'loading') return
  doubanFetchState.value = 'loading'
  loadingDouban.value = true
  try {
    const meta = await invoke<DoubanSubjectMeta | null>('fetch_douban_subject_metadata', {
      itemId: itemId.value,
    })
    doubanMeta.value = meta
  } catch (e) {
    console.error('[VodDetail] fetch_douban_subject_metadata failed:', e)
    doubanMeta.value = null
  } finally {
    loadingDouban.value = false
    doubanFetchState.value = 'done'
  }
}

async function searchSources(title: string) {
  // Reset douban fetch state when title changes (navigating to different search)
  doubanFetchState.value = 'idle'
  const requestVersion = ++searchRequestVersion.value
  loadingSearch.value = true
  searchError.value = null

  const cachedSnapshot = getVodDetailSearchSnapshot(title)
  if (cachedSnapshot) {
    restoreSearchSnapshot(cachedSnapshot)
    loadingSearch.value = false
    return
  }

  providerDetailCache.value.clear()
  preloadingKeys.value.clear()
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
    if (requestVersion !== searchRequestVersion.value) return
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      for (const item of r.items) {
        if (!r.source_name) continue
        // Filter out results whose title shares no characters with the keyword
        if (!isTitleRelevant(item.title, title)) continue
        grouped[r.source_name] ||= []
        grouped[r.source_name].push({
          source: r.source_key,
          source_name: r.source_name,
          detail_url: item.source_item_key,
          item_type: item.item_type as SearchResult['item_type'],
          title: item.title,
          poster: item.poster,
        })
      }
    }
    searchResults.value = Object.entries(grouped)
      .filter(([, results]) => results.length > 0)
      .map(([source_name, results]) => ({
        source_name,
        results,
      }))

    // Preload source details so real cards have episodes data ready.
    // Keep skeleton visible until preloading completes to avoid layout shift.
    await Promise.all(dedupSearchItems.value.map(item => preloadAllSources(item)))
    if (requestVersion !== searchRequestVersion.value) return
    setVodDetailSearchSnapshot(normalizeVodDetailSearchKey(title), {
      searchResults: searchResults.value,
      providerDetailEntries: Array.from(providerDetailCache.value.entries()),
    })
    // Only hide skeleton after preloading completes
    loadingSearch.value = false
  } catch (e) {
    if (requestVersion !== searchRequestVersion.value) return
    console.error('[VodDetail] searchSources failed:', e)
    searchError.value = String(e)
    searchResults.value = []
  } finally {
    if (requestVersion === searchRequestVersion.value) {
      loadingSearch.value = false
    }
  }
}

onMounted(loadDetail)
const stopWatch = watch(itemId, loadDetail, { immediate: false })
onUnmounted(() => {
  stopWatch()
})

function handlePlay(ue: UnifiedEpisode) {
  if (ue.sources.length === 0) return
  playerStore.setPendingUnifiedEpisode(ue)
  if (detailStore.item) {
    playerStore.setPendingVodDetail({
      item: detailStore.item,
      episode_groups: detailStore.episodeGroups,
    })
  }
  const episode = ue.sources[0].episode
  console.error('[playback-dbg][voddetail] handlePlay', {
    itemId: itemId.value,
    episodeId: episode.id,
    episodeLabel: episode.episode_label,
    playUrl: episode.play_url,
    unifiedEpisode: ue.normalizedIndex,
    sourceCount: ue.sources.length,
  })
  router.push({
    path: `/player/vod/${itemId.value}`,
    query: {
      episode: episode.play_url,
      episodeId: String(episode.id),
      title: detailStore.item?.title ?? undefined,
    },
  })
}

</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1400px]">
      <button class="action-button action-button-secondary" type="button" @click="router.back()">
        返回片库
      </button>

      <!-- Loading state: no item -->
      <div v-if="detailStore.loading && !detailStore.item && !isFromDouban" class="surface-panel mt-6 flex min-h-[420px] items-center justify-center rounded-[2.4rem]">
        <LoadingSpinner size="lg" />
      </div>

      <!-- Douban hot direct entry or search results -->
      <div v-else-if="isFromDouban || isSearch" class="mt-6 space-y-6">
        <!-- Top zone: Douban metadata (for douban flow and search results with matched meta) -->
        <template v-if="displayMeta">
          <DoubanMetaPanel
            :meta="displayMeta"
            :loading="loadingDouban"
            class="top-zone"
          />
        </template>
        <DetailMetaSkeleton v-else-if="loadingDouban" class="top-zone" />

        <!-- Search results from sources -->
        <section v-if="loadingSearch" class="space-y-4">
          <SearchResultCardSkeleton v-for="i in 4" :key="i" />
        </section>

        <section v-else-if="visibleDedupSearchItems.length" class="source-list space-y-4">
          <SearchResultCard
            v-for="item in visibleDedupSearchItems"
            :key="item.title"
            :title="item.title"
            :poster="item.poster"
            :item-type="(item.item_type === 'generic' ? 'movie' : item.item_type) as CatalogItemType"
            :sources="item.sources"
            :source-details="getSourceDetailsForItem(item)"
            :loading-sources="getLoadingSourcesForItem(item)"
            @play-episode="(ep, sourceKey) => handleCardEpisodePlay(ep, sourceKey, item)"
            @select-source="(sourceKey) => preloadSource(item, sourceKey)"
          />
        </section>

        <div v-if="searchError" class="home-empty-state text-red-500">
          {{ searchError }}
        </div>
        <div v-else-if="!loadingSearch && visibleDedupSearchItems.length === 0" class="home-empty-state">
          暂未找到可用的播放源
        </div>
      </div>

      <!-- Normal catalog item -->
      <div v-else-if="detailStore.item" class="mt-6 space-y-6">
        <!-- Top zone: Douban metadata -->
        <DoubanMetaPanel
          v-if="doubanMeta"
          :meta="doubanMeta"
          :poster="detailStore.item.poster"
          class="top-zone"
        />
        <DetailMetaSkeleton v-else-if="loadingDouban" class="top-zone" />

        <!-- Bottom: all source lists -->
        <section v-if="detailStore.loading && detailStore.item" class="space-y-4">
          <EpisodeGroupSkeleton :count="12" />
        </section>

        <section v-else-if="detailStore.episodeGroups.length" class="source-list space-y-4">
          <EpisodeGroupPanel
            :groups="detailStore.episodeGroups"
            :item_type="detailStore.item?.item_type"
            @play="handlePlay"
          />
        </section>

        <div v-else-if="detailStore.item" class="home-empty-state">
          当前内容没有可展示的播放入口。
        </div>
      </div>

      <div v-else class="surface-panel mt-6 flex min-h-[320px] items-center justify-center rounded-[2rem] text-sm text-white/45">
        没有找到内容详情。
      </div>
    </div>
  </div>
</template>
