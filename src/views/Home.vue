<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useSubscriptionStore } from '@/stores/subscription'
import { useLibraryStore } from '@/stores/library'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import MediaRail from '@/components/home/MediaRail.vue'
import LiveNowPanel from '@/components/home/LiveNowPanel.vue'
import type { CatalogCard, CatalogItemType, LiveChannel, VodItem } from '@/types'

type HomeTabKey = 'live' | CatalogItemType

const route = useRoute()
const router = useRouter()
const liveStore = useLiveStore()
const subStore = useSubscriptionStore()
const libraryStore = useLibraryStore()

const tabLabels: Record<string, { label: string; eyebrow: string }> = {
  live: { label: '直播', eyebrow: 'Live' },
  movie: { label: '电影', eyebrow: 'Movie' },
  series: { label: '剧集', eyebrow: 'Series' },
  variety: { label: '综艺', eyebrow: 'Shows' },
  anime: { label: '动漫', eyebrow: 'Anime' },
  short_drama: { label: '短剧', eyebrow: 'Short' },
  web_drama: { label: '网剧', eyebrow: 'Web' }
}

const tabs = computed(() => {
  return ['live', ...libraryStore.availableTypes]
    .filter(type => tabLabels[type])
    .map(key => ({
      key,
      ...tabLabels[key]
    }))
})

const activeTab = ref<HomeTabKey>('live')
const searchKeyword = ref('')
const expandedChannels = ref<Set<string>>(new Set())
const showAllVod = ref(false)

const validTabs = computed(() => new Set(tabs.value.map(tab => tab.key)))
const catalogTypes: CatalogItemType[] = ['movie', 'series', 'variety', 'anime']

function normalizeTab(tab: string | string[] | undefined): HomeTabKey {
  if (typeof tab === 'string' && validTabs.value.has(tab as HomeTabKey)) {
    return tab as HomeTabKey
  }

  return 'live'
}

function formatTypeLabel(type: CatalogItemType | HomeTabKey) {
  return tabs.value.find(tab => tab.key === type)?.label ?? '片库'
}

const enabledSubscriptions = computed(() => subStore.subscriptions.filter(sub => sub.enabled))

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

const displayedVodItems = computed(() => {
  if (showAllVod.value) return libraryStore.catalogItems
  return libraryStore.catalogItems.slice(0, 18)
})

const railSummaries: Record<string, string> = {
  movie: '用横向海报流先给电影入口，而不是要求先筛选。',
  series: '剧集更新和继续观看并排进入，适合长线追更。',
  variety: '综艺内容保留轻量浏览节奏，快速判断是否可播。',
  anime: '动漫片库独立成排，避免被电影和剧集淹没。',
  short_drama: '短剧内容独立展示，方便快速浏览。',
  web_drama: '网剧内容独立展示，避免与普通剧集混淆。'
}

const rails = computed(() =>
  catalogTypes
    .map(type => ({
      type,
      title: formatTypeLabel(type),
      summary: railSummaries[type],
      items: libraryStore.getRail(type)
    }))
    .filter(rail => rail.items.length > 0)
)

const activeTabMeta = computed(() => tabs.value.find(tab => tab.key === activeTab.value) ?? tabs.value[0])

async function hydrateSources() {
  try {
    await subStore.fetchSubscriptions()
  } catch {
    // Keep rendering with whatever cache exists.
  }

  for (const subscription of enabledSubscriptions.value) {
    try {
      await subStore.refreshSubscription(subscription.id, false)
    } catch {
      // Continue refreshing other subscriptions even if one fails.
    }
  }

  try {
    await subStore.fetchSubscriptions()
  } catch {
    // Keep rendering with whatever cache exists.
  }

  await Promise.allSettled([
    liveStore.fetchGroups(),
    libraryStore.fetchHome(),
    libraryStore.fetchCatalog()
  ])

  if (activeTab.value !== 'live') {
    await libraryStore.fetchCatalog(activeTab.value)
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
    searchKeyword.value = ''
    showAllVod.value = false

    if (nextTab !== 'live') {
      await libraryStore.fetchCatalog(nextTab)
    }
  },
  { immediate: true }
)

function onTabChange(tab: string) {
  router.push(`/library/${tab}`)
}

function handleLiveSearch(keyword: string) {
  searchKeyword.value = keyword
}

function handleVodSearch(keyword: string) {
  if (keyword) {
    if (activeTab.value !== 'live') {
      void libraryStore.fetchCatalog(activeTab.value, keyword)
    }
    return
  }

  showAllVod.value = false
  if (activeTab.value !== 'live') {
    void libraryStore.fetchCatalog(activeTab.value)
  }
}

function handlePlayChannel(channel: LiveChannel, _sourceUrl?: string) {
  router.push(`/player/live/${channel.id}`)
}

function handleVodClick(item: CatalogCard | VodItem) {
  router.push(`/detail/${item.id}`)
}

function handleCatalogClick(card: CatalogCard) {
  router.push(`/detail/${card.id}`)
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
          <div class="home-topbar-title">饭太硬媒体中枢</div>
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
        <MediaRail
          v-for="rail in rails"
          :key="rail.type"
          :title="rail.title"
          :summary="rail.summary"
          :items="rail.items"
          @select="handleCatalogClick"
        />

        <LiveNowPanel :groups="liveStore.groups" @play="handlePlayChannel" />

        <section class="home-secondary-browser">
          <div class="home-secondary-header">
            <div>
              <div class="eyebrow">{{ activeTabMeta.eyebrow }}</div>
              <h2>{{ formatTypeLabel(activeTab) }}浏览</h2>
            </div>

            <div class="home-secondary-search">
              <SearchBar
                :placeholder="activeTab === 'live' ? '搜索频道、卫视、央视频道...' : `搜索${formatTypeLabel(activeTab)}...`"
                @search="activeTab === 'live' ? handleLiveSearch($event) : handleVodSearch($event)"
              />
            </div>
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

          <div v-else>
            <div v-if="libraryStore.loading" class="flex min-h-[220px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="libraryStore.catalogItems.length === 0" class="home-empty-state">
              暂无{{ formatTypeLabel(activeTab) }}，先检查订阅源是否成功刷新。
            </div>

            <div v-else class="mt-6">
              <div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
                <VodCard
                  v-for="item in displayedVodItems"
                  :key="item.id"
                  :item="item"
                  @click="handleVodClick"
                />
              </div>

              <div v-if="libraryStore.catalogItems.length > 18 && !showAllVod" class="mt-8 flex justify-center">
                <button class="action-button action-button-secondary" type="button" @click="showAllVod = true">
                  加载更多
                </button>
              </div>
            </div>
          </div>
        </section>
      </main>
    </div>
  </div>
</template>
