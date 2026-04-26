<template>
  <section class="douban-meta-panel">
    <div class="douban-meta-poster">
      <div class="skeleton-pulse skeleton-poster"></div>
    </div>
    <div class="douban-meta-content">
      <div class="skeleton-pulse skeleton-title"></div>
      <div class="skeleton-pulse skeleton-rating"></div>
      <div class="skeleton-meta-lines">
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
        <div class="skeleton-pulse skeleton-meta-line"></div>
      </div>
      <div class="skeleton-pulse skeleton-summary"></div>
    </div>
  </section>
</template>

<style scoped>
@keyframes pulse {
  0% { background-position: 200% 0; }
  100% { background-position: -200% 0; }
}
.skeleton-pulse {
  background: linear-gradient(90deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.1) 50%, rgba(255,255,255,0.05) 100%);
  background-size: 200% 100%;
  animation: pulse 1.5s ease-in-out infinite;
  border-radius: 0.5rem;
}
.skeleton-poster { width: 180px; height: 260px; }
.skeleton-title { height: 2rem; width: 60%; margin-bottom: 1rem; }
.skeleton-rating { height: 1.5rem; width: 30%; margin-bottom: 1.5rem; }
.skeleton-meta-lines { display: flex; flex-direction: column; gap: 0.75rem; margin-bottom: 1.5rem; }
.skeleton-meta-line { height: 1rem; width: 80%; }
.skeleton-summary { height: 4rem; width: 100%; }
.douban-meta-panel {
  display: flex;
  gap: 2rem;
  padding: 1.5rem;
  background: rgba(255,255,255,0.05);
  border-radius: 1.5rem;
}
.douban-meta-poster { flex-shrink: 0; width: 180px; }
.douban-meta-content { flex: 1; min-width: 0; }
</style>