<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useVodStore } from '@/stores/vod'
import VodCard from '@/components/VodCard.vue'
import SearchBar from '@/components/SearchBar.vue'
import type { VodItem } from '@/types'

const vodStore = useVodStore()
const selectedType = ref<string | null>(null)

const types = [
  { label: '全部', value: null },
  { label: '电影', value: 'movie' },
  { label: '电视剧', value: 'tv' },
  { label: '综艺', value: 'variety' },
  { label: '动漫', value: 'anime' }
]

onMounted(() => {
  vodStore.fetchItems()
})

function handleSearch(keyword: string) {
  if (keyword) {
    vodStore.search(keyword)
  } else {
    vodStore.fetchItems(selectedType.value || undefined)
  }
}

function handleTypeChange(type: string | null) {
  selectedType.value = type
  vodStore.fetchItems(type || undefined)
}

function handleVodClick(item: VodItem) {
  window.location.href = `/vod/${item.id}`
}
</script>

<template>
  <div class="vod-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <h1 class="text-2xl font-bold mb-4">🎬 影视点播</h1>
      <SearchBar placeholder="搜索影视..." @search="handleSearch" />

      <!-- Type Filter -->
      <div class="flex gap-2 mt-4 flex-wrap">
        <button
          v-for="t in types"
          :key="t.value ?? 'all'"
          :class="[
            'px-3 py-1 rounded transition',
            selectedType === t.value ? 'bg-primary' : 'bg-gray-700 hover:bg-gray-600'
          ]"
          @click="handleTypeChange(t.value)"
        >
          {{ t.label }}
        </button>
      </div>
    </header>

    <div v-if="vodStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="vodStore.items.length === 0" class="text-center py-8 text-gray-400">
      暂无影视
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
</template>
