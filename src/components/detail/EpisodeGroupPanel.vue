<script setup lang="ts">
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisode, CatalogEpisodeGroup } from '@/types'

defineProps<{
  group: CatalogEpisodeGroup
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()
</script>

<template>
  <section class="source-group-panel">
    <div class="source-group-header">
      <span class="section-title">{{ group.source_name }}</span>
      <SourceBadge :label="`${group.episodes.length} 个播放源`" tone="neutral" />
    </div>

    <div class="episode-chip-grid">
      <EpisodeChip
        v-for="episode in group.episodes"
        :key="episode.id"
        :label="episode.episode_label"
        state="playable"
        @click="emit('play', episode)"
      />
    </div>
  </section>
</template>

<style scoped>
.source-group-panel {
  padding: 1rem 1.25rem;
  background: rgba(255,255,255,0.05);
  border-radius: 1.25rem;
}
.source-group-header {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  margin-bottom: 1rem;
}
.episode-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}
</style>
