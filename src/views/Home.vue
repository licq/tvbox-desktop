<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useDoubanStore, type MatchedHotItem } from '@/stores/douban'
import { useSubscriptionStore } from '@/stores/subscription'
import { useLibraryStore } from '@/stores/library'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { CatalogCard, CatalogItemType, LiveChannel, VodItem } from '@/types'

type HomeTabKey = 'live' | 'hot' | CatalogItemType

const route = useRoute()
const router = useRouter()
const liveStore = useLiveStore()
const doubanStore = useDoubanStore()
const subStore = useSubscriptionStore()
const libraryStore = useLibraryStore()

const tabs: { key: HomeTabKey; label: string; eyebrow: string }[] = [
  { key: 'live', label: '直播调度台', eyebrow: 'Live' },
  { key: 'hot', label: '热映观察', eyebrow: 'Hot' },
  { key: 'movie', label: '电影片库', eyebrow: 'Movie' },
  { key: 'series', label: '剧集片库', eyebrow: 'Series' },
  { key: 'variety', label: '综艺片库', eyebrow: 'Shows' },
  { key: 'anime', label: '动漫片库', eyebrow: 'Anime' }
]

const activeTab = ref<HomeTabKey>('live')
const searchKeyword = ref('')
const expandedChannels = ref<Set<string>>(new Set())
const showAllVod = ref(false)

const validTabs = new Set<HomeTabKey>(tabs.map(tab => tab.key))

function normalizeTab(tab: string | string[] | undefined): HomeTabKey {
  if (typeof tab === 'string' && validTabs.has(tab as HomeTabKey)) {
    return tab as HomeTabKey
  }

  return 'live'
}

function formatTypeLabel(type: CatalogItemType | HomeTabKey) {
  return tabs.find(tab => tab.key === type)?.label ?? '片库'
}

const matchedHotItems = computed<MatchedHotItem[]>(() => doubanStore.matchedItems)
const enabledSubscriptions = computed(() => subStore.subscriptions.filter(sub => sub.enabled))
const featuredHero = computed(() => libraryStore.featured[0] ?? libraryStore.latestUpdates[0] ?? libraryStore.continueWatching[0] ?? null)
const featuredBackdrop = computed(() => featuredHero.value?.poster ?? '')
const heroLabel = computed(() => tabs.find(tab => tab.key === activeTab.value)?.eyebrow ?? 'Library')
const heroTitle = computed(() => {
  switch (activeTab.value) {
    case 'live':
      return '把直播、片库和源状态放到同一张桌面里'
    case 'hot':
      return '先看今天值得点开的热源和热度变化'
    default:
      return `${formatTypeLabel(activeTab.value)}按片库节奏重新组织`
  }
})
const heroSummary = computed(() => {
  switch (activeTab.value) {
    case 'live':
      return '优先显示可播频道、片库推荐和订阅状态，让入口像媒体中心而不是工具列表。'
    case 'hot':
      return '豆瓣热度和片库命中并列呈现，先看热度，再决定切到哪个源。'
    default:
      return '保留搜索与快速进入，但把重点放到海报、更新状态和可播放路径。'
  }
})
const heroMetrics = computed(() => [
  { label: '启用源', value: `${enabledSubscriptions.value.length}` },
  { label: '直播分组', value: `${liveStore.groups.length}` },
  { label: '推荐片单', value: `${libraryStore.featured.length}` }
])

const filteredGroups = computed(() => {
  if (!searchKeyword.value) return liveStore.groups
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

const spotlightCards = computed(() => libraryStore.featured.slice(0, 4))
const continueCards = computed(() => libraryStore.continueWatching.slice(0, 3))
const latestCards = computed(() => libraryStore.latestUpdates.slice(0, 5))

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
    doubanStore.fetchMatchedHot(),
    libraryStore.fetchHome()
  ])
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

    if (nextTab !== 'live' && nextTab !== 'hot') {
      await libraryStore.fetchCatalog(nextTab)
    }
  },
  { immediate: true }
)

function onTabChange(tab: HomeTabKey) {
  router.push(`/library/${tab}`)
}

function handleLiveSearch(keyword: string) {
  searchKeyword.value = keyword
}

function handleVodSearch(keyword: string) {
  if (keyword) {
    if (activeTab.value !== 'live' && activeTab.value !== 'hot') {
      void libraryStore.fetchCatalog(activeTab.value, keyword)
    }
    return
  }

  showAllVod.value = false
  if (activeTab.value !== 'live' && activeTab.value !== 'hot') {
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
      <header class="surface-panel rounded-[2rem] px-5 py-5 md:px-7">
        <div class="flex flex-col gap-5 xl:flex-row xl:items-center xl:justify-between">
          <div>
            <div class="eyebrow">TVBox Desktop</div>
            <div class="mt-2 text-3xl font-semibold tracking-[0.08em] text-white md:text-4xl">饭太硬媒体中枢</div>
            <p class="mt-3 max-w-2xl text-sm text-white/55 md:text-base">
              把直播、片库、热度和源状态压进一个连续界面里，先看什么、从哪条线进、哪里失效，一眼能分清。
            </p>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            <RouterLink to="/subscriptions" class="action-button action-button-primary">
              订阅管理
            </RouterLink>
            <RouterLink to="/settings" class="action-button action-button-secondary">
              设置
            </RouterLink>
          </div>
        </div>

        <div class="mt-5 flex flex-wrap gap-2">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            :class="['nav-pill', activeTab === tab.key ? 'nav-pill-active' : '']"
            @click="onTabChange(tab.key)"
          >
            {{ tab.label }}
          </button>
        </div>
      </header>

      <main class="mt-6 grid gap-6 xl:grid-cols-[minmax(0,1fr)_320px]">
        <div class="space-y-6">
          <section class="surface-panel relative overflow-hidden rounded-[2rem] px-6 py-7 md:px-8 md:py-9">
            <div
              class="absolute inset-0 opacity-35"
              :style="featuredBackdrop ? { backgroundImage: `linear-gradient(90deg, rgba(8, 10, 16, 0.94), rgba(8, 10, 16, 0.5)), url(${featuredBackdrop})`, backgroundSize: 'cover', backgroundPosition: 'center' } : undefined"
            ></div>
            <div class="relative grid gap-8 xl:grid-cols-[minmax(0,1.2fr)_360px]">
              <div>
                <div class="eyebrow">{{ heroLabel }}</div>
                <h1 class="mt-3 max-w-3xl text-4xl font-semibold leading-tight text-white md:text-5xl">
                  {{ heroTitle }}
                </h1>
                <p class="mt-4 max-w-2xl text-sm leading-7 text-white/62 md:text-base">
                  {{ heroSummary }}
                </p>

                <div class="mt-6 max-w-2xl">
                  <SearchBar
                    :placeholder="activeTab === 'live' ? '搜索频道、卫视、央视频道...' : `搜索${formatTypeLabel(activeTab)}...`"
                    @search="activeTab === 'live' ? handleLiveSearch($event) : handleVodSearch($event)"
                  />
                </div>

                <div class="mt-6 flex flex-wrap gap-3">
                  <button
                    v-if="featuredHero"
                    class="action-button action-button-primary"
                    @click="handleCatalogClick(featuredHero)"
                  >
                    打开推荐内容
                  </button>
                  <RouterLink to="/subscriptions" class="action-button action-button-secondary">
                    查看源状态
                  </RouterLink>
                </div>
              </div>

              <div class="grid gap-3 sm:grid-cols-3 xl:grid-cols-1">
                <div
                  v-for="metric in heroMetrics"
                  :key="metric.label"
                  class="surface-muted rounded-[1.5rem] p-5"
                >
                  <div class="text-[11px] uppercase tracking-[0.28em] text-white/38">{{ metric.label }}</div>
                  <div class="mt-3 text-3xl font-semibold text-white">{{ metric.value }}</div>
                </div>
              </div>
            </div>
          </section>

          <section v-if="spotlightCards.length" class="surface-panel rounded-[2rem] px-6 py-6 md:px-7">
            <div class="flex items-center justify-between gap-4">
              <div>
                <div class="section-title">片库推荐</div>
                <p class="mt-2 text-sm text-white/52">优先把新 query 面给出的推荐条目放到首页，而不是让用户先自己翻目录。</p>
              </div>
            </div>

            <div class="mt-6 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <button
                v-for="card in spotlightCards"
                :key="card.id"
                class="surface-muted overflow-hidden rounded-[1.75rem] text-left transition duration-300 hover:-translate-y-1"
                @click="handleCatalogClick(card)"
              >
                <div
                  class="aspect-[4/5] bg-slate-900 bg-cover bg-center"
                  :style="card.poster ? { backgroundImage: `linear-gradient(180deg, rgba(8, 12, 18, 0.08), rgba(8, 12, 18, 0.88)), url(${card.poster})` } : undefined"
                >
                  <div class="flex h-full flex-col justify-end p-4">
                    <div class="text-[10px] uppercase tracking-[0.28em] text-white/60">{{ formatTypeLabel(card.item_type) }}</div>
                    <div class="mt-2 text-lg font-semibold text-white">{{ card.title }}</div>
                    <div class="mt-3 text-xs text-white/45">
                      {{ card.update_badge || card.source_badge || '来自片库推荐' }}
                    </div>
                  </div>
                </div>
              </button>
            </div>
          </section>

          <template v-if="activeTab === 'live'">
            <section class="grid gap-6 lg:grid-cols-2">
              <div class="surface-panel rounded-[2rem] px-6 py-6">
                <div class="section-title">继续观看</div>
                <div v-if="libraryStore.loading" class="flex min-h-[160px] items-center justify-center">
                  <LoadingSpinner />
                </div>
                <div v-else-if="continueCards.length" class="mt-5 space-y-3">
                  <button
                    v-for="card in continueCards"
                    :key="card.id"
                    class="surface-muted flex w-full items-center gap-4 rounded-[1.4rem] p-4 text-left transition hover:bg-white/[0.07]"
                    @click="handleCatalogClick(card)"
                  >
                    <div
                      class="h-20 w-16 shrink-0 rounded-2xl bg-cover bg-center"
                      :style="card.poster ? { backgroundImage: `url(${card.poster})` } : undefined"
                    ></div>
                    <div class="min-w-0 flex-1">
                      <div class="text-sm font-semibold text-white">{{ card.title }}</div>
                      <div class="mt-2 text-xs uppercase tracking-[0.24em] text-white/38">{{ formatTypeLabel(card.item_type) }}</div>
                      <div class="mt-3 h-1.5 overflow-hidden rounded-full bg-white/8">
                        <div class="h-full rounded-full bg-[#d89a57]" :style="{ width: `${card.progress ?? 0}%` }"></div>
                      </div>
                    </div>
                  </button>
                </div>
                <div v-else class="mt-5 text-sm text-white/45">还没有播放历史，等点开一条片库内容后会出现在这里。</div>
              </div>

              <div class="surface-panel rounded-[2rem] px-6 py-6">
                <div class="section-title">最近更新</div>
                <div v-if="latestCards.length" class="mt-5 space-y-3">
                  <button
                    v-for="card in latestCards"
                    :key="card.id"
                    class="flex w-full items-center justify-between gap-4 rounded-[1.2rem] border border-white/6 px-4 py-3 text-left transition hover:bg-white/[0.04]"
                    @click="handleCatalogClick(card)"
                  >
                    <div>
                      <div class="text-sm font-medium text-white">{{ card.title }}</div>
                      <div class="mt-2 text-[11px] uppercase tracking-[0.24em] text-white/36">{{ formatTypeLabel(card.item_type) }}</div>
                    </div>
                    <div class="text-xs text-white/48">{{ card.update_badge || card.source_badge || '已入库' }}</div>
                  </button>
                </div>
                <div v-else class="mt-5 text-sm text-white/45">当前还没有同步到最近更新列表。</div>
              </div>
            </section>

            <section class="surface-panel rounded-[2rem] px-6 py-6 md:px-7">
              <div class="flex items-center justify-between gap-4">
                <div>
                  <div class="section-title">直播频道</div>
                  <p class="mt-2 text-sm text-white/52">按分组展开，保留快速搜索，同时把“可用线路数”前置出来。</p>
                </div>
              </div>

              <div v-if="liveStore.loading" class="flex min-h-[220px] items-center justify-center">
                <LoadingSpinner />
              </div>

              <div v-else-if="filteredGroups.length === 0" class="mt-8 text-center text-white/45">
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
            </section>
          </template>

          <section
            v-if="activeTab === 'hot'"
            class="surface-panel rounded-[2rem] px-6 py-6 md:px-7"
          >
            <div class="section-title">热映观察</div>

            <div v-if="doubanStore.loading" class="flex min-h-[240px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="matchedHotItems.length === 0" class="mt-6 text-sm text-white/45">
              暂无热门数据。
            </div>

            <div v-else class="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5">
              <button
                v-for="item in matchedHotItems"
                :key="item.douban.id"
                class="surface-muted overflow-hidden rounded-[1.75rem] text-left transition hover:-translate-y-1"
                @click="router.push(`/detail/${item.vod_id}`)"
              >
                <div
                  class="aspect-[4/5] bg-cover bg-center"
                  :style="item.douban.poster ? { backgroundImage: `linear-gradient(180deg, rgba(8, 12, 18, 0.06), rgba(8, 12, 18, 0.9)), url(${item.douban.poster})` } : undefined"
                >
                  <div class="flex h-full flex-col justify-between p-4">
                    <div class="inline-flex w-fit rounded-full bg-white/10 px-3 py-1 text-[10px] uppercase tracking-[0.28em] text-white/68">
                      TOP {{ item.douban.rank }}
                    </div>
                    <div>
                      <div class="text-lg font-semibold text-white">{{ item.vod_name || item.douban.name }}</div>
                      <div class="mt-2 text-xs text-white/55">豆瓣热度命中片库</div>
                    </div>
                  </div>
                </div>
              </button>
            </div>
          </section>

          <section
            v-if="activeTab !== 'live' && activeTab !== 'hot'"
            class="surface-panel rounded-[2rem] px-6 py-6 md:px-7"
          >
            <div class="flex items-end justify-between gap-4">
              <div>
                <div class="section-title">{{ formatTypeLabel(activeTab) }}</div>
                <p class="mt-2 text-sm text-white/52">保留目录浏览，但把第一屏留给海报和快速进入，不再用密集小按钮压缩信息。</p>
              </div>
              <div class="text-xs uppercase tracking-[0.24em] text-white/35">{{ libraryStore.catalogItems.length }} items</div>
            </div>

            <div v-if="libraryStore.loading" class="flex min-h-[240px] items-center justify-center">
              <LoadingSpinner />
            </div>

            <div v-else-if="libraryStore.catalogItems.length === 0" class="mt-6 text-sm text-white/45">
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
                <button
                  class="action-button action-button-secondary"
                  @click="showAllVod = true"
                >
                  加载更多
                </button>
              </div>
            </div>
          </section>
        </div>

        <aside class="space-y-6">
          <section class="surface-panel rounded-[2rem] px-5 py-5">
            <div class="section-title">源状态</div>
            <div class="mt-5 space-y-4">
              <div class="surface-muted rounded-[1.4rem] p-4">
                <div class="text-[11px] uppercase tracking-[0.24em] text-white/36">Enabled</div>
                <div class="mt-2 text-3xl font-semibold text-white">{{ enabledSubscriptions.length }}</div>
              </div>
              <div class="surface-muted rounded-[1.4rem] p-4">
                <div class="text-[11px] uppercase tracking-[0.24em] text-white/36">Failures</div>
                <div class="mt-2 text-3xl font-semibold text-white">
                  {{ subStore.subscriptions.filter(sub => sub.enabled && sub.last_error).length }}
                </div>
              </div>
            </div>
          </section>

          <section class="surface-panel rounded-[2rem] px-5 py-5">
            <div class="section-title">订阅列表</div>
            <div class="mt-5 space-y-3">
              <div
                v-for="subscription in subStore.subscriptions.slice(0, 6)"
                :key="subscription.id"
                class="rounded-[1.2rem] border border-white/6 px-4 py-3"
              >
                <div class="flex items-center justify-between gap-3">
                  <div class="text-sm font-medium text-white">{{ subscription.name }}</div>
                  <div
                    :class="[
                      'rounded-full px-2 py-1 text-[10px] uppercase tracking-[0.2em]',
                      !subscription.enabled
                        ? 'bg-white/10 text-white/60'
                        : subscription.last_error
                          ? 'bg-red-500/15 text-red-200'
                          : 'bg-emerald-500/15 text-emerald-200'
                    ]"
                  >
                    {{ !subscription.enabled ? '停用' : (subscription.last_error ? '异常' : '正常') }}
                  </div>
                </div>
                <div class="mt-2 text-[11px] uppercase tracking-[0.22em] text-white/32">{{ subscription.kind }}</div>
              </div>
            </div>
          </section>

          <section class="surface-panel rounded-[2rem] px-5 py-5">
            <div class="section-title">最近更新速览</div>
            <div class="mt-5 space-y-3">
              <button
                v-for="card in latestCards.slice(0, 4)"
                :key="card.id"
                class="surface-muted flex w-full items-center justify-between gap-3 rounded-[1.2rem] px-4 py-3 text-left"
                @click="handleCatalogClick(card)"
              >
                <div class="min-w-0">
                  <div class="truncate text-sm font-medium text-white">{{ card.title }}</div>
                  <div class="mt-1 text-[11px] uppercase tracking-[0.24em] text-white/35">{{ formatTypeLabel(card.item_type) }}</div>
                </div>
                <div class="text-xs text-white/42">{{ card.update_badge || '更新' }}</div>
              </button>
            </div>
          </section>
        </aside>
      </main>
    </div>
  </div>
</template>
