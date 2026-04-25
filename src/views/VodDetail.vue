<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import DetailHero from '@/components/detail/DetailHero.vue'
import RecommendedSourcePanel from '@/components/detail/RecommendedSourcePanel.vue'
import EpisodeGroupPanel from '@/components/detail/EpisodeGroupPanel.vue'
import EpisodeGroupSkeleton from '@/components/detail/EpisodeGroupSkeleton.vue'
import type { CatalogEpisode } from '@/types'

const route = useRoute()
const router = useRouter()
const detailStore = useDetailStore()

const itemId = computed(() => Number(route.params.itemId))
const backdropStyle = computed(() => {
  const poster = detailStore.item?.poster
  if (!poster) return undefined

  return {
    backgroundImage: `linear-gradient(90deg, rgba(7, 10, 15, 0.96), rgba(7, 10, 15, 0.78) 45%, rgba(7, 10, 15, 0.92)), url(${poster})`,
    backgroundSize: 'cover',
    backgroundPosition: 'center'
  }
})
const episodeCount = computed(() =>
  detailStore.episodeGroups.reduce((count, group) => count + group.episodes.length, 0)
)
const firstPlayableEpisode = computed(() => detailStore.recommendedGroup?.episodes[0] ?? null)

async function loadDetail() {
  if (!Number.isFinite(itemId.value) || itemId.value <= 0) {
    return
  }

  await detailStore.fetchDetail(itemId.value)
}

onMounted(loadDetail)
watch(itemId, loadDetail)

function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episode=${encodeURIComponent(episode.play_url)}&episodeId=${episode.id}`)
}

function handlePlayNow() {
  if (firstPlayableEpisode.value) {
    handlePlay(firstPlayableEpisode.value)
  }
}
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1400px]">
      <button class="action-button action-button-secondary" type="button" @click="router.back()">
        返回片库
      </button>

      <div v-if="detailStore.loading && !detailStore.item" class="surface-panel mt-6 flex min-h-[420px] items-center justify-center rounded-[2.4rem]">
        <LoadingSpinner size="lg" />
      </div>

      <div v-else-if="detailStore.item" class="mt-6 space-y-6">
        <div class="detail-backdrop-shell" :style="backdropStyle">
          <DetailHero
            :item="detailStore.item"
            :source-count="detailStore.episodeGroups.length"
            :episode-count="episodeCount"
            @play="handlePlayNow"
          />
        </div>

        <RecommendedSourcePanel :group="detailStore.recommendedGroup" />

        <section v-if="detailStore.loading && detailStore.item" class="space-y-4">
          <EpisodeGroupSkeleton :count="8" />
        </section>

        <section v-else-if="detailStore.episodeGroups.length" class="space-y-4">
          <EpisodeGroupPanel
            v-for="group in detailStore.episodeGroups"
            :key="group.source_name"
            :group="group"
            :recommended="group.source_name === detailStore.recommendedGroup?.source_name"
            @play="handlePlay"
          />
        </section>

        <div v-else-if="detailStore.item" class="home-empty-state">
          当前内容没有可展示的播放入口。
        </div>
      </div>

      <div v-else class="surface-panel mt-6 flex min-h-[320px] items-center justify-center rounded-[2rem] text-sm text-white/45">
        没有找到内容详情。
      </div>
    </div>
  </div>
</template>
