<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDetailStore } from '@/stores/detail'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
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
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1400px]">
      <section
        class="surface-panel relative overflow-hidden rounded-[2.4rem] px-6 py-6 md:px-8 md:py-8"
        :style="backdropStyle"
      >
        <div class="relative">
          <button class="action-button action-button-secondary" @click="router.back()">
            返回片库
          </button>

          <div v-if="detailStore.loading" class="flex min-h-[420px] items-center justify-center">
            <LoadingSpinner size="lg" />
          </div>

          <div v-else-if="detailStore.item" class="mt-8 grid gap-8 xl:grid-cols-[280px_minmax(0,1fr)]">
            <div class="poster-shadow overflow-hidden rounded-[2rem]">
              <img
                v-if="detailStore.item.poster"
                :src="detailStore.item.poster"
                :alt="detailStore.item.title"
                class="aspect-[2/3] w-full object-cover"
              />
              <div v-else class="flex aspect-[2/3] items-center justify-center bg-slate-900 text-6xl text-white/30">
                🎬
              </div>
            </div>

            <div class="flex flex-col justify-between">
              <div>
                <div class="eyebrow">{{ detailStore.item.item_type }}</div>
                <h1 class="mt-3 max-w-4xl text-4xl font-semibold leading-tight text-white md:text-5xl">
                  {{ detailStore.item.title }}
                </h1>
                <p v-if="detailStore.item.summary" class="mt-5 max-w-3xl text-sm leading-7 text-white/68 md:text-base">
                  {{ detailStore.item.summary }}
                </p>

                <div class="mt-6 flex flex-wrap gap-3">
                  <div class="surface-muted rounded-full px-4 py-2 text-xs uppercase tracking-[0.24em] text-white/56">
                    {{ detailStore.episodeGroups.length }} 条线路
                  </div>
                  <div class="surface-muted rounded-full px-4 py-2 text-xs uppercase tracking-[0.24em] text-white/56">
                    {{ detailStore.episodeGroups.reduce((count, group) => count + group.episodes.length, 0) }} 个可播入口
                  </div>
                </div>
              </div>

              <div class="mt-8 grid gap-4 md:grid-cols-3">
                <div class="surface-muted rounded-[1.6rem] p-4">
                  <div class="text-[11px] uppercase tracking-[0.28em] text-white/34">当前结构</div>
                  <div class="mt-3 text-lg font-semibold text-white">来源分组 + 选集矩阵</div>
                </div>
                <div class="surface-muted rounded-[1.6rem] p-4">
                  <div class="text-[11px] uppercase tracking-[0.28em] text-white/34">播放方式</div>
                  <div class="mt-3 text-lg font-semibold text-white">进入播放器后选线</div>
                </div>
                <div class="surface-muted rounded-[1.6rem] p-4">
                  <div class="text-[11px] uppercase tracking-[0.28em] text-white/34">状态目标</div>
                  <div class="mt-3 text-lg font-semibold text-white">先分清源，再点播</div>
                </div>
              </div>
            </div>
          </div>

          <div v-else class="flex min-h-[320px] items-center justify-center text-sm text-white/45">
            没有找到内容详情。
          </div>
        </div>
      </section>

      <section v-if="detailStore.episodeGroups.length" class="mt-6 space-y-6">
        <div
          v-for="group in detailStore.episodeGroups"
          :key="group.source_name"
          class="surface-panel rounded-[2rem] px-6 py-6 md:px-7"
        >
          <div class="flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
            <div>
              <div class="section-title">{{ group.source_name }}</div>
              <p class="mt-2 text-sm text-white/52">同一来源下集中展示选集，减少大列表混排造成的误点。</p>
            </div>
            <div class="text-xs uppercase tracking-[0.24em] text-white/35">{{ group.episodes.length }} episodes</div>
          </div>

          <div class="mt-6 grid gap-3 sm:grid-cols-3 lg:grid-cols-5 xl:grid-cols-7">
            <button
              v-for="episode in group.episodes"
              :key="episode.id"
              class="surface-muted rounded-[1.2rem] px-4 py-3 text-left transition duration-300 hover:-translate-y-1 hover:bg-white/[0.08]"
              @click="handlePlay(episode)"
            >
              <div class="text-[10px] uppercase tracking-[0.28em] text-white/34">Episode</div>
              <div class="mt-3 text-sm font-medium text-white">{{ episode.episode_label }}</div>
            </button>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>
