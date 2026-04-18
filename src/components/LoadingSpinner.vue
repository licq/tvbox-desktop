<script setup lang="ts">
defineProps<{
  size?: 'sm' | 'md' | 'lg'
}>()
</script>

<template>
  <div class="loading-spinner flex items-center justify-center">
    <div
      :class="[
        'animate-spin rounded-full border-2 border-gray-600 border-t-primary',
        { 'w-4 h-4': size === 'sm', 'w-8 h-8': size === 'md', 'w-12 h-12': size === 'lg' || !size }
      ]"
    ></div>
  </div>
</template>
