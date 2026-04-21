<script setup lang="ts">
defineProps<{
  size?: 'sm' | 'md' | 'lg'
}>()
</script>

<template>
  <div class="loading-spinner flex items-center justify-center">
    <div
      :class="[
        'animate-spin rounded-full border-[3px] border-white/10 border-t-[#d89a57]',
        { 'h-4 w-4': size === 'sm', 'h-8 w-8': size === 'md', 'h-12 w-12': size === 'lg' || !size }
      ]"
    ></div>
  </div>
</template>
