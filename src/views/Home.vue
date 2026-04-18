<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterView } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { useVodStore } from '@/stores/vod'
import ChannelCard from '@/components/ChannelCard.vue'
import VodCard from '@/components/VodCard.vue'
import type { LiveChannel, VodItem } from '@/types'

const liveStore = useLiveStore()
const vodStore = useVodStore()

const activeTab = ref<'live' | 'vod'>('live')

onMounted(async () => {
  await Promise.all([
    liveStore.fetchChannels(),
    vodStore.fetchItems()
  ])
})

function handlePlayChannel(channel: LiveChannel) {
  window.location.href = `/player/live/${channel.id}`
}

function handleVodClick(item: VodItem) {
  window.location.href = `/vod/${item.id}`
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

    <!-- Tabs -->
    <div class="flex border-b border-gray-700">
      <button
        :class="['px-6 py-3 text-lg', activeTab === 'live' ? 'border-b-2 border-primary text-primary' : 'text-gray-400']"
        @click="activeTab = 'live'"
      >
        📺 直播
      </button>
      <button
        :class="['px-6 py-3 text-lg', activeTab === 'vod' ? 'border-b-2 border-primary text-primary' : 'text-gray-400']"
        @click="activeTab = 'vod'"
      >
        🎬 点播
      </button>
    </div>

    <!-- Content -->
    <main class="p-4">
      <!-- Live Tab -->
      <div v-if="activeTab === 'live'">
        <div v-if="liveStore.loading" class="flex justify-center py-8">
          <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
        </div>
        <div v-else-if="liveStore.channels.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无频道</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>
        <div v-else class="grid grid-cols-4 md:grid-cols-6 lg:grid-cols-8 gap-4">
          <ChannelCard
            v-for="channel in liveStore.channels"
            :key="channel.id"
            :channel="channel"
            @play="handlePlayChannel"
          />
        </div>
      </div>

      <!-- VOD Tab -->
      <div v-if="activeTab === 'vod'">
        <div v-if="vodStore.loading" class="flex justify-center py-8">
          <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
        </div>
        <div v-else-if="vodStore.items.length === 0" class="text-center py-8 text-gray-400">
          <p class="text-xl mb-4">暂无影视</p>
          <RouterLink to="/subscriptions" class="text-primary hover:underline">
            添加订阅源
          </RouterLink>
        </div>
        <div v-else class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4">
          <VodCard
            v-for="item in vodStore.items"
            :key="item.id"
            :item="item"
            @click="handleVodClick"
          />
        </div>
      </div>
    </main>
  </div>
</template>
