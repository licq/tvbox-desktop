<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'

type PlayerSource = {
  url: string
  label: string
  kind: 'hls' | 'http' | 'external' | 'embed'
}

defineProps<{
  sources: PlayerSource[]
  currentIndex: number
  failedIndexes: number[]
  status: string
  errorMessage?: string | null
}>()

defineEmits<{
  select: [index: number]
}>()

function sourceTone(kind: PlayerSource['kind']) {
  if (kind === 'external' || kind === 'embed') return 'danger'
  if (kind === 'hls') return 'cool'
  return 'neutral'
}
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

    <p class="playback-drawer-copy">
      当前线路、失败线路和不能内置播放的线路集中展示；自动切换失败线路时不会遮挡画面。
    </p>

    <div v-if="sources.length" class="playback-source-list">
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

    <div v-else class="playback-empty">
      没有解析出可展示线路。
    </div>

    <div class="playback-current-url">
      <div class="eyebrow">Current Url</div>
      <p>{{ sources[currentIndex]?.url || errorMessage || '当前没有可用地址' }}</p>
    </div>
  </aside>
</template>
