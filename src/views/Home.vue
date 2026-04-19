<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useVodStore } from '@/stores/vod'
import { useDoubanStore, type MatchedHotItem } from '@/stores/douban'
import { useSubscriptionStore } from '@/stores/subscription'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { LiveChannel, VodItem } from '@/types'

const router = useRouter()
const liveStore = useLiveStore()
const vodStore = useVodStore()
const doubanStore = useDoubanStore()
const subStore = useSubscriptionStore()

const tabs = [
  { key: 'live', label: '直播', icon: '📺' },
  { key: 'hot', label: '热门', icon: '🔥' },
  { key: 'movie', label: '电影', icon: '🎬' },
  { key: 'tv', label: '电视剧', icon: '📺' },
  { key: 'variety', label: '综艺', icon: '🎭' },
  { key: 'anime', label: '动漫', icon: '🅰️' }
]

const activeTab = ref('live')
const searchKeyword = ref('')
const expandedChannels = ref<Set<string>>(new Set())
const showAllVod = ref(false)

const matchedHotItems = computed<MatchedHotItem[]>(() => doubanStore.matchedItems)

function getTabLabel(tab: string): string {
  return tabs.find(t => t.key === tab)?.label || ''
}

onMounted(async () => {
  // Fetch all initial data
  await subStore.fetchSubscriptions()

  const refreshPromises = subStore.subscriptions
    .filter(sub => sub.enabled)
    .map(sub => subStore.refreshSubscription(sub.id))
  await Promise.allSettled(refreshPromises)

  // Fetch grouped live channels
  await liveStore.fetchGroups()

  // Fetch matched hot list
  await doubanStore.fetchMatchedHot()
})

function handleLiveSearch(keyword: string) {
  searchKeyword.value = keyword
}

function handleVodSearch(keyword: string) {
  if (keyword) {
    vodStore.search(keyword)
  } else {
    showAllVod.value = false
    const vtype = activeTab.value === 'hot' ? undefined : activeTab.value
    vodStore.fetchItems(vtype)
  }
}

function handlePlayChannel(channel: LiveChannel, _sourceUrl?: string) {
  router.push(`/player/live/${channel.id}`)
}

function handleVodClick(item: VodItem) {
  router.push(`/vod/${item.id}`)
}

function toggleChannelExpansion(category: string) {
  if (expandedChannels.value.has(category)) {
    expandedChannels.value.delete(category)
  } else {
    expandedChannels.value.add(category)
  }
}

// Computed for filtered live channels based on search
const filteredGroups = computed(() => {
  if (!searchKeyword.value) return liveStore.groups
  const keyword = searchKeyword.value.toLowerCase()
  return liveStore.groups.map(group => ({
    ...group,
    channels: group.channels.filter(ch =>
      ch.name.toLowerCase().includes(keyword)
    )
  })).filter(group => group.channels.length > 0)
})

// Computed for displayed VOD items (first 20 or all if showAllVod)
const displayedVodItems = computed(() => {
  if (showAllVod.value) return vodStore.items
  return vodStore.items.slice(0, 20)
})

// Fetch VOD items when switching to a VOD tab
function onTabChange(tab: string) {
  activeTab.value = tab
  searchKeyword.value = ''
  showAllVod.value = false

  if (tab === 'live') {
    // Already loaded via fetchGroups
  } else if (tab === 'hot') {
    // Hot uses matchedHotItems, no fetch needed
  } else {
    // movie, tv, variety, anime
    vodStore.fetchItems(tab)
  }
}
</script>

<template>
  <div class="home min-h-screen bg-gray-900 text-white">
    <!-- Header -->
    <header class="bg-gray-800 p-4 flex items-center justify-between">
      <h1 class="text-2xl font-bold">📺 TVBox 影视仓</h1>
      <div class="flex gap-4">
        <RouterLink to="/subscriptions" class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition">
          订阅管理
        </RouterLink>
        <RouterLink to="/settings" class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition">
          ⚙️
        </RouterLink>
      </div>
    </header>

    <!-- Tab Navigation -->
    <div class="flex border-b border-gray-700 overflow-x-auto">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        :class="[
          'px-4 py-3 text-sm md:text-base whitespace-nowrap',
          activeTab === tab.key ? 'border-b-2 border-primary text-primary' : 'text-gray-400 hover:text-white'
        ]"
        @click="onTabChange(tab.key)"
      >
        {{ tab.icon }} {{ tab.label }}
      </button>
    </div>

    <!-- Content -->
    <main class="p-4">

      <!-- Live Tab -->
      <div v-if="activeTab === 'live'">
        <div class="mb-4">
          <SearchBar
            placeholder="搜索频道..."
            @search="handleLiveSearch"
          />
        </div>

        <div v-if="liveStore.loading" class="flex justify-center py-8">
          <LoadingSpinner />
        </div>

        <div v-else-if="liveStore.groups.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无频道</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>

        <div v-else>
          <div
            v-for="group in filteredGroups"
            :key="group.category"
            class="mb-6"
          >
            <div class="flex items-center justify-between mb-3">
              <h2 class="text-lg font-semibold text-gray-200">{{ group.category }}</h2>
              <button
                v-if="group.channels.length > 20"
                class="text-sm text-primary hover:underline"
                @click="toggleChannelExpansion(group.category)"
              >
                {{ expandedChannels.has(group.category) ? '收起' : '展开更多' }}
              </button>
            </div>

            <div class="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
              <ChannelCard
                v-for="channel in expandedChannels.has(group.category) ? group.channels : group.channels.slice(0, 20)"
                :key="channel.id"
                :channel="channel"
                :source-url="channel.sources[0]?.url"
                @play="handlePlayChannel"
              />
            </div>
          </div>
        </div>
      </div>

      <!-- Hot Tab -->
      <div v-if="activeTab === 'hot'">
        <div v-if="doubanStore.loading" class="flex justify-center py-8">
          <LoadingSpinner />
        </div>

        <div v-else-if="matchedHotItems.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无热门数据</p>
        </div>

        <div v-else class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
          <VodCard
            v-for="item in matchedHotItems"
            :key="item.douban.id"
            :item="{
              id: item.vod_id,
              subscription_id: 0,
              name: item.vod_name || item.douban.name,
              type: 'movie',
              poster: item.douban.poster,
              description: '',
              episodes: []
            }"
            @click="(vodItem) => router.push(`/vod/${vodItem.id}`)"
          />
        </div>
      </div>

      <!-- Movie, TV, Variety, Anime Tabs -->
      <div v-if="['movie', 'tv', 'variety', 'anime'].includes(activeTab)">
        <div class="mb-4">
          <SearchBar
            :placeholder="`搜索${getTabLabel(activeTab)}...`"
            @search="handleVodSearch"
          />
        </div>

        <div v-if="vodStore.loading" class="flex justify-center py-8">
          <LoadingSpinner />
        </div>

        <div v-else-if="vodStore.items.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无{{ getTabLabel(activeTab) }}</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>

        <div v-else>
          <div class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
            <VodCard
              v-for="item in displayedVodItems"
              :key="item.id"
              :item="item"
              @click="handleVodClick"
            />
          </div>

          <div v-if="vodStore.items.length > 20 && !showAllVod" class="text-center mt-6">
            <button
              class="px-6 py-2 bg-gray-700 rounded hover:bg-gray-600 transition"
              @click="showAllVod = true"
            >
              加载更多 ({{ vodStore.items.length }})
            </button>
          </div>
        </div>
      </div>

    </main>
  </div>
</template>
