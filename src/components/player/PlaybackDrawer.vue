<script setup lang="ts">
import { ref, computed } from 'vue'
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogItemType, PlayerSource, UnifiedEpisode } from '@/types'

const props = defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
  unifiedEpisodes?: UnifiedEpisode[]
  currentNormalizedIndex?: number
  itemType: CatalogItemType
}>()

const emit = defineEmits<{
  selectEpisode: [unifiedEpisode: UnifiedEpisode]
  switchLine: [index: number]
}>()

const copied = ref(false)

function sourceTone(kind: PlayerSource['kind']) {
  if (kind === 'external' || kind === 'embed') return 'danger'
  if (kind === 'hls') return 'cool'
  return 'neutral'
}

async function copyUrl(url: string) {
  try {
    await navigator.clipboard.writeText(url)
    copied.value = true
    setTimeout(() => { copied.value = false }, 2000)
  } catch {
    // Clipboard API not available — silently fail
  }
}

const isSeries = computed(() =>
  props.itemType === 'series' || props.itemType === 'variety' || props.itemType === 'anime'
)

const currentSource = computed(() => props.sources[props.currentIndex] ?? null)
</script>

<template>
  <aside class="playback-drawer">
    <!-- PlaybackHeader -->
    <div class="playback-header">
      <div class="playback-header-title">
        <template v-if="isSeries && currentNormalizedIndex !== undefined && unifiedEpisodes?.length">
          <span class="eyebrow">正在播放</span>
          <h2>{{ unifiedEpisodes.find(e => e.normalizedIndex === currentNormalizedIndex)?.displayLabel || '选集' }}</h2>
        </template>
        <template v-else>
          <span class="eyebrow">播放线路</span>
          <h2>{{ currentSource?.label || '选择线路' }}</h2>
        </template>
      </div>
      <SourceBadge :label="status" :tone="status === 'failed' ? 'danger' : 'warm'" />
    </div>

    <!-- EpisodeSection (scrollable) -->
    <div class="episode-section">
      <!-- Series mode: episode grid -->
      <div v-if="isSeries && unifiedEpisodes?.length" class="episode-grid">
        <button
          v-for="ue in unifiedEpisodes"
          :key="ue.normalizedIndex"
          :class="[
            'episode-chip',
            ue.normalizedIndex === currentNormalizedIndex ? 'episode-chip-active' : ''
          ]"
          type="button"
          @click="emit('selectEpisode', ue)"
        >
          {{ ue.displayLabel }}
          <span v-if="ue.sources.length > 1" class="source-count-badge">{{ ue.sources.length }}源</span>
        </button>
      </div>

      <!-- Movie mode: source list -->
      <div v-else-if="sources.length" class="source-list">
        <button
          v-for="(source, index) in sources"
          :key="`${source.url}-${index}`"
          :class="[
            'source-row',
            index === currentIndex ? 'source-row-active' : '',
            failedIndexes.includes(index) ? 'source-row-failed' : ''
          ]"
          type="button"
          @click="emit('switchLine', index)"
        >
          <span class="source-row-label">{{ source.label }}</span>
          <SourceBadge :label="source.kind" :tone="sourceTone(source.kind)" />
        </button>
      </div>

      <!-- Empty state -->
      <div v-else class="playback-empty">没有可用线路</div>
    </div>

    <!-- LinkInfoPanel (fixed bottom) -->
    <div class="link-info-panel">
      <!-- LineSwitcher -->
      <div v-if="sources.length > 1" class="line-switcher">
        <button
          v-for="(_, index) in sources"
          :key="index"
          :class="[
            'line-btn',
            index === currentIndex ? 'line-btn-active' : '',
            failedIndexes.includes(index) ? 'line-btn-failed' : ''
          ]"
          type="button"
          @click="emit('switchLine', index)"
        >
          线路{{ index + 1 }}
        </button>
      </div>

      <!-- UrlDisplay -->
      <div
        v-if="currentSource?.url"
        :class="['url-display', { 'url-display-copied': copied }]"
        @click="copyUrl(currentSource.url)"
        title="点击复制 URL"
      >
        <span class="url-text">{{ currentSource.url }}</span>
        <span class="url-copy-hint">{{ copied ? '✓ 已复制' : '复制' }}</span>
      </div>

      <!-- ErrorDisplay -->
      <div v-if="errorMessage" class="error-display">
        {{ errorMessage }}
      </div>
    </div>
  </aside>
</template>

<style scoped>
.playback-drawer {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.playback-empty {
  padding: 1rem;
  color: var(--text-muted);
  text-align: center;
  font-size: 0.875rem;
}

/* PlaybackHeader */
.playback-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  padding: 0.75rem;
  border-bottom: 1px solid var(--stroke);
}

.playback-header-title .eyebrow {
  font-size: 0.7rem;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 0.15rem;
}

.playback-header-title h2 {
  font-size: 0.95rem;
  font-weight: 600;
  margin: 0;
  line-height: 1.3;
}

/* EpisodeSection — scrollable */
.episode-section {
  flex: 1;
  overflow-y: auto;
  min-height: 0;
  padding: 0.75rem;
}

/* Episode grid (series/variety/anime) */
.episode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(48px, 1fr));
  gap: 0.35rem;
}

.episode-chip {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0.3rem 0.2rem;
  font-size: 0.72rem;
  border-radius: 0.45rem;
  min-height: 1.75rem;
  background: rgba(255, 255, 255, 0.045);
  border: 1px solid rgba(255, 255, 255, 0.08);
  color: rgba(246, 241, 232, 0.82);
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s, color 0.15s, transform 0.15s;
  position: relative;
  line-height: 1.2;
}

.episode-chip:hover {
  border-color: rgba(216, 154, 87, 0.45);
  background: rgba(255, 255, 255, 0.08);
  transform: translateY(-1px);
}

.episode-chip-active {
  background: rgba(216, 154, 87, 0.1);
  color: var(--accent);
  font-weight: 600;
  border-color: rgba(216, 154, 87, 0.35);
}

.source-count-badge {
  position: absolute;
  top: -3px;
  right: -3px;
  font-size: 0.55rem;
  background: rgba(160, 120, 200, 0.22);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.02rem 0.22rem;
  border-radius: 0.2rem;
  line-height: 1.3;
}

/* Source list (movie mode) */
.source-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.source-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  padding: 0.65rem 0.75rem;
  border-radius: 0.5rem;
  background: transparent;
  border: 1px solid var(--stroke);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  transition: background 0.15s, border-color 0.15s;
}

.source-row:hover {
  border-color: var(--accent);
}

.source-row-active {
  background: var(--accent);
  color: #000;
  border-color: var(--accent);
  font-weight: 600;
}

.source-row-failed {
  border-color: var(--danger);
}

.source-row-label {
  font-weight: 500;
  font-size: 0.875rem;
}

/* LinkInfoPanel — fixed bottom */
.link-info-panel {
  border-top: 1px solid var(--stroke);
  padding: 0.75rem;
  background: var(--bg-primary);
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* LineSwitcher */
.line-switcher {
  display: flex;
  gap: 0.35rem;
  flex-wrap: wrap;
}

.line-btn {
  flex: 1;
  min-width: 55px;
  padding: 0.3rem 0.5rem;
  border-radius: 0.375rem;
  border: 1px solid var(--stroke);
  background: transparent;
  color: var(--text);
  font-size: 0.75rem;
  cursor: pointer;
  text-align: center;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
}

.line-btn:hover {
  border-color: var(--accent);
}

.line-btn-active {
  background: rgba(216, 154, 87, 0.08);
  color: var(--accent);
  font-weight: 500;
  border-color: rgba(216, 154, 87, 0.28);
}

.line-btn-failed {
  border-color: var(--danger);
  color: var(--danger);
}

/* UrlDisplay */
.url-display {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.35rem;
  padding: 0.35rem 0.5rem;
  border-radius: 0.25rem;
  border: 1px solid var(--stroke);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
  font-size: 0.7rem;
  font-family: monospace;
}

.url-display:hover {
  border-color: var(--accent);
  background: rgba(255, 255, 255, 0.03);
}

.url-display-copied {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}

.url-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-secondary);
  min-width: 0;
}

.url-copy-hint {
  flex-shrink: 0;
  font-size: 0.65rem;
  color: var(--text-muted);
  white-space: nowrap;
}

.url-display-copied .url-copy-hint {
  color: var(--accent);
}

/* ErrorDisplay */
.error-display {
  font-size: 0.75rem;
  color: var(--danger);
  padding: 0.4rem 0.5rem;
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  border-radius: 0.25rem;
  line-height: 1.4;
}
</style>
