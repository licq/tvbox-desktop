<script setup lang="ts">
import { computed } from 'vue'

interface Source {
  source: string
  source_name: string
  detail_url: string
}

const props = defineProps<{
  sources: Source[]
}>()

const emit = defineEmits<{
  play: [source: string, detail_url: string]
}>()

const primarySource = computed(() => props.sources[0])
const extraSources = computed(() => props.sources.slice(1, 3))
const hasMore = computed(() => props.sources.length > 3)
const moreCount = computed(() => props.sources.length - 3)
</script>

<template>
  <div class="movie-action-panel">
    <button
      v-if="primarySource"
      type="button"
      class="play-btn-primary"
      @click="emit('play', primarySource.source, primarySource.detail_url)"
    >
      <span class="play-icon">▶</span>
      <span>立即播放</span>
    </button>
    <button
      v-for="src in extraSources"
      :key="src.detail_url"
      type="button"
      class="play-btn-secondary"
      @click="emit('play', src.source, src.detail_url)"
    >
      {{ src.source_name }}
    </button>
    <button
      v-if="hasMore"
      type="button"
      class="play-btn-secondary play-btn-more"
    >
      +{{ moreCount }}
    </button>
  </div>
</template>

<style scoped>
.movie-action-panel {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  flex-wrap: wrap;
}
.play-btn-primary {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 0.45rem;
  padding: 0.45rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 500;
  background: rgba(117, 169, 195, 0.15);
  color: rgba(200, 230, 245, 0.95);
  border: 1px solid rgba(117, 169, 195, 0.25);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-btn-primary:hover {
  background: rgba(117, 169, 195, 0.25);
  border-color: rgba(117, 169, 195, 0.4);
}
.play-btn-secondary {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  border-radius: 0.45rem;
  padding: 0.45rem 0.7rem;
  font-size: 0.72rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-btn-secondary:hover {
  background: rgba(255, 255, 255, 0.08);
  border-color: rgba(255, 255, 255, 0.15);
  color: rgba(255, 255, 255, 0.8);
}
.play-icon {
  color: rgba(117, 169, 195, 0.7);
  font-size: 0.65rem;
}
.play-btn-primary:hover .play-icon {
  color: rgba(200, 230, 245, 0.95);
}
</style>
