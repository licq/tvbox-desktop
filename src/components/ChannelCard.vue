<script setup lang="ts">
import { computed } from 'vue'
import type { LiveChannel } from '@/types'

const props = defineProps<{
  channel: LiveChannel
  sourceUrl?: string
}>()

const emit = defineEmits<{
  play: [channel: LiveChannel, sourceUrl?: string]
}>()

const sourceCount = computed(() => props.channel.sources.length)

function handleClick() {
  emit('play', props.channel, props.sourceUrl)
}
</script>

<template>
  <button
    class="surface-muted group flex w-full flex-col gap-4 rounded-[1.5rem] p-4 text-left transition duration-300 hover:-translate-y-1 hover:border-white/15 hover:bg-white/[0.07]"
    @click="handleClick"
  >
    <div class="flex items-start justify-between gap-3">
      <div class="flex items-center gap-3">
        <img
          v-if="channel.logo"
          :src="channel.logo"
          :alt="channel.name"
          class="h-12 w-12 rounded-2xl object-contain bg-white/5 p-2"
        />
        <div v-else class="flex h-12 w-12 items-center justify-center rounded-2xl bg-white/5 text-xl text-white/60">
          📺
        </div>
        <div>
          <div class="text-sm font-semibold text-white">{{ channel.name }}</div>
          <div class="mt-1 text-xs uppercase tracking-[0.22em] text-white/35">{{ channel.category }}</div>
        </div>
      </div>
      <div class="rounded-full bg-white/8 px-3 py-1 text-[10px] uppercase tracking-[0.24em] text-white/55">
        {{ sourceCount }} 路
      </div>
    </div>

    <div class="flex items-center justify-between text-xs text-white/48">
      <span>直播汇聚</span>
      <span class="translate-x-0 transition group-hover:translate-x-1">立即播放</span>
    </div>
  </button>
</template>
