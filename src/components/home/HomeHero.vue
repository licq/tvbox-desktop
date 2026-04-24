<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogCard, HeroMetric } from '@/types'

defineProps<{
  item: CatalogCard | null
  metrics: HeroMetric[]
  title: string
  summary?: string
}>()

defineEmits<{
  open: [item: CatalogCard]
}>()
</script>

<template>
  <section class="home-hero">
    <div
      class="home-hero-backdrop"
      :style="item?.poster ? { backgroundImage: `linear-gradient(90deg, rgba(8, 10, 16, 0.96), rgba(8, 10, 16, 0.62), rgba(8, 10, 16, 0.88)), url(${item.poster})` } : undefined"
    ></div>

    <div class="home-hero-copy">
      <div class="eyebrow">Media Center</div>
      <h1 class="home-hero-title">{{ title }}</h1>
      <p class="home-hero-summary">{{ summary }}</p>

      <button v-if="item" class="home-hero-feature" type="button" @click="$emit('open', item)">
        <span class="home-hero-feature-poster">
          <img v-if="item.poster" :src="item.poster" :alt="item.title" />
          <span v-else>无海报</span>
        </span>
        <span class="home-hero-feature-copy">
          <SourceBadge :label="item.source_badge || item.update_badge || 'Featured'" tone="warm" />
          <strong>{{ item.title }}</strong>
          <small>{{ item.update_badge || '打开推荐内容' }}</small>
        </span>
      </button>
    </div>

    <div class="home-hero-metrics">
      <div v-for="metric in metrics" :key="metric.label" class="home-hero-metric">
        <span>{{ metric.label }}</span>
        <strong>{{ metric.value }}</strong>
      </div>
    </div>
  </section>
</template>
