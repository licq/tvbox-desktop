<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import { useLibraryStore } from '@/stores/library'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import DoubanMetaPanel from '@/components/detail/DoubanMetaPanel.vue'
import DetailMetaSkeleton from '@/components/detail/DetailMetaSkeleton.vue'
import EpisodeGroupPanel from '@/components/detail/EpisodeGroupPanel.vue'
import EpisodeGroupSkeleton from '@/components/detail/EpisodeGroupSkeleton.vue'
import type { CatalogEpisode, DoubanHot, SearchResult } from '@/types'

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

const route = useRoute()
const router = useRouter()
const detailStore = useDetailStore()
const libraryStore = useLibraryStore()

const itemId = computed(() => Number(route.params.itemId))
const isFromDouban = computed(() => route.query.douban === '1')

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

// Display meta: use enriched meta when available, otherwise preliminary from hot list
const displayMeta = computed(() => doubanMeta.value ?? preliminaryMeta.value)
const loadingDouban = ref(false)
const searchResults = ref<GroupedSearchResults[]>([])
const loadingSearch = ref(false)
const searchError = ref<string | null>(null)

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
  try {
    const results = await invoke<SearchResult[]>('search_vod_sources', { title })
    // Group by source
    const grouped: Record<string, SearchResult[]> = {}
    for (const r of results) {
      if (!grouped[r.source_name]) {
        grouped[r.source_name] = []
      }
      grouped[r.source_name].push(r)
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
watch(itemId, loadDetail)

function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`)
}

function handleSearchResultPlay(result: SearchResult) {
  // Navigate to player with source detail URL for playback
  router.push(`/player/source/${encodeURIComponent(result.detail_url)}?source=${result.source}&title=${encodeURIComponent(result.title || '')}`)
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

      <!-- Douban hot direct entry -->
      <div v-else-if="isFromDouban" class="mt-6 space-y-6">
        <!-- Top zone: Douban metadata (preliminary immediately, enriched when scraper returns) -->
        <DoubanMetaPanel
          v-if="displayMeta"
          :meta="displayMeta"
          :loading="loadingDouban"
          class="top-zone"
        />
        <DetailMetaSkeleton v-else-if="loadingDouban && !preliminaryMeta" class="top-zone" />

        <!-- Search results from sources -->
        <section v-if="loadingSearch" class="space-y-4">
          <EpisodeGroupSkeleton :count="4" />
        </section>

        <section v-else-if="searchResults.length" class="source-list space-y-4">
          <div
            v-for="group in searchResults"
            :key="group.source_name"
            class="rounded-xl bg-white/5 p-4"
          >
            <div class="mb-3 flex items-center justify-between">
              <h3 class="text-lg font-semibold text-white">{{ group.source_name }}</h3>
              <span class="text-sm text-white/40">{{ group.results.length }} 个结果</span>
            </div>
            <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
              <div
                v-for="result in group.results"
                :key="result.detail_url"
                class="flex items-center gap-3 rounded-lg bg-white/5 p-3 cursor-pointer hover:bg-white/10 transition-colors"
                @click="handleSearchResultPlay(result)"
              >
                <img v-if="result.poster" :src="result.poster" class="w-12 h-16 object-cover rounded" />
                <div class="flex-1 min-w-0">
                  <p class="text-white text-sm font-medium truncate">{{ result.title || doubanMeta?.title }}</p>
                  <p class="text-white/40 text-xs">{{ result.source_name }}</p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <div v-else-if="searchError" class="home-empty-state text-red-500">
          {{ searchError }}
        </div>
        <div v-else-if="doubanMeta && !loadingSearch" class="home-empty-state">
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
