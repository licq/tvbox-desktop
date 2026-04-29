<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import { useLibraryStore } from '@/stores/library'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import DoubanMetaPanel from '@/components/detail/DoubanMetaPanel.vue'
import DetailMetaSkeleton from '@/components/detail/DetailMetaSkeleton.vue'
import EpisodeGroupPanel from '@/components/detail/EpisodeGroupPanel.vue'
import EpisodeGroupSkeleton from '@/components/detail/EpisodeGroupSkeleton.vue'
import SearchResultCard from '@/components/detail/SearchResultCard.vue'
import type { CatalogEpisode, CatalogItemType, DoubanHot, PlaybackTarget, SearchResult, SourceSearchResult } from '@/types'

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

interface GroupedSearchResults {
  source_name: string
  results: SearchResult[]
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

const itemId = computed(() => Number(route.params.itemId))
const isFromDouban = computed(() => route.query.douban === '1')
const isSearch = computed(() => route.query.search === '1')

// Clean common Chinese suffixes from search result titles (e.g. "生命树[全集]" → "生命树")
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
    .trim()
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
  // Fallback: fetch from database by type
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
const searchResults = ref<GroupedSearchResults[]>([])
const loadingSearch = ref(false)
const searchError = ref<string | null>(null)

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
  // Infer series from preloaded provider details: if any source has >1 episodes,
  // upgrade the item type from movie/generic to series.
  for (const item of map.values()) {
    if (item.item_type !== 'movie' && item.item_type !== 'generic') continue
    for (const src of item.sources) {
      const cacheKey = getCacheKey(item.title, src.source)
      const detail = providerDetailCache.value.get(cacheKey)
      if (detail && detail.episodes.length > 1) {
        item.item_type = 'series'
        break
      }
    }
  }
  return Array.from(map.values())
})

interface ProviderDetailResult {
  title: string | null
  poster: string | null
  summary: string | null
  episodes: CatalogEpisode[]
}
const providerDetailCache = ref(new Map<string, ProviderDetailResult>())
const preloadingKeys = ref(new Set<string>())

function getCacheKey(title: string, source: string): string {
  return `${title}-${source}`
}

async function preloadFirstSource(item: DedupSearchItem) {
  const first = item.sources[0]
  if (!first) return
  await preloadSource(item, first.source)
}

async function preloadSource(item: DedupSearchItem, sourceKey: string) {
  const key = getCacheKey(item.title, sourceKey)
  if (providerDetailCache.value.has(key) || preloadingKeys.value.has(key)) return

  const source = item.sources.find(s => s.source === sourceKey)
  if (!source) return

  preloadingKeys.value.add(key)
  try {
    const detail = await invoke<ProviderDetailResult>('provider_detail', {
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
      router.push(`/player/vod/0?episode=${encodeURIComponent(target.target_url)}&source=${source.source}&detailUrl=${encodeURIComponent(source.detail_url)}&episodeLabel=${encodeURIComponent(episode.episode_label)}`)
    } else {
      searchError.value = '播放地址获取失败'
    }
  } catch (e) {
    console.error('[VodDetail] provider_play failed:', e)
    searchError.value = '播放地址获取失败'
  }
}

function getSourceDetailsForItem(item: DedupSearchItem): Record<string, ProviderDetailResult> {
  const result: Record<string, ProviderDetailResult> = {}
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
      searchSources(hotItem.value.name)
    }

    // Also fetch enriched Douban metadata in background for director/actors/summary
    loadingDouban.value = true
    invoke<DoubanSubjectMeta | null>('fetch_douban_metadata_by_id', {
      douban_id: itemId.value,
    }).then(meta => {
      doubanMeta.value = meta
    }).catch(e => {
      console.error('[VodDetail] fetch_douban_metadata_by_id failed:', e)
    }).finally(() => {
      loadingDouban.value = false
    })
    return
  }

  // Direct search from home page - use keyword from query param
  if (isSearch.value) {
    const keyword = route.query.keyword as string
    if (keyword) {
      searchSources(keyword)
      // Try to get Douban metadata for the top panel (use cleaned keyword)
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
  }
}

async function searchSources(title: string) {
  loadingSearch.value = true
  searchError.value = null
  providerDetailCache.value.clear()
  preloadingKeys.value.clear()
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      for (const item of r.items) {
        if (!r.source_name) continue
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

    // Preload first source detail for each dedup result
    for (const item of dedupSearchItems.value) {
      preloadFirstSource(item)
    }
  } catch (e) {
    console.error('[VodDetail] searchSources failed:', e)
    searchError.value = String(e)
    searchResults.value = []
  } finally {
    loadingSearch.value = false
  }
}

onMounted(loadDetail)
const stopWatch = watch(itemId, loadDetail, { immediate: false })
onUnmounted(() => {
  stopWatch()
})

function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`)
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
          <EpisodeGroupSkeleton :count="4" />
        </section>

        <section v-else-if="dedupSearchItems.length" class="source-list space-y-4">
          <SearchResultCard
            v-for="item in dedupSearchItems"
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
        <div v-else-if="!loadingSearch && dedupSearchItems.length === 0" class="home-empty-state">
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
          <EpisodeGroupSkeleton :count="8" />
        </section>

        <section v-else-if="detailStore.episodeGroups.length" class="source-list space-y-4">
          <EpisodeGroupPanel
            v-for="group in detailStore.episodeGroups"
            :key="group.source_name"
            :group="group"
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

