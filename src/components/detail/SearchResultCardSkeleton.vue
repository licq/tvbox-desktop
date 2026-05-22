<script setup lang="ts">
defineProps<{
  itemType?: 'movie' | 'series' | 'variety' | 'anime'
}>()
</script>

<template>
  <div class="search-result-card skeleton">
    <div class="card-left">
      <div class="card-poster skeleton-poster"></div>
      <div class="card-info">
        <div class="card-title-row">
          <div class="skeleton-title"></div>
          <div class="skeleton-tag"></div>
        </div>
        <div class="skeleton-meta"></div>
      </div>
    </div>
    <div class="card-right">
      <div class="source-action-area">
        <!-- Movie mode: show episode button placeholders -->
        <template v-if="itemType === 'movie'">
          <div class="skeleton-movie-row">
            <div class="skeleton-line"></div>
            <div class="skeleton-chips">
              <div class="skeleton-chip" v-for="i in 3" :key="i"></div>
            </div>
          </div>
        </template>
        <!-- Series mode: show source selector placeholders -->
        <template v-else>
          <div class="skeleton-source-selector">
            <div class="skeleton-chip" v-for="i in 3" :key="i"></div>
          </div>
          <div class="skeleton-episode-grid">
            <div class="skeleton-chip" v-for="i in 8" :key="i"></div>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.search-result-card.skeleton {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.75rem 1rem;
  display: flex;
  gap: 0.75rem;
  align-items: stretch;
  overflow: hidden;
}
.card-left {
  display: flex;
  align-items: center;
  gap: 0.7rem;
  flex: 0 0 auto;
  width: 240px;
  min-width: 0;
}
.skeleton-poster {
  width: 3.2rem;
  height: 4.8rem;
  border-radius: 0.4rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
  flex-shrink: 0;
}
.card-info {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 0.25rem;
}
.card-title-row {
  display: flex;
  align-items: center;
  gap: 0.4rem;
}
.skeleton-title {
  height: 1rem;
  width: 5rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-tag {
  height: 0.85rem;
  width: 2rem;
  border-radius: 0.25rem;
  background: rgba(255, 255, 255, 0.04);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-meta {
  height: 0.7rem;
  width: 4rem;
  border-radius: 0.25rem;
  background: rgba(255, 255, 255, 0.04);
  animation: pulse 1.5s ease-in-out infinite;
}
.card-right {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 0.3rem;
  flex-wrap: wrap;
}
.source-action-area {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  align-items: flex-end;
}
.skeleton-movie-row {
  display: flex;
  flex-direction: column;
  gap: 0.45rem;
  align-items: flex-end;
}
.skeleton-line {
  height: 0.75rem;
  width: 6rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-source-selector,
.skeleton-chips {
  display: flex;
  gap: 0.3rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.skeleton-chip {
  height: 1.8rem;
  width: 3.5rem;
  border-radius: 0.55rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-episode-grid .skeleton-chip {
  width: clamp(3.25rem, 8vw, 5rem);
  height: 2.75rem;
  border-radius: 1.1rem;
}

/* Use CSS Grid to match SearchResultCard's .loading-grid layout */
.skeleton-episode-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(3rem, 1fr));
  gap: 0.35rem;
}

@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}

@media (max-width: 768px) {
  .search-result-card.skeleton {
    flex-direction: column;
    align-items: stretch;
  }
  .card-left {
    width: auto;
  }
  .card-right {
    justify-content: flex-start;
  }
  .source-action-area {
    align-items: flex-start;
  }
  .skeleton-source-selector,
  .skeleton-chips,
  .skeleton-episode-grid {
    justify-content: flex-start;
  }
}
</style>
