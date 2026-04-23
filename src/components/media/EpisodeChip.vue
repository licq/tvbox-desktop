<script setup lang="ts">
import type { EpisodeAvailabilityState } from '@/types'

const props = withDefaults(
  defineProps<{
    label: string
    state?: EpisodeAvailabilityState
  }>(),
  {
    state: 'playable'
  }
)
</script>

<template>
  <button :class="['episode-chip', `episode-chip-${props.state}`]" type="button">
    <span class="episode-chip-label">{{ label }}</span>
  </button>
</template>
