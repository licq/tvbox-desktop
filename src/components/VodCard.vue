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
  <button
    class="group relative w-full overflow-hidden rounded-[1.75rem] text-left poster-shadow transition duration-500 hover:-translate-y-1"
    @click="$emit('click', item)"
  >
    <div class="absolute inset-0 bg-gradient-to-t from-black via-black/25 to-transparent opacity-90"></div>
    <img
      v-if="item.poster"
      :src="item.poster"
      :alt="item.name"
      class="h-full w-full aspect-[2/3] object-cover transition duration-700 group-hover:scale-[1.04]"
    />
    <div v-else class="flex aspect-[2/3] w-full items-center justify-center bg-slate-800 text-5xl text-white/35">
      🎬
    </div>

    <div class="absolute inset-x-0 bottom-0 p-4">
      <div class="mb-2 inline-flex rounded-full bg-white/12 px-3 py-1 text-[10px] uppercase tracking-[0.28em] text-white/70">
        {{ item.type }}
      </div>
      <div class="text-base font-semibold text-white line-clamp-2">{{ item.name }}</div>
      <div class="mt-2 text-xs text-white/55">
        {{ item.episodes?.length ? `${item.episodes.length} 集可播` : '片库条目' }}
      </div>
    </div>
  </button>
</template>
