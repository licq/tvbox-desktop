<script setup lang="ts">
interface DoubanMeta {
  doubanId: number
  title: string
  rating: number | null
  ratingCount: number | null
  director: string[]
  writer: string[]
  actors: string[]
  genre: string[]
  country: string[]
  language: string[]
  releaseDate: string[]
  runtime: string | null
  summary: string | null
  poster: string | null
}

defineProps<{
  meta: DoubanMeta
  poster?: string  // from catalog_items
}>()

function formatList(list: string[], max: number): string {
  if (list.length <= max) return list.join(' / ')
  return list.slice(0, max).join(' / ') + '...'
}
</script>

<template>
  <section class="douban-meta-panel">
    <div class="douban-meta-poster">
      <img v-if="poster" :src="poster" :alt="meta.title" class="poster-img" />
      <img v-else-if="meta.poster" :src="meta.poster" :alt="meta.title" class="poster-img" />
      <div v-else class="poster-fallback">{{ meta.title }}</div>
    </div>

    <div class="douban-meta-content">
      <h1 class="douban-meta-title">{{ meta.title }}</h1>

      <div v-if="meta.rating" class="douban-meta-rating">
        <span class="rating-star">★</span>
        <span class="rating-num">{{ meta.rating.toFixed(1) }}</span>
        <span v-if="meta.ratingCount" class="rating-count">
          ({{ (meta.ratingCount / 10000).toFixed(1) }}万人评价)
        </span>
      </div>

      <dl class="douban-meta-list">
        <template v-if="meta.director.length">
          <dt>导演</dt>
          <dd>{{ meta.director.join(' / ') }}</dd>
        </template>
        <template v-if="meta.writer.length">
          <dt>编剧</dt>
          <dd>{{ meta.writer.join(' / ') }}</dd>
        </template>
        <template v-if="meta.actors.length">
          <dt>主演</dt>
          <dd>{{ formatList(meta.actors, 5) }}</dd>
        </template>
        <template v-if="meta.genre.length">
          <dt>类型</dt>
          <dd>{{ meta.genre.join(' / ') }}</dd>
        </template>
        <template v-if="meta.country.length">
          <dt>制片国家/地区</dt>
          <dd>{{ meta.country.join(' / ') }}</dd>
        </template>
        <template v-if="meta.language.length">
          <dt>语言</dt>
          <dd>{{ meta.language.join(' / ') }}</dd>
        </template>
        <template v-if="meta.releaseDate.length">
          <dt>上映日期</dt>
          <dd>{{ meta.releaseDate.join(' / ') }}</dd>
        </template>
        <template v-if="meta.runtime">
          <dt>片长</dt>
          <dd>{{ meta.runtime }}</dd>
        </template>
      </dl>

      <div v-if="meta.summary" class="douban-meta-summary">
        <p>{{ meta.summary }}</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.douban-meta-panel {
  display: flex;
  gap: 2rem;
  padding: 1.5rem;
  background: rgba(255,255,255,0.05);
  border-radius: 1.5rem;
}
.douban-meta-poster {
  flex-shrink: 0;
  width: 180px;
}
.poster-img {
  width: 180px;
  height: 260px;
  object-fit: cover;
  border-radius: 0.75rem;
}
.poster-fallback {
  width: 180px;
  height: 260px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255,255,255,0.1);
  border-radius: 0.75rem;
  font-size: 0.875rem;
  color: rgba(255,255,255,0.5);
}
.douban-meta-content {
  flex: 1;
  min-width: 0;
}
.douban-meta-title {
  font-size: 1.75rem;
  font-weight: 700;
  color: white;
  margin-bottom: 0.5rem;
}
.douban-meta-rating {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1rem;
}
.rating-star {
  color: #ffb800;
  font-size: 1.25rem;
}
.rating-num {
  color: #ffb800;
  font-size: 1.25rem;
  font-weight: 700;
}
.rating-count {
  color: rgba(255,255,255,0.5);
  font-size: 0.875rem;
}
.douban-meta-list {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 0.25rem 1rem;
  margin-bottom: 1rem;
}
.douban-meta-list dt {
  color: rgba(255,255,255,0.4);
  font-size: 0.875rem;
  white-space: nowrap;
}
.douban-meta-list dd {
  color: rgba(255,255,255,0.85);
  font-size: 0.875rem;
}
.douban-meta-summary {
  margin-top: 1rem;
  padding-top: 1rem;
  border-top: 1px solid rgba(255,255,255,0.1);
}
.douban-meta-summary p {
  color: rgba(255,255,255,0.7);
  font-size: 0.9rem;
  line-height: 1.6;
}
</style>
