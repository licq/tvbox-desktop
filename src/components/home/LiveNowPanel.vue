<script setup lang="ts">
import type { LiveChannel, LiveChannelGroup } from '@/types'

const props = defineProps<{
  groups?: LiveChannelGroup[]
  channels?: LiveChannel[]
}>()

defineEmits<{
  play: [channel: LiveChannel, sourceUrl?: string]
}>()

function visibleChannels() {
  if (props.channels?.length) return props.channels.slice(0, 12)

  return (props.groups ?? [])
    .flatMap(group => group.channels)
    .slice(0, 12)
}
</script>

<template>
  <section class="live-now-panel">
    <div class="media-rail-header">
      <div>
        <div class="section-title">Live Now</div>
        <p>把可播频道前置到首页，直播仍然一键进播放器。</p>
      </div>
      <span>{{ groups?.length ?? 0 }} groups</span>
    </div>

    <div v-if="visibleChannels().length" class="live-now-grid">
      <button
        v-for="channel in visibleChannels()"
        :key="channel.id"
        class="live-now-card"
        type="button"
        @click="$emit('play', channel, channel.sources[0]?.url)"
      >
        <img v-if="channel.logo" :src="channel.logo" :alt="channel.name" />
        <span v-else class="live-now-logo-fallback">TV</span>
        <span>
          <strong>{{ channel.name }}</strong>
          <small>{{ channel.category }} · {{ channel.sources.length }} 路</small>
        </span>
      </button>
    </div>

    <div v-else class="home-empty-state">暂无直播频道，先去订阅页检查源状态。</div>
  </section>
</template>
