<script setup lang="ts">
import type { LiveChannel } from '@/types'

defineProps<{
  channel: LiveChannel
}>()

defineEmits<{
  play: [channel: LiveChannel]
}>()
</script>

<template>
  <div
    class="channel-card bg-gray-800 rounded-lg p-4 cursor-pointer hover:bg-gray-700 transition-colors"
    @click="$emit('play', channel)"
  >
    <img
      v-if="channel.logo"
      :src="channel.logo"
      :alt="channel.name"
      class="w-16 h-16 mx-auto mb-2 object-contain"
    />
    <div v-else class="w-16 h-16 mx-auto mb-2 bg-gray-600 rounded-full flex items-center justify-center text-2xl">
      📺
    </div>
    <div class="text-center text-sm truncate">{{ channel.name }}</div>
    <div v-if="channel.category" class="text-center text-xs text-gray-400 mt-1">
      {{ channel.category }}
    </div>
  </div>
</template>
