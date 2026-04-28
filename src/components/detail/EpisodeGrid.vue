<script setup lang="ts">
import { computed, ref } from 'vue'
import type { CatalogEpisode } from '@/types'

const props = withDefaults(defineProps<{
  episodes: CatalogEpisode[]
  visibleCount?: number
}>(), {
  visibleCount: 12,
})

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()

const expanded = ref(false)

const visibleEpisodes = computed(() => {
  if (expanded.value) return props.episodes
  return props.episodes.slice(0, props.visibleCount)
})

const hasMore = computed(() => props.episodes.length > props.visibleCount)
</script>

<template>
  <div class="episode-grid">
    <button
      v-for="ep in visibleEpisodes"
      :key="ep.id"
      type="button"
      class="episode-chip"
      @click="emit('play', ep)"
    >
      {{ ep.episode_label }}
    </button>
    <button
      v-if="hasMore && !expanded"
      type="button"
      class="episode-chip episode-chip-more"
      @click="expanded = true"
    >
      ⋯
    </button>
    <button
      v-if="expanded && hasMore"
      type="button"
      class="episode-chip episode-chip-collapse"
      @click="expanded = false"
    >
      收起
    </button>
  </div>
</template>

<style scoped>
.episode-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.3rem;
}
.episode-chip {
  min-width: 2.4rem;
  height: 1.8rem;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 0.4rem;
  padding: 0 0.4rem;
  font-size: 0.65rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.7);
  cursor: pointer;
  transition: all 180ms ease;
}
.episode-chip:hover {
  background: rgba(160, 120, 200, 0.12);
  border-color: rgba(160, 120, 200, 0.25);
  color: rgba(220, 200, 245, 0.9);
}
.episode-chip-more,
.episode-chip-collapse {
  background: rgba(255, 255, 255, 0.06);
  color: rgba(255, 255, 255, 0.35);
}
</style>
