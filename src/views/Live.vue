<script setup lang="ts">
import { onMounted } from 'vue'
import { useLiveStore } from '@/stores/live'
import ChannelCard from '@/components/ChannelCard.vue'
import type { LiveChannel } from '@/types'

const liveStore = useLiveStore()

onMounted(async () => {
  await liveStore.fetchChannels()
  await liveStore.fetchCategories()
})

function handlePlayChannel(channel: LiveChannel) {
  window.location.href = `/player/live/${channel.id}`
}
</script>

<template>
  <div class="live-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <h1 class="text-2xl font-bold mb-4">📺 直播电视</h1>

      <!-- Categories -->
      <div class="flex gap-2 flex-wrap">
        <button
          class="px-3 py-1 bg-gray-700 rounded hover:bg-primary transition"
          @click="liveStore.fetchChannels()"
        >
          全部
        </button>
        <button
          v-for="cat in liveStore.categories"
          :key="cat"
          class="px-3 py-1 bg-gray-700 rounded hover:bg-primary transition"
          @click="liveStore.fetchChannels(cat)"
        >
          {{ cat }}
        </button>
      </div>
    </header>

    <div v-if="liveStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="liveStore.channels.length === 0" class="text-center py-8 text-gray-400">
      暂无频道
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
</template>
