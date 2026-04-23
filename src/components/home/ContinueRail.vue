<script setup lang="ts">
import MediaCard from '@/components/media/MediaCard.vue'
import type { CatalogCard } from '@/types'

defineProps<{
  items: CatalogCard[]
}>()

defineEmits<{
  select: [item: CatalogCard]
}>()

function formatProgress(progress?: number) {
  return Math.max(0, Math.min(100, Math.round(progress ?? 0)))
}
</script>

<template>
  <section v-if="items.length" class="media-rail continue-rail">
    <div class="media-rail-header">
      <div>
        <div class="section-title">继续观看</div>
        <p>从播放历史接回，不需要先回到目录里找。</p>
      </div>
    </div>

    <div class="media-rail-track">
      <button
        v-for="item in items"
        :key="item.id"
        class="media-rail-card continue-rail-card"
        type="button"
        @click="$emit('select', item)"
      >
        <MediaCard
          :title="item.title"
          :poster="item.poster"
          :subtitle="item.update_badge || `${formatProgress(item.progress)}% watched`"
          :source-badge="item.source_badge"
        />
        <span v-if="item.progress !== undefined" class="continue-rail-progress">
          <span :style="{ width: `${formatProgress(item.progress)}%` }"></span>
        </span>
      </button>
    </div>
  </section>
</template>
