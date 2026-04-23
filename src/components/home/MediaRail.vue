<script setup lang="ts">
import MediaCard from '@/components/media/MediaCard.vue'
import type { CatalogCard } from '@/types'

defineProps<{
  title: string
  items: CatalogCard[]
  summary?: string
}>()

defineEmits<{
  select: [item: CatalogCard]
}>()
</script>

<template>
  <section v-if="items.length" class="media-rail">
    <div class="media-rail-header">
      <div>
        <div class="section-title">{{ title }}</div>
        <p v-if="summary">{{ summary }}</p>
      </div>
      <span>{{ items.length }} items</span>
    </div>

    <div class="media-rail-track">
      <button
        v-for="item in items"
        :key="item.id"
        class="media-rail-card"
        type="button"
        @click="$emit('select', item)"
      >
        <MediaCard
          :title="item.title"
          :poster="item.poster"
          :subtitle="item.update_badge || item.source_badge || '片库条目'"
          :source-badge="item.source_badge"
        />
      </button>
    </div>
  </section>
</template>
