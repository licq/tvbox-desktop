<script setup lang="ts">
import type { EpisodeAvailabilityState } from '@/types'

const props = withDefaults(
  defineProps<{
    label: string
    state?: EpisodeAvailabilityState
    sourceCount?: number
  }>(),
  {
    state: 'playable'
  }
)
</script>

<template>
  <button
    :class="['episode-chip', `episode-chip-${props.state}`]"
    :disabled="props.state === 'unavailable'"
    :aria-disabled="props.state === 'unavailable'"
    type="button"
  >
    <span class="episode-chip-label">{{ label }}</span>
    <span v-if="sourceCount && sourceCount > 1" class="episode-chip-badge">
      {{ sourceCount }}源
    </span>
  </button>
</template>

<style scoped>
.episode-chip-badge {
  font-size: 0.6rem;
  background: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.05rem 0.3rem;
  border-radius: 0.2rem;
  margin-left: 0.25rem;
}
</style>
