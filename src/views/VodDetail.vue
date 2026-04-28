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
function typeLabel(itemType: string): string {
  switch (itemType) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
}

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

const dedupSearchItems = computed<DedupSearchItem[]>(() => {
  const map = new Map<string, DedupSearchItem>()
  for (const group of searchResults.value) {
    for (const r of group.results) {
      const key = `${r.title ?? ''}-${r.item_type}`
      let item = map.get(key)
      if (!item) {
        item = { title: r.title ?? '', poster: r.poster, item_type: r.item_type, sources: [] }
        map.set(key, item)
      }
      if (!item.sources.some(s => s.source === r.source)) {
        item.sources.push({ source: r.source, source_name: group.source_name, detail_url: r.detail_url })
      }
    }
  }
  return Array.from(map.values())
})

// Provider detail state for selected search result
interface ProviderEpisodeGroup {
  source_name: string
  source_key: string
  episodes: CatalogEpisode[]
}

interface ProviderDetailResult {
  title: string | null
  poster: string | null
  summary: string | null
  episodes: CatalogEpisode[]
}
const providerEpisodes = ref<ProviderEpisodeGroup[] | null>(null)
const providerItemType = ref<CatalogItemType>('movie')
const loadingProviderDetail = ref(false)
const providerDetailError = ref<string | null>(null)

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
  providerEpisodes.value = null
  providerDetailError.value = null
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword: title })
    // Group by source, transform to SearchResult format
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      for (const item of r.items) {
        if (!r.source_name) continue // skip items with no source name
        grouped[r.source_name] ||= []
        grouped[r.source_name].push({
          source: r.source_key,
          source_name: r.source_name,
          // detail_url carries source_item_key for provider_detail lookup
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

async function handleSearchResultPlay(result: SearchResult) {
  // detail_url is the source_item_key from the scraped item
  const source = result.source
  const ids = result.detail_url

  if (!source || !ids) {
    searchError.value = '播放信息不完整'
    return
  }

  // Fetch episodes from provider detail
  loadingProviderDetail.value = true
  providerDetailError.value = null
  providerEpisodes.value = null
  providerItemType.value = (result.item_type === 'generic' ? 'movie' : result.item_type) as CatalogItemType
  try {
    const detailResult = await invoke<ProviderDetailResult>('provider_detail', {
      source,
      ids,
    })
    if (detailResult.episodes.length === 0) {
      providerDetailError.value = '该视频没有可播放的剧集'
      return
    }
    // Update top panel with source detail metadata (poster, title, summary)
    if (detailResult.title || detailResult.poster) {
      sourceDetailMeta.value = {
        doubanId: 0,
        title: detailResult.title || cleanTitle(result.title || ''),
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
        summary: detailResult.summary,
        poster: detailResult.poster || result.poster || null,
      }
    }
    providerEpisodes.value = [{
      source_name: result.source_name,
      source_key: source,
      episodes: detailResult.episodes,
    }]
  } catch (e) {
    console.error('[VodDetail] provider_detail failed:', e)
    providerDetailError.value = String(e)
  } finally {
    loadingProviderDetail.value = false
  }
}

function handleSourceClick(item: DedupSearchItem, src: { source: string; source_name: string; detail_url: string }) {
  handleSearchResultPlay({
    source: src.source,
    source_name: src.source_name,
    detail_url: src.detail_url,
    item_type: item.item_type,
    title: item.title,
    poster: item.poster,
  })
}

async function handleProviderEpisodePlay(episode: CatalogEpisode) {
  if (!providerEpisodes.value?.length) return
  const source = providerEpisodes.value[0].source_key

  try {
    const targets = await invoke<PlaybackTarget[]>('provider_play', {
      source,
      flag: 'auto',
      playUrl: episode.play_url,
    })
    if (targets.length > 0) {
      const target = targets[0]
      // Navigate to vod player which uses playbackStore.resolve() to handle
      // various play page formats (xb6v, zxzj, etc.)
      router.push(`/player/vod/0?episode=${encodeURIComponent(target.target_url)}&source=${source}`)
    } else {
      providerDetailError.value = '播放地址获取失败'
    }
  } catch (e) {
    console.error('[VodDetail] provider_play failed:', e)
    providerDetailError.value = String(e)
  }
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
          <div
            v-for="item in dedupSearchItems"
            :key="item.title"
            class="source-group-card"
          >
            <div class="source-group-header">
              <div class="source-group-header-left">
                <img v-if="item.poster" :src="item.poster" class="dedup-poster" />
                <div class="dedup-search-info">
                  <span class="source-group-name">{{ item.title }}</span>
                  <span class="source-group-count-badge">{{ item.sources.length }} 个播放源</span>
                </div>
              </div>
              <span class="source-group-type-tag">{{ typeLabel(item.item_type) }}</span>
            </div>
            <div class="source-group-body">
              <div class="play-button-row">
                <button
                  v-for="src in item.sources"
                  :key="src.detail_url"
                  class="play-button"
                  @click="handleSourceClick(item, src)"
                >
                  <span class="play-icon">▶</span>
                  <span class="play-label">{{ src.source_name }}</span>
                </button>
              </div>
            </div>
          </div>
        </section>

        <!-- Provider detail episodes (shown after clicking a search result) -->
        <section v-if="loadingProviderDetail" class="space-y-4">
          <EpisodeGroupSkeleton :count="4" />
        </section>
        <section v-else-if="providerEpisodes" class="source-list space-y-4">
          <EpisodeGroupPanel
            v-for="group in providerEpisodes"
            :key="group.source_name"
            :group="group"
            :item_type="providerItemType"
            @play="handleProviderEpisodePlay"
          />
        </section>
        <div v-else-if="providerDetailError" class="home-empty-state text-red-500">
          {{ providerDetailError }}
        </div>

        <div v-else-if="searchError" class="home-empty-state text-red-500">
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

<style scoped>
.dedup-poster {
  width: 3rem;
  height: 4.5rem;
  object-fit: cover;
  border-radius: 0.4rem;
  flex-shrink: 0;
}
.dedup-search-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
  transition: transform 200ms ease, border-color 200ms ease;
}
.source-group-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 255, 255, 0.12);
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-count-badge {
  font-size: 0.65rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.35);
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 0.25rem;
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.75rem 1rem;
}
.play-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.play-button {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 0.5rem;
  padding: 0.4rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.play-icon {
  color: rgba(117, 169, 195, 0.7);
  font-size: 0.65rem;
}
.play-button:hover .play-icon {
  color: rgba(200, 230, 245, 0.95);
}
</style>
