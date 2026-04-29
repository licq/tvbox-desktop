<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { CatalogEpisode, CatalogItemType } from '@/types'
import EpisodeGrid from './EpisodeGrid.vue'

interface Source {
  source: string
  source_name: string
  detail_url: string
}

interface ProviderDetailResult {
  title: string | null
  poster: string | null
  summary: string | null
  episodes: CatalogEpisode[]
}

const props = defineProps<{
  title: string
  poster?: string
  itemType: CatalogItemType
  sources: Source[]
  sourceDetails?: Record<string, ProviderDetailResult>
  loadingSources?: string[]
}>()

const emit = defineEmits<{
  'play-episode': [episode: CatalogEpisode, sourceKey: string]
  'play-source': [source: string, detail_url: string]
  'select-source': [sourceKey: string]
}>()

const isMovie = computed(() => props.itemType === 'movie')

const selectedSourceKey = ref(props.sources[0]?.source ?? '')

watch(
  () => props.sources.map(s => s.source).join(','),
  () => {
    if (props.sources.length > 0 && !props.sources.some(s => s.source === selectedSourceKey.value)) {
      selectedSourceKey.value = props.sources[0].source
    }
  },
)

const currentDetail = computed(() => {
  if (!selectedSourceKey.value) return undefined
  return props.sourceDetails?.[selectedSourceKey.value]
})

const currentEpisodes = computed(() => currentDetail.value?.episodes ?? [])

const isLoadingCurrent = computed(() => {
  return props.loadingSources?.includes(selectedSourceKey.value) ?? false
})

function onSelectSource(sourceKey: string) {
  selectedSourceKey.value = sourceKey
  emit('select-source', sourceKey)
}

const typeLabel = computed(() => {
  switch (props.itemType) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
})

const isLoadingAnyMovieSource = computed(() => {
  if (!isMovie.value) return false
  return props.sources.some(s =>
    props.loadingSources?.includes(s.source)
  )
})

interface MovieEpisodeButton {
  key: string
  source: string
  episode: CatalogEpisode
  label: string
}

const movieEpisodeButtons = computed<MovieEpisodeButton[]>(() => {
  if (!isMovie.value) return []
  const buttons: MovieEpisodeButton[] = []
  for (const src of props.sources) {
    const detail = props.sourceDetails?.[src.source]
    if (!detail) continue
    for (const ep of detail.episodes) {
      const label = formatEpisodeLabel(src.source_name, ep.episode_label)
      buttons.push({
        key: `${src.source}-${ep.id}`,
        source: src.source,
        episode: ep,
        label,
      })
    }
  }
  return buttons
})

function extractShortName(name: string): string {
  // For CJK names, take first 2 characters (typically enough to identify)
  // For ASCII names, take first 4 characters
  const cjkMatch = name.match(/^[\u4e00-\u9fa5]{2,}/)
  if (cjkMatch) {
    return cjkMatch[0].slice(0, 2)
  }
  return name.slice(0, 4)
}

function formatEpisodeLabel(sourceName: string, episodeLabel: string): string {
  const short = extractShortName(sourceName)

  // If episodeLabel already contains the short name, don't prefix (avoid repetition)
  if (short && episodeLabel.includes(short)) {
    return episodeLabel
  }

  return `${short} · ${episodeLabel}`
}
</script>

<template>
  <div class="search-result-card">
    <div class="card-left">
      <img v-if="poster" :src="poster" class="card-poster" />
      <div v-else class="card-poster card-poster-placeholder" />
      <div class="card-info">
        <div class="card-title-row">
          <span class="card-title">{{ title }}</span>
          <span class="card-type-tag">{{ typeLabel }}</span>
        </div>
        <div class="card-meta">{{ sources.length }} 个播放源</div>
      </div>
    </div>
    <div class="card-right">
      <template v-if="isMovie">
        <div class="source-action-area">
          <div v-if="isLoadingAnyMovieSource" class="loading-placeholder">
            加载中…
          </div>
          <div v-else class="source-selector-row">
            <button
              v-for="btn in movieEpisodeButtons"
              :key="btn.key"
              type="button"
              class="source-btn"
              @click="emit('play-episode', btn.episode, btn.source)"
            >
              {{ btn.label }}
            </button>
          </div>
          <div
            v-if="!isLoadingAnyMovieSource && movieEpisodeButtons.length === 0"
            class="load-episodes-btn"
          >
            暂无播放链接
          </div>
        </div>
      </template>
      <template v-else>
        <div class="source-action-area">
          <div class="source-selector-row">
            <button
              v-for="src in sources"
              :key="src.source"
              type="button"
              :class="['source-btn', { active: selectedSourceKey === src.source }]"
              @click="onSelectSource(src.source)"
            >
              {{ src.source_name }}
            </button>
          </div>

          <EpisodeGrid
            v-if="currentEpisodes.length > 0"
            :episodes="currentEpisodes"
            @play="(ep) => emit('play-episode', ep, selectedSourceKey)"
          />
          <button
            v-else-if="isLoadingCurrent"
            type="button"
            class="loading-placeholder"
            disabled
          >
            加载中...
          </button>
          <button
            v-else
            type="button"
            class="load-episodes-btn"
            @click="emit('select-source', selectedSourceKey)"
          >
            点击加载集数
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.search-result-card {
  border-radius: 0.8rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.75rem 1rem;
  display: flex;
  gap: 0.75rem;
  align-items: stretch;
  transition: transform 200ms ease, border-color 200ms ease;
}
.search-result-card:hover {
  transform: translateY(-1px);
  border-color: rgba(255, 255, 255, 0.12);
}
.card-left {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  flex: 0 0 auto;
  width: 240px;
  min-width: 0;
}
.card-poster {
  width: 3.2rem;
  height: 4.8rem;
  object-fit: cover;
  border-radius: 0.4rem;
  flex-shrink: 0;
}
.card-poster-placeholder {
  background: rgba(255, 255, 255, 0.06);
}
.card-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 0.25rem;
}
.card-title-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.card-title {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.9);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.card-type-tag {
  font-size: 0.6rem;
  color: rgba(255, 255, 255, 0.35);
  background: rgba(255, 255, 255, 0.06);
  padding: 0.1rem 0.35rem;
  border-radius: 0.25rem;
  white-space: nowrap;
}
.card-meta {
  font-size: 0.7rem;
  color: rgba(255, 255, 255, 0.4);
}
.card-right {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.3rem;
  flex-wrap: wrap;
}
.source-action-area {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: flex-end;
}
.source-selector-row {
  display: flex;
  gap: 0.3rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.source-btn {
  border-radius: 0.35rem;
  padding: 0.25rem 0.5rem;
  font-size: 0.65rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.5);
  cursor: pointer;
  transition: all 180ms ease;
}
.source-btn.active,
.source-btn:hover {
  background: rgba(160, 120, 200, 0.12);
  border-color: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.85);
}
.load-episodes-btn,
.loading-placeholder {
  border-radius: 0.45rem;
  padding: 0.45rem 0.9rem;
  font-size: 0.78rem;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.55);
  cursor: pointer;
  transition: all 180ms ease;
}
.load-episodes-btn:hover {
  background: rgba(255, 255, 255, 0.08);
  color: rgba(255, 255, 255, 0.8);
}
.loading-placeholder {
  cursor: not-allowed;
  opacity: 0.5;
}

@media (max-width: 768px) {
  .search-result-card {
    flex-direction: column;
    align-items: stretch;
  }
  .card-left {
    width: auto;
  }
  .card-right {
    justify-content: flex-start;
  }
  .source-action-area {
    align-items: flex-start;
  }
  .source-selector-row {
    justify-content: flex-start;
  }
}
</style>
