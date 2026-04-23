<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogDetailItem } from '@/types'

defineProps<{
  item: CatalogDetailItem
  sourceCount: number
  episodeCount: number
}>()

defineEmits<{
  play: []
}>()
</script>

<template>
  <section class="detail-hero">
    <div class="detail-hero-poster">
      <img v-if="item.poster" :src="item.poster" :alt="item.title" class="detail-hero-image" />
      <div v-else class="detail-hero-fallback">No Poster</div>
    </div>

    <div class="detail-hero-copy">
      <SourceBadge :label="item.item_type" tone="warm" />
      <h1 class="detail-hero-title">{{ item.title }}</h1>
      <p v-if="item.summary" class="detail-hero-summary">{{ item.summary }}</p>
      <p v-else class="detail-hero-summary">暂无简介。优先从可播线路进入，减少无效尝试。</p>

      <div class="detail-hero-actions">
        <button class="action-button action-button-primary" type="button" :disabled="episodeCount === 0" @click="$emit('play')">
          立即播放
        </button>
        <span class="detail-hero-meta">{{ sourceCount }} 个来源 · {{ episodeCount }} 个入口</span>
      </div>
    </div>
  </section>
</template>
