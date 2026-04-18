<script setup lang="ts">
import type { VodItem } from '@/types'

defineProps<{
  item: VodItem
}>()

defineEmits<{
  click: [item: VodItem]
}>()
</script>

<template>
  <div
    class="vod-card bg-gray-800 rounded-lg overflow-hidden cursor-pointer hover:bg-gray-700 transition-colors"
    @click="$emit('click', item)"
  >
    <img
      v-if="item.poster"
      :src="item.poster"
      :alt="item.name"
      class="w-full aspect-[2/3] object-cover"
    />
    <div v-else class="w-full aspect-[2/3] bg-gray-600 flex items-center justify-center text-4xl">
      🎬
    </div>
    <div class="p-2">
      <div class="text-sm truncate">{{ item.name }}</div>
      <div class="text-xs text-gray-400 mt-1">{{ item.type }}</div>
    </div>
  </div>
</template>
