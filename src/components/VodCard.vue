<script setup lang="ts">
import { ref, watch } from 'vue'
import type { CatalogCard, VodItem } from '@/types'
import { getDoubanImageUrl } from '@/utils/douban'

const props = defineProps<{
  item: CatalogCard | VodItem
}>()

defineEmits<{
  click: [item: CatalogCard | VodItem]
}>()

const imageUrl = ref('')
watch(() => props.item.poster, async (newPoster) => {
  imageUrl.value = await getDoubanImageUrl(newPoster)
}, { immediate: true })

function itemTitle(item: CatalogCard | VodItem) {
  return 'title' in item ? item.title : item.name
}

function itemEpisodeMeta(item: CatalogCard | VodItem) {
  return 'episodes' in item && item.episodes?.length ? `${item.episodes.length} 集可播` : ''
}

function itemRating(item: CatalogCard | VodItem): number | null {
  return 'rating' in item ? (item as any).rating : null
}
</script>

<template>
  <button
    class="group relative w-full overflow-hidden rounded-[1.75rem] text-left poster-shadow transition duration-500 hover:-translate-y-1"
    @click="$emit('click', item)"
  >
    <div class="absolute inset-0 bg-gradient-to-t from-black via-black/25 to-transparent opacity-90"></div>
    <img
      v-if="imageUrl"
      :src="imageUrl"
      :alt="itemTitle(item)"
      class="h-full w-full aspect-[2/3] object-cover transition duration-700 group-hover:scale-[1.04]"
    />
    <div v-else class="flex aspect-[2/3] w-full items-center justify-center bg-slate-800 text-5xl text-white/35">
      🎬
    </div>

    <div class="absolute inset-x-0 bottom-0 p-4">
      <div class="text-base font-semibold text-white line-clamp-2">{{ itemTitle(item) }}</div>
      <div v-if="itemEpisodeMeta(item)" class="mt-2 text-xs text-white/55">
        {{ itemEpisodeMeta(item) }}
      </div>
      <div v-if="itemRating(item)" class="absolute bottom-4 right-4 text-sm font-medium text-yellow-400">
        ⭐ {{ itemRating(item) }}
      </div>
    </div>
  </button>
</template>
