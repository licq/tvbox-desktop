<script setup lang="ts">
defineProps<{
  count?: number
}>()
</script>

<template>
  <div class="episode-group-skeleton">
    <div class="skeleton-header">
      <div class="skeleton-title"></div>
      <div class="skeleton-tag"></div>
    </div>
    <div class="skeleton-chips">
      <div
        v-for="i in (count ?? 8)"
        :key="i"
        class="skeleton-chip"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.episode-group-skeleton {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.75rem 1rem;
}
.skeleton-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.skeleton-title {
  height: 1.15rem;
  width: 6rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-tag {
  height: 0.75rem;
  width: 2rem;
  border-radius: 0.25rem;
  background: rgba(255, 255, 255, 0.04);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.skeleton-chip {
  height: 2rem;
  width: 3.5rem;
  border-radius: 0.5rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}
</style>
