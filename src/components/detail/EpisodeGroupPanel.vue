<script setup lang="ts">
import { computed, ref } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import { mergeEpisodes } from '@/utils/episode'
import type { CatalogEpisodeGroup, CatalogItemType, UnifiedEpisode } from '@/types'

const props = defineProps<{
  groups: CatalogEpisodeGroup[]
  item_type?: CatalogItemType
}>()

const emit = defineEmits<{
  play: [episode: UnifiedEpisode]
}>()

const isMovie = computed(() => props.item_type === 'movie')

const typeLabel = computed(() => {
  switch (props.item_type) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
})

const unifiedEpisodes = computed(() => {
  const itemType = props.item_type ?? 'series'
  return mergeEpisodes(props.groups, itemType)
})

const EXPAND_THRESHOLD = 24
const needsExpand = computed(() => {
  return !isMovie.value && unifiedEpisodes.value.length > EXPAND_THRESHOLD
})
const expanded = ref(false)

const visibleEpisodes = computed(() => {
  if (isMovie.value || !needsExpand.value || expanded.value) {
    return unifiedEpisodes.value
  }
  return unifiedEpisodes.value.slice(0, EXPAND_THRESHOLD)
})

const remainingCount = computed(() => {
  return unifiedEpisodes.value.length - EXPAND_THRESHOLD
})
</script>

<template>
  <section class="source-group-card">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <span class="source-group-name">全部播放源</span>
        <span class="source-group-count-badge">
          {{ isMovie ? `${unifiedEpisodes.length} 个播放源` : `${unifiedEpisodes.length} 集` }}
        </span>
      </div>
      <span class="source-group-type-tag">{{ typeLabel }}</span>
    </div>

    <div class="source-group-body">
      <div v-if="isMovie" class="play-button-row">
        <button
          v-for="ue in unifiedEpisodes"
          :key="ue.sources[0].episode.id"
          class="play-button"
          @click="emit('play', ue)"
        >
          <span class="play-icon">▶</span>
          <span class="play-label">{{ ue.displayLabel }}</span>
        </button>
      </div>

      <template v-else>
        <div class="episode-chip-grid">
          <EpisodeChip
            v-for="ue in visibleEpisodes"
            :key="ue.normalizedIndex"
            :label="ue.displayLabel"
            :source-count="ue.sources.length"
            state="playable"
            @click="emit('play', ue)"
          />
        </div>

        <button
          v-if="needsExpand && !expanded"
          class="expand-toggle-button"
          @click="expanded = true"
        >
          <span>展开剩余 {{ remainingCount }} 集</span>
          <svg class="expand-chevron" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>

        <button
          v-else-if="needsExpand && expanded"
          class="expand-toggle-button"
          @click="expanded = false"
        >
          <span>收起</span>
          <svg class="expand-chevron expanded" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 10L8 6L12 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </template>
    </div>
  </section>
</template>

<style scoped>
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
  transition: transform 200ms ease, border-color 200ms ease, box-shadow 200ms ease;
}
.source-group-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 255, 255, 0.12);
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.18);
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-count-badge {
  font-size: 0.65rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.35);
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 0.25rem;
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.75rem 1rem;
}
.play-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.play-button {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 0.5rem;
  padding: 0.4rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.play-icon {
  color: rgba(117, 169, 195, 0.7);
  font-size: 0.65rem;
}
.play-button:hover .play-icon {
  color: rgba(200, 230, 245, 0.95);
}
.episode-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.expand-toggle-button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  width: 100%;
  margin-top: 0.6rem;
  padding: 0.5rem 0.8rem;
  border-radius: 0.6rem;
  font-size: 0.78rem;
  font-weight: 500;
  background: rgba(117, 169, 195, 0.06);
  border: 1px solid rgba(117, 169, 195, 0.15);
  color: rgba(200, 230, 245, 0.7);
  cursor: pointer;
  transition: all 180ms ease;
}
.expand-toggle-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.3);
  color: rgba(200, 230, 245, 0.95);
}
.expand-chevron {
  opacity: 0.6;
}
</style>
