<script setup lang="ts">
defineProps<{
  count?: number  // 显示几个骨架项，默认 6
}>()
</script>

<template>
  <div class="episode-group-skeleton space-y-4">
    <div class="skeleton-header">
      <div class="skeleton-title"></div>
    </div>
    <div class="flex flex-wrap gap-2">
      <div
        v-for="i in (count ?? 6)"
        :key="i"
        class="skeleton-chip"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.skeleton-header {
  @apply mb-3;
}

.skeleton-title {
  @apply h-5 w-32 rounded bg-white/10 animate-pulse;
}

.skeleton-chip {
  @apply h-9 w-20 rounded-full bg-white/10 animate-pulse;
}
</style>
