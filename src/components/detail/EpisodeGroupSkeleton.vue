<script setup lang="ts">
defineProps<{
  count?: number
}>()
</script>

<template>
  <section class="source-group-card episode-group-skeleton" aria-busy="true">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <div class="skeleton-title"></div>
        <div class="skeleton-badge"></div>
      </div>
      <div class="skeleton-tag"></div>
    </div>
    <div class="source-group-body">
      <div class="episode-chip-grid skeleton-chips">
        <div
          v-for="i in (count ?? 8)"
          :key="i"
          class="skeleton-chip"
        ></div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.episode-group-skeleton {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
}
.skeleton-title {
  height: 1.15rem;
  width: 6rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-body {
  padding: 0.75rem 1rem;
}
.skeleton-badge {
  height: 0.75rem;
  width: 2.8rem;
  border-radius: 0.25rem;
  background: rgba(255, 255, 255, 0.04);
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
  height: 2.75rem;
  width: clamp(3.25rem, 8vw, 5rem);
  border-radius: 1.1rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}
</style>
