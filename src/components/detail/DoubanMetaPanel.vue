<script setup lang="ts">
import { computed, ref, watchEffect } from 'vue'
import { invoke } from '@tauri-apps/api/core'

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

const props = defineProps<{
  meta: DoubanMeta
  poster?: string
  loading?: boolean  // true when enriched metadata is being fetched
}>()

// Proxy Douban images through backend to avoid Referer blocking
const proxiedPoster = ref<string | null>(null)
const posterLoading = ref(false)

watchEffect(async () => {
  const url = props.meta.poster
  if (!url || !url.includes('doubanio.com')) {
    proxiedPoster.value = null
    return
  }
  posterLoading.value = true
  try {
    const b64 = await invoke<string>('proxy_image', { url })
    proxiedPoster.value = `data:image/jpeg;base64,${b64}`
  } catch {
    proxiedPoster.value = null
  } finally {
    posterLoading.value = false
  }
})

const effectivePoster = computed(() => {
  if (posterLoading.value) return null
  return proxiedPoster.value ?? props.poster ?? props.meta.poster ?? null
})

function formatList(list: string[], max: number): string {
  if (list.length <= max) return list.join(' / ')
  return list.slice(0, max).join(' / ') + '...'
}

// True when enriched metadata fields are still empty
const isEnriching = computed(() => {
  return props.loading && (
    props.meta.director.length === 0 &&
    props.meta.writer.length === 0 &&
    props.meta.actors.length === 0
  )
})
</script>

<template>
  <section class="douban-meta-panel">
    <div class="douban-meta-poster">
      <img v-if="effectivePoster" :src="effectivePoster" :alt="meta.title" class="poster-img" />
      <div v-else class="poster-fallback">
        <span v-if="posterLoading" class="poster-loading">加载中...</span>
        <span v-else>{{ meta.title }}</span>
      </div>
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

      <!-- Loading skeleton for content area -->
      <div v-if="isEnriching" class="meta-skeleton">
        <div class="skel-row skel-title"></div>
        <div class="skel-row skel-rating"></div>
        <div class="skel-row skel-line"></div>
        <div class="skel-row skel-line"></div>
        <div class="skel-row skel-line short"></div>
      </div>

      <dl v-else class="douban-meta-list">
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
    </div>

    <div v-if="meta.summary" class="douban-meta-summary">
      <h3>剧情简介</h3>
      <p>{{ meta.summary }}</p>
    </div>
  </section>
</template>

<style scoped>
.douban-meta-panel {
  display: grid;
  grid-template-columns: 220px 1fr 300px;
  gap: 1.5rem;
  padding: 1.5rem;
  background: rgba(255,255,255,0.05);
  border-radius: 1.5rem;
  min-height: 360px;
  align-items: start;
}
.douban-meta-poster {
  position: sticky;
  top: 0;
}
.poster-img {
  width: 220px;
  height: 320px;
  object-fit: cover;
  border-radius: 0.75rem;
}
.poster-fallback {
  width: 220px;
  height: 320px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255,255,255,0.1);
  border-radius: 0.75rem;
  font-size: 0.875rem;
  color: rgba(255,255,255,0.5);
  text-align: center;
  padding: 1rem;
}
.poster-loading {
  font-size: 0.75rem;
  color: rgba(255,255,255,0.3);
}
.douban-meta-content {
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
  border-left: 1px solid rgba(255,255,255,0.1);
  padding-left: 1.5rem;
}
.douban-meta-summary h3 {
  font-size: 1rem;
  color: rgba(255,255,255,0.5);
  margin-bottom: 0.75rem;
  font-weight: 500;
}
.douban-meta-summary p {
  color: rgba(255,255,255,0.7);
  font-size: 0.9rem;
  line-height: 1.6;
}

/* Loading skeleton */
.meta-skeleton {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}
.skel-row {
  background: rgba(255,255,255,0.06);
  border-radius: 0.375rem;
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}
.skel-title { height: 2rem; width: 70%; }
.skel-rating { height: 1.25rem; width: 40%; }
.skel-line { height: 1rem; width: 90%; }
.skel-line.short { width: 60%; }
@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}

@media (max-width: 1023px) {
  .douban-meta-panel {
    grid-template-columns: 160px 1fr;
  }
  .douban-meta-summary {
    grid-column: 1 / -1;
    border-left: none;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-left: 0;
    padding-top: 1rem;
  }
  .poster-img,
  .poster-fallback {
    width: 160px;
    height: 230px;
  }
}

@media (max-width: 639px) {
  .douban-meta-panel {
    grid-template-columns: 1fr;
    gap: 1rem;
  }
  .douban-meta-poster {
    position: static;
    width: 140px;
    margin: 0 auto;
  }
  .poster-img,
  .poster-fallback {
    width: 140px;
    height: 200px;
  }
  .douban-meta-title {
    font-size: 1.4rem;
    text-align: center;
  }
  .douban-meta-rating {
    justify-content: center;
  }
  .douban-meta-summary {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-top: 1rem;
  }
}
</style>
