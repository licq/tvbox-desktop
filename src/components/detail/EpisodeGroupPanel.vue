<script setup lang="ts">
import { computed, ref } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisode, CatalogEpisodeGroup, CatalogItemType } from '@/types'

const COLLAPSE_THRESHOLD = 24

const props = defineProps<{
  group: CatalogEpisodeGroup
  item_type?: CatalogItemType
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()

const isMovie = computed(() => props.item_type === 'movie')
const expanded = ref(false)

const displayEpisodes = computed(() => {
  if (expanded.value || props.group.episodes.length <= COLLAPSE_THRESHOLD) {
    return props.group.episodes
  }
  return props.group.episodes.slice(0, COLLAPSE_THRESHOLD)
})
</script>

<template>
  <section class="source-group-card">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <span class="source-group-name">{{ group.source_name }}</span>
        <SourceBadge
          :label="isMovie ? `${group.episodes.length} 个版本` : `${group.episodes.length} 集`"
          tone="warm"
        />
      </div>
      <span class="source-group-type-tag">{{ isMovie ? '电影' : '剧集' }}</span>
    </div>

    <div class="source-group-body">
      <!-- Movie mode: version buttons -->
      <div v-if="isMovie" class="version-button-row">
        <button
          v-for="episode in group.episodes"
          :key="episode.id"
          class="version-button"
          @click="emit('play', episode)"
        >
          ▶ {{ episode.episode_label }}
        </button>
      </div>

      <!-- Series mode: episode chips grid -->
      <div v-else>
        <div class="episode-chip-grid">
          <EpisodeChip
            v-for="episode in displayEpisodes"
            :key="episode.id"
            :label="episode.episode_label"
            state="playable"
            @click="emit('play', episode)"
          />
        </div>
        <button
          v-if="group.episodes.length > COLLAPSE_THRESHOLD && !expanded"
          class="expand-button"
          @click="expanded = true"
        >
          展开全部 ({{ group.episodes.length }}集)
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.7rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.7rem 1rem;
}
.version-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.version-button {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 0.5rem;
  padding: 0.35rem 0.7rem;
  font-size: 0.72rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.version-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.episode-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.expand-button {
  margin-top: 0.5rem;
  border-radius: 0.5rem;
  padding: 0.35rem 0.8rem;
  font-size: 0.72rem;
  background: rgba(216, 154, 87, 0.1);
  border: 1px solid rgba(216, 154, 87, 0.2);
  color: rgba(240, 179, 107, 0.9);
  cursor: pointer;
  transition: all 180ms ease;
}
.expand-button:hover {
  background: rgba(216, 154, 87, 0.18);
}
</style>