<script setup lang="ts">
import { onMounted } from 'vue'
import { useVodStore } from '@/stores/vod'
import type { Episode } from '@/types'

const vodStore = useVodStore()

// Get id from URL
const pathParts = window.location.pathname.split('/')
const id = parseInt(pathParts[pathParts.length - 1])

onMounted(() => {
  vodStore.fetchDetail(id)
})

function handlePlay(episode: Episode) {
  window.location.href = `/player/vod/${id}?episode=${encodeURIComponent(episode.url)}`
}
</script>

<template>
  <div class="vod-detail-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="window.history.back()"
      >
        ← 返回
      </button>
    </header>

    <div v-if="vodStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="vodStore.currentItem" class="max-w-4xl mx-auto">
      <!-- Info -->
      <div class="flex gap-6 mb-6">
        <img
          v-if="vodStore.currentItem.poster"
          :src="vodStore.currentItem.poster"
          :alt="vodStore.currentItem.name"
          class="w-48 aspect-[2/3] object-cover rounded-lg"
        />
        <div v-else class="w-48 aspect-[2/3] bg-gray-700 rounded-lg flex items-center justify-center text-4xl">
          🎬
        </div>
        <div class="flex-1">
          <h1 class="text-2xl font-bold mb-2">{{ vodStore.currentItem.name }}</h1>
          <div class="text-gray-400 mb-2">类型: {{ vodStore.currentItem.type }}</div>
          <p v-if="vodStore.currentItem.description" class="text-gray-300">
            {{ vodStore.currentItem.description }}
          </p>
        </div>
      </div>

      <!-- Episodes -->
      <div v-if="vodStore.currentItem.episodes?.length" class="mt-6">
        <h2 class="text-xl font-bold mb-4">选集</h2>
        <div class="grid grid-cols-6 gap-2">
          <button
            v-for="(ep, idx) in vodStore.currentItem.episodes"
            :key="idx"
            class="px-3 py-2 bg-gray-800 rounded hover:bg-primary transition text-center"
            @click="handlePlay(ep)"
          >
            {{ ep.name }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
