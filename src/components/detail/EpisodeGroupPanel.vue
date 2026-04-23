<script setup lang="ts">
import { ref, watch } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisode, CatalogEpisodeGroup } from '@/types'

const props = defineProps<{
  group: CatalogEpisodeGroup
  recommended?: boolean
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()

const expanded = ref(false)

watch(
  () => props.recommended,
  (isRecommended) => {
    if (isRecommended) {
      expanded.value = true
    }
  },
  { immediate: true }
)
</script>

<template>
  <section :class="['episode-group-panel', recommended ? 'episode-group-panel-recommended' : '']">
    <button class="episode-group-header" type="button" @click="expanded = !expanded">
      <span>
        <span class="section-title">{{ group.source_name }}</span>
        <small>{{ recommended ? '推荐来源，默认展开' : '备用来源，按需展开' }}</small>
      </span>
      <span class="episode-group-meta">
        <SourceBadge :label="`${group.episodes.length} episodes`" :tone="recommended ? 'warm' : 'neutral'" />
        <span>{{ expanded ? '收起' : '展开' }}</span>
      </span>
    </button>

    <div v-if="expanded" class="episode-chip-grid">
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
