<script setup lang="ts">
import { ref } from 'vue'
import SourceBadge from '@/components/media/SourceBadge.vue'

type PlayerSource = {
  url: string
  label: string
  kind: 'hls' | 'http' | 'external' | 'embed'
}

const props = defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
  episodes?: import('@/types').CatalogEpisode[]
  currentEpisodeId?: number
  activeTab?: 'sources' | 'episodes'
}>()

defineEmits<{
  select: [index: number]
  selectEpisode: [episode: import('@/types').CatalogEpisode]
  tabChange: [tab: 'sources' | 'episodes']
}>()

function sourceTone(kind: PlayerSource['kind']) {
  if (kind === 'external' || kind === 'embed') return 'danger'
  if (kind === 'hls') return 'cool'
  return 'neutral'
}

const innerTab = ref<'sources' | 'episodes'>(props.activeTab ?? 'sources')
</script>

<template>
  <aside class="playback-drawer">
    <div class="playback-drawer-header">
      <div>
        <div class="eyebrow">Source Drawer</div>
        <h2>播放线路</h2>
      </div>
      <SourceBadge :label="status" :tone="status === 'failed' ? 'danger' : 'warm'" />
    </div>

    <div class="drawer-tabs">
      <button
        :class="{ active: innerTab === 'sources' }"
        @click="innerTab = 'sources'; $emit('tabChange', 'sources')"
      >线路</button>
      <button
        :class="{ active: innerTab === 'episodes' }"
        @click="innerTab = 'episodes'; $emit('tabChange', 'episodes')"
      >选集</button>
    </div>

    <p class="playback-drawer-copy">
      当前线路、失败线路和不能内置播放的线路集中展示；自动切换失败线路时不会遮挡画面。
    </p>

    <!-- Source list — shown when on sources tab with items -->
    <div v-if="innerTab === 'sources' && sources.length" class="playback-source-list">
      <button
        v-for="(source, index) in sources"
        :key="`${source.url}-${index}`"
        :class="[
          'playback-source-row',
          index === currentIndex ? 'playback-source-row-active' : '',
          failedIndexes.includes(index) ? 'playback-source-row-failed' : ''
        ]"
        type="button"
        @click="$emit('select', index)"
      >
        <span>
          <small>Line {{ index + 1 }}</small>
          <strong>{{ source.label }}</strong>
        </span>
        <span class="playback-source-meta">
          <SourceBadge :label="source.kind" :tone="sourceTone(source.kind)" />
          <em v-if="failedIndexes.includes(index)">失败</em>
          <em v-else-if="index === currentIndex">当前</em>
        </span>
      </button>
    </div>

    <!-- Episodes grid — shown when on episodes tab with episodes -->
    <div v-else-if="innerTab === 'episodes' && episodes?.length" class="episode-grid">
      <button
        v-for="ep in episodes"
        :key="ep.id"
        :class="['episode-chip', ep.id === currentEpisodeId ? 'episode-chip-active' : '']"
        type="button"
        @click="$emit('selectEpisode', ep)"
      >
        {{ ep.episode_label }}
      </button>
    </div>

    <!-- Empty state: no sources on sources tab -->
    <div v-else-if="innerTab === 'sources'" class="playback-empty">
      没有解析出可展示线路。
    </div>

    <!-- Empty state: no episodes on episodes tab -->
    <div v-else-if="innerTab === 'episodes'" class="playback-empty">
      当前无可用选集
    </div>

    <div class="playback-current-url">
      <div class="eyebrow">Current Url</div>
      <p>{{ sources[currentIndex]?.url || errorMessage || '当前没有可用地址' }}</p>
    </div>
  </aside>
</template>

<style scoped>
.playback-empty {
  padding: 1rem;
  color: var(--text-muted);
  text-align: center;
  font-size: 0.875rem;
}

.drawer-tabs {
  display: flex;
  border-bottom: 1px solid var(--stroke);
  margin-bottom: 0.75rem;
}

.drawer-tabs button {
  flex: 1;
  padding: 0.5rem;
  font-size: 0.875rem;
  color: var(--text-muted);
  border-bottom: 2px solid transparent;
  background: none;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.drawer-tabs button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.episode-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 0.5rem;
  max-height: 280px;
  overflow-y: auto;
}

.episode-chip {
  padding: 0.5rem 0.25rem;
  font-size: 0.8125rem;
  border-radius: 0.375rem;
  background: var(--bg-elevated);
  border: 1px solid var(--stroke);
  color: var(--text);
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s;
}

.episode-chip:hover {
  border-color: var(--accent);
}

.episode-chip-active {
  background: var(--accent);
  color: #000;
  font-weight: 600;
  border-color: var(--accent);
}
</style>
