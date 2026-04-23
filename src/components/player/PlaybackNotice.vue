<script setup lang="ts">
defineProps<{
  message: string
  tone?: 'info' | 'warning' | 'danger'
}>()
</script>

<template>
  <div :class="['playback-notice', `playback-notice-${tone ?? 'info'}`]" role="status">
    <span class="playback-notice-dot" aria-hidden="true"></span>
    <span>{{ message }}</span>
  </div>
</template>
