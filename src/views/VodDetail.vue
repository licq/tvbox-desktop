<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import type { CatalogEpisode } from '@/types'

const route = useRoute()
const router = useRouter()
const detailStore = useDetailStore()

const itemId = computed(() => Number(route.params.itemId))

async function loadDetail() {
  if (!Number.isFinite(itemId.value) || itemId.value <= 0) {
    return
  }

  await detailStore.fetchDetail(itemId.value)
}

onMounted(loadDetail)

watch(itemId, loadDetail)

function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}`)
}
</script>

<template>
  <div class="vod-detail-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="router.back()"
      >
        ← 返回
      </button>
    </header>

    <div v-if="detailStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="detailStore.item" class="max-w-4xl mx-auto">
      <!-- Info -->
      <div class="flex gap-6 mb-6">
        <img
          v-if="detailStore.item.poster"
          :src="detailStore.item.poster"
          :alt="detailStore.item.title"
          class="w-48 aspect-[2/3] object-cover rounded-lg"
        />
        <div v-else class="w-48 aspect-[2/3] bg-gray-700 rounded-lg flex items-center justify-center text-4xl">
          🎬
        </div>
        <div class="flex-1">
          <h1 class="text-2xl font-bold mb-2">{{ detailStore.item.title }}</h1>
          <div class="text-gray-400 mb-2">类型: {{ detailStore.item.item_type }}</div>
          <p v-if="detailStore.item.summary" class="text-gray-300">
            {{ detailStore.item.summary }}
          </p>
        </div>
      </div>

      <!-- Episodes -->
      <div v-if="detailStore.episodeGroups.length" class="mt-6 space-y-6">
        <div
          v-for="group in detailStore.episodeGroups"
          :key="group.source_name"
        >
          <h2 class="text-xl font-bold mb-4">{{ group.source_name }}</h2>
          <div class="grid grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2">
          <button
            v-for="episode in group.episodes"
            :key="episode.id"
            class="px-3 py-2 bg-gray-800 rounded hover:bg-primary transition text-center"
            @click="handlePlay(episode)"
          >
            {{ episode.episode_label }}
          </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
