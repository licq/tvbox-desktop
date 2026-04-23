<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'

defineProps<{
  title: string
  poster?: string
  subtitle?: string
  sourceBadge?: string
}>()
</script>

<template>
  <article class="media-card">
    <div class="media-card-poster">
      <img v-if="poster" :src="poster" :alt="title" class="media-card-image" />
      <div v-else class="media-card-fallback">
        <span>无海报</span>
      </div>
    </div>

    <div class="media-card-body">
      <SourceBadge v-if="sourceBadge" :label="sourceBadge" tone="cool" />
      <h3 class="media-card-title">{{ title }}</h3>
      <p v-if="subtitle" class="media-card-subtitle">{{ subtitle }}</p>
    </div>
  </article>
</template>
