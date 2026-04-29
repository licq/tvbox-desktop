<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useSubscriptionStore } from '@/stores/subscription'
import { useLibraryStore } from '@/stores/library'
import { invoke } from '@tauri-apps/api/core'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import MediaCard from '@/components/media/MediaCard.vue'
import type { CatalogItemType, DoubanHot, LiveChannel, SourceSearchResult, ProviderCatalogItem } from '@/types'

type HomeTabKey = 'live' | CatalogItemType | 'search'

const route = useRoute()
const router = useRouter()
const liveStore = useLiveStore()
const subStore = useSubscriptionStore()
const libraryStore = useLibraryStore()

const tabs = computed(() => {
  const fixedTabs: { key: HomeTabKey; label: string; eyebrow?: string }[] = [
    { key: 'movie', label: '电影', eyebrow: 'Movie' },
    { key: 'series', label: '剧集', eyebrow: 'Series' },
    { key: 'variety', label: '综艺', eyebrow: 'Shows' },
    { key: 'anime', label: '动漫', eyebrow: 'Anime' },
    { key: 'search', label: '搜索', eyebrow: 'Search' },
    { key: 'live', label: '直播', eyebrow: 'Live' },
  ]
  return fixedTabs
})

const activeTab = ref<HomeTabKey>('movie')
const searchKeyword = ref('')
const expandedChannels = ref<Set<string>>(new Set())
const providerSearchResults = ref<{ source_name: string; results: ProviderCatalogItem[] }[]>([])
const loadingProviderSearch = ref(false)
const searchFilter = ref<'all' | CatalogItemType>('all')
let searchVersion = 0

interface FlatSearchItem extends ProviderCatalogItem {
  source_name: string
}

const allSearchItems = computed(() => {
  return providerSearchResults.value.flatMap(group =>
    group.results.map(item => ({ ...item, source_name: group.source_name } as FlatSearchItem))
  )
})

const filteredSearchItems = computed(() => {
  if (searchFilter.value === 'all') return allSearchItems.value
  return allSearchItems.value.filter(
    item => item.item_type === searchFilter.value
  )
})

const countByType = computed(() => {
  const counts: Record<string, number> = { movie: 0, series: 0, variety: 0, anime: 0 }
  for (const item of allSearchItems.value) {
    if (item.item_type in counts) {
      counts[item.item_type]++
    }
  }
  return counts
})

const validTabs = computed(() => new Set(tabs.value.map(tab => tab.key)))

function normalizeTab(tab: string | string[] | undefined): HomeTabKey {
  if (typeof tab === 'string' && validTabs.value.has(tab as HomeTabKey)) {
    return tab as HomeTabKey
  }

  return 'movie'
}

function formatTypeLabel(type: CatalogItemType | HomeTabKey) {
  return tabs.value.find(tab => tab.key === type)?.label ?? '片库'
}

const filteredGroups = computed(() => {
  if (!searchKeyword.value || activeTab.value !== 'live') return liveStore.groups
  const keyword = searchKeyword.value.toLowerCase()

  return liveStore.groups
    .map(group => ({
      ...group,
      channels: group.channels.filter(channel => channel.name.toLowerCase().includes(keyword))
    }))
    .filter(group => group.channels.length > 0)
})

const displayedHotItems = computed(() => {
  if (activeTab.value === 'live') return []
  const type = activeTab.value as string
  return libraryStore.getDoubanHotByType(type)
})

async function hydrateSources() {
  if (activeTab.value === 'search') return

  // Minimal data fetch only (skip subscription refresh to avoid blocking)
  try {
    await libraryStore.fetchCatalog()
  } catch {
    console.error('[hydrateSources] fetchCatalog failed')
  }

  try {
    await liveStore.fetchGroups()
  } catch {
    console.error('[hydrateSources] fetchGroups failed')
  }

  if (activeTab.value !== 'live') {
    await libraryStore.fetchDoubanHotByType(activeTab.value)
  }
}

onMounted(hydrateSources)

watch(
  () => route.params.type,
  async (tabParam) => {
    const nextTab = normalizeTab(tabParam)

    if (typeof tabParam === 'string' && nextTab !== tabParam) {
      await router.replace(`/library/${nextTab}`)
      return
    }

    activeTab.value = nextTab

    if (nextTab === 'search') {
      const keywordFromQuery = typeof route.query.keyword === 'string' ? route.query.keyword : undefined
      if (keywordFromQuery) {
        searchKeyword.value = keywordFromQuery
        await searchAllProviders(keywordFromQuery)
      } else {
        searchKeyword.value = ''
        providerSearchResults.value = []
        searchFilter.value = 'all'
      }
      return
    }

    searchKeyword.value = ''
    providerSearchResults.value = []
    searchFilter.value = 'all'

    if (nextTab === 'live') {
      // existing live logic remains
    } else {
      // Fetch douban hot for this tab type
      await libraryStore.fetchDoubanHotByType(nextTab)
    }
  },
  { immediate: true }
)

function onTabChange(tab: string) {
  router.push(`/library/${tab}`)
}

async function handleVodSearch(keyword: string) {
  if (keyword) {
    searchKeyword.value = keyword
    providerSearchResults.value = []
    await searchAllProviders(keyword)
    await router.replace({ query: { ...route.query, keyword } })
    return
  }
  searchKeyword.value = ''
  providerSearchResults.value = []
  await router.replace({ query: { ...route.query, keyword: undefined } })
}

async function searchAllProviders(keyword: string) {
  const currentVersion = ++searchVersion
  loadingProviderSearch.value = true
  try {
    const results = await invoke<SourceSearchResult[]>('search_all_sources', { keyword })
    if (currentVersion !== searchVersion) return // stale, discard
    const grouped: Record<string, ProviderCatalogItem[]> = {}
    for (const r of results) {
      if (!r.source_name) continue
      if (r.items.length === 0) continue
      grouped[r.source_name] = r.items
    }
    providerSearchResults.value = Object.entries(grouped)
      .filter(([, items]) => items.length > 0)
      .map(([source_name, results]) => ({ source_name, results }))
  } catch (e) {
    console.error('[Home] searchAllProviders failed:', e)
    providerSearchResults.value = []
  } finally {
    if (currentVersion === searchVersion) {
      loadingProviderSearch.value = false
    }
  }
}

function handleProviderResultClick(item: ProviderCatalogItem) {
  // Navigate to detail page with search keyword so episodes can be displayed
  const keyword = item.title || searchKeyword.value
  router.push(`/detail/0?search=1&keyword=${encodeURIComponent(keyword)}`)
}

function handlePlayChannel(channel: LiveChannel, _sourceUrl?: string) {
  router.push(`/player/live/${channel.id}`)
}

function handleHotClick(hot: DoubanHot) {
  router.push(`/detail/${hot.id}?douban=1`)
}

function toggleChannelExpansion(category: string) {
  if (expandedChannels.value.has(category)) {
    expandedChannels.value.delete(category)
  } else {
    expandedChannels.value.add(category)
  }
}
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1500px]">
      <header class="home-topbar">
        <div>
          <div class="eyebrow">TVBox Desktop</div>
          <div class="home-topbar-title">媒体中枢</div>
        </div>

        <div class="home-topbar-actions">
          <RouterLink to="/subscriptions" class="action-button action-button-secondary">订阅</RouterLink>
          <RouterLink to="/settings" class="action-button action-button-secondary">设置</RouterLink>
        </div>
      </header>

      <nav class="home-category-nav" aria-label="媒体分类">
        <button
          v-for="tab in tabs"
          :key="tab.key"
          :class="['nav-pill', activeTab === tab.key ? 'nav-pill-active' : '']"
          type="button"
          @click="onTabChange(tab.key)"
        >
          <span>{{ tab.eyebrow }}</span>
          {{ tab.label }}
        </button>
      </nav>

      <main class="home-landing">
        <div v-if="subStore.isRefreshing" class="mb-4 flex items-center gap-3 rounded-lg bg-white/10 px-4 py-2">
          <LoadingSpinner size="sm" />
          <span class="text-white/70">
            刷新 {{ subStore.refreshingName }} ({{ subStore.refreshingIndex }}/{{ subStore.refreshingTotal }})
          </span>
        </div>

        <section v-if="libraryStore.doubanHot.length" class="hot-section mb-8">
          <div class="flex items-center gap-2 mb-4">
            <span class="text-xl">🔥</span>
            <span class="text-lg font-semibold text-white">豆瓣热播</span>
          </div>
          <div class="flex gap-4 overflow-x-auto pb-4">
            <VodCard
              v-for="hot in libraryStore.doubanHot.slice(0, 10)"
              :key="hot.id"
              :item="(hot as any)"
              @click="handleHotClick(hot)"
            />
          </div>
        </section>

        <section class="home-secondary-browser">
          <div v-if="activeTab === 'search'" class="home-secondary-search">
            <SearchBar
              placeholder="搜索电影、剧集、综艺..."
              :keyword="searchKeyword"
              @search="handleVodSearch"
            />
          </div>

          <div v-if="activeTab === 'live'">
            <div v-if="liveStore.loading" class="flex min-h-[220px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="filteredGroups.length === 0" class="home-empty-state">
              暂无直播频道，先去订阅页检查源状态。
            </div>

            <div v-else class="mt-8 space-y-8">
              <div
                v-for="group in filteredGroups"
                :key="group.category"
                class="border-t soft-divider pt-6 first:border-t-0 first:pt-0"
              >
                <div class="mb-4 flex items-center justify-between gap-3">
                  <div>
                    <div class="text-2xl font-semibold text-white">{{ group.category }}</div>
                    <div class="mt-1 text-xs uppercase tracking-[0.24em] text-white/35">{{ group.channels.length }} channels</div>
                  </div>
                  <button
                    v-if="group.channels.length > 12"
                    class="action-button action-button-secondary px-3 py-2 text-xs"
                    type="button"
                    @click="toggleChannelExpansion(group.category)"
                  >
                    {{ expandedChannels.has(group.category) ? '收起' : '查看更多' }}
                  </button>
                </div>

                <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
                  <ChannelCard
                    v-for="channel in expandedChannels.has(group.category) ? group.channels : group.channels.slice(0, 12)"
                    :key="channel.id"
                    :channel="channel"
                    :source-url="channel.sources[0]?.url"
                    @play="handlePlayChannel"
                  />
                </div>
              </div>
            </div>
          </div>

          <div v-else-if="activeTab === 'search'">
            <div v-if="loadingProviderSearch" class="flex min-h-[220px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="searchKeyword && providerSearchResults.length" class="mt-6">
              <!-- Filter pills -->
              <div class="mb-4 flex flex-wrap gap-2">
                <button
                  :class="['rounded-full px-3 py-1 text-xs transition-colors', searchFilter === 'all' ? 'bg-accent/20 text-accent-strong' : 'bg-white/5 text-white/50 hover:text-white/70']"
                  @click="searchFilter = 'all'"
                >
                  全部
                  <span class="opacity-50">({{ allSearchItems.length }})</span>
                </button>
                <button
                  v-for="type in (['movie', 'series', 'variety', 'anime'] as const)"
                  :key="type"
                  :class="['rounded-full px-3 py-1 text-xs transition-colors', searchFilter === type ? 'bg-accent/20 text-accent-strong' : 'bg-white/5 text-white/50 hover:text-white/70']"
                  @click="searchFilter = type"
                >
                  {{ formatTypeLabel(type) }}
                  <span class="opacity-50">({{ countByType[type] }})</span>
                </button>
              </div>

              <!-- Result count -->
              <div class="mb-4 text-sm text-white/40">
                找到 <span class="text-white/60">{{ filteredSearchItems.length }}</span> 个结果
              </div>

              <!-- Result grid -->
              <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                <MediaCard
                  v-for="item in filteredSearchItems"
                  :key="item.source_item_key"
                  :title="item.title"
                  :poster="item.poster"
                  :subtitle="item.source_name"
                  class="cursor-pointer"
                  @click="handleProviderResultClick(item)"
                />
              </div>
            </div>

            <div v-else-if="searchKeyword && !loadingProviderSearch" class="home-empty-state">
              未找到与"{{ searchKeyword }}"相关的内容
            </div>

            <div v-else class="home-empty-state">
              输入关键词搜索电影、剧集、综艺和动漫
            </div>
          </div>

          <div v-else>
            <div v-if="libraryStore.loading" class="flex min-h-[220px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="displayedHotItems.length === 0" class="home-empty-state">
              暂无{{ formatTypeLabel(activeTab) }}热播数据
            </div>

            <div v-else class="mt-6">
              <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                <VodCard
                  v-for="hot in displayedHotItems"
                  :key="hot.id"
                  :item="(hot as any)"
                  @click="handleHotClick(hot)"
                />
              </div>

              </div>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>
