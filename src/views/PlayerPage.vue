<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-shell'
import Hls from 'hls.js'
import { useLiveStore } from '@/stores/live'
import { usePlayerStore } from '@/stores/player'
import { usePlaybackStore } from '@/stores/playback'

const route = useRoute()
const router = useRouter()
const liveStore = useLiveStore()
const playerStore = usePlayerStore()
const playbackStore = usePlaybackStore()

type PlayerSource = {
  url: string
  label: string
  kind: 'hls' | 'http' | 'external' | 'embed'
}

const videoRef = ref<HTMLVideoElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)
const errorMsg = ref('')

const sources = ref<PlayerSource[]>([])
const currentSourceIndex = ref(0)

const currentSource = computed(() => sources.value[currentSourceIndex.value] ?? null)
const isEmbedSource = computed(() => currentSource.value?.kind === 'embed')
const mode = computed(() => String(route.params.mode ?? 'live'))
const itemId = computed(() => Number(route.params.id))
const episodeUrl = computed(() => {
  const value = route.query.episode
  return typeof value === 'string' ? value : null
})
const sourceLabel = computed(() => currentSource.value?.label ?? `线路 ${currentSourceIndex.value + 1}`)

let hlsInstance: Hls | null = null
let progressUpdateInterval: number | null = null

onMounted(async () => {
  if (mode.value === 'live') {
    await liveStore.fetchChannels()
    const channel = liveStore.channels.find(channel => channel.id === itemId.value)
    if (channel && channel.sources.length > 0) {
      sources.value = channel.sources.map((source, index) => ({
        url: source.url,
        label: `直播线路 ${index + 1}`,
        kind: source.url.includes('.m3u8') ? 'hls' : 'http'
      }))
      currentSourceIndex.value = 0
      await playSource(sources.value[0])
    } else {
      errorMsg.value = '当前频道没有可用线路'
    }
  } else if (mode.value === 'vod' && episodeUrl.value) {
    const url = decodeURIComponent(episodeUrl.value)
    const resolved = await playbackStore.resolve(url)
    sources.value = resolved.candidates.map(candidate => ({
      url: candidate.url,
      label: candidate.label,
      kind: candidate.kind
    }))
    currentSourceIndex.value = 0

    if (resolved.status === 'ready' && sources.value.length > 0) {
      await playSource(sources.value[0])
    } else if (resolved.status === 'external_required' && sources.value.length > 0) {
      errorMsg.value = resolved.errorMessage ?? '当前资源需要外部处理'
      await playSource(sources.value[0])
    } else {
      errorMsg.value = resolved.errorMessage ?? '当前条目没有可用线路'
    }
  } else {
    errorMsg.value = '缺少播放地址'
  }

  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }

  progressUpdateInterval = window.setInterval(() => {
    if (!videoRef.value) return
    currentTime.value = videoRef.value.currentTime
    duration.value = videoRef.value.duration || 0
  }, 1000)
})

onUnmounted(() => {
  if (progressUpdateInterval) {
    window.clearInterval(progressUpdateInterval)
  }

  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  if (mode.value === 'vod' && duration.value > 0) {
    const progress = (currentTime.value / duration.value) * 100
    void playerStore.saveHistory('vod', itemId.value, progress)
  }
})

function togglePlay() {
  if (!videoRef.value) return

  if (playing.value) {
    videoRef.value.pause()
    playing.value = false
    return
  }

  void videoRef.value.play()
  playing.value = true
}

function seek(time: number) {
  if (!videoRef.value) return
  videoRef.value.currentTime = time
}

function handleVolumeChange(event: Event) {
  const target = event.target as HTMLInputElement
  volume.value = parseFloat(target.value)
  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }
}

async function toggleFullscreen() {
  if (!document.fullscreenElement) {
    await document.documentElement.requestFullscreen()
    fullscreen.value = true
    return
  }

  await document.exitFullscreen()
  fullscreen.value = false
}

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)

  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
  }

  return `${m}:${s.toString().padStart(2, '0')}`
}

function isDrpyProtocol(url: string) {
  return url.startsWith('drpy://')
}

function resetVideoElement() {
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }
  if (videoRef.value) {
    videoRef.value.pause()
    videoRef.value.removeAttribute('src')
    videoRef.value.load()
  }
  playing.value = false
}

async function switchToSource(index: number) {
  if (index < 0 || index >= sources.value.length) return
  currentSourceIndex.value = index
  await playSource(sources.value[index])
}

async function playSource(source: PlayerSource) {
  errorMsg.value = ''
  const url = source.url

  if (isDrpyProtocol(url) || source.kind === 'external') {
    resetVideoElement()
    errorMsg.value = source.kind === 'external' ? '该线路需要外部工具处理，已尝试交给系统打开' : '该地址需要外部解析，已尝试交给系统处理'
    await open(url)
    return
  }

  if (source.kind === 'embed') {
    resetVideoElement()
    errorMsg.value = ''
    return
  }

  initHlsPlayer(url)
}

function initHlsPlayer(url: string) {
  if (!videoRef.value) return

  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  if (url.includes('.m3u8')) {
    if (Hls.isSupported()) {
      const hls = new Hls()
      hlsInstance = hls
      hls.loadSource(url)
      hls.attachMedia(videoRef.value)

      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (!data.fatal) return

        if (currentSourceIndex.value < sources.value.length - 1) {
          void switchToSource(currentSourceIndex.value + 1)
        } else {
          errorMsg.value = '所有线路均不可用'
        }
      })

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        videoRef.value?.play().then(() => {
          playing.value = true
        }).catch(() => {
          errorMsg.value = '自动播放失败，请手动开始播放'
        })
      })

      return
    }

    if (videoRef.value.canPlayType('application/vnd.apple.mpegurl')) {
      videoRef.value.src = url
      videoRef.value.play().then(() => {
        playing.value = true
      }).catch(() => {
        errorMsg.value = '自动播放失败，请手动开始播放'
      })
      return
    }
  }

  videoRef.value.src = url
  videoRef.value.play().then(() => {
    playing.value = true
  }).catch(() => {
    errorMsg.value = '无法直接播放当前地址'
  })
}
</script>

<template>
  <div class="min-h-screen bg-[#05070b] px-4 py-4 text-white md:px-6">
    <div class="mx-auto max-w-[1500px]">
      <div class="mb-4 flex flex-wrap items-center justify-between gap-3">
        <button class="action-button action-button-secondary" @click="router.back()">
          返回
        </button>
        <div class="flex items-center gap-2">
          <div class="rounded-full bg-white/8 px-3 py-2 text-[11px] uppercase tracking-[0.28em] text-white/50">
            {{ mode }}
          </div>
          <div class="rounded-full bg-white/8 px-3 py-2 text-[11px] uppercase tracking-[0.28em] text-white/50">
            {{ sourceLabel }}
          </div>
        </div>
      </div>

      <div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_320px]">
        <section class="surface-panel overflow-hidden rounded-[2rem]">
          <div class="relative">
            <video
              v-show="!isEmbedSource"
              ref="videoRef"
              class="aspect-video w-full bg-black"
              :title="currentSource?.url || ''"
              @click="togglePlay"
            ></video>
            <iframe
              v-if="isEmbedSource && currentSource"
              class="aspect-video w-full bg-black"
              :src="currentSource.url"
              allow="autoplay; fullscreen"
              referrerpolicy="no-referrer"
            ></iframe>

            <div class="pointer-events-none absolute inset-x-0 top-0 h-32 bg-gradient-to-b from-black/60 to-transparent"></div>
            <div class="pointer-events-none absolute inset-x-0 bottom-0 h-40 bg-gradient-to-t from-black/85 via-black/40 to-transparent"></div>

            <div class="absolute inset-x-0 bottom-0 p-5 md:p-6">
              <div v-if="errorMsg" class="mb-4 rounded-[1.25rem] border border-amber-300/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-100">
                {{ errorMsg }}
              </div>

              <div class="space-y-4">
                <div class="flex items-center gap-3 text-xs text-white/60">
                  <span>{{ formatTime(currentTime) }}</span>
                  <input
                    type="range"
                    :value="currentTime"
                    :max="duration || 100"
                    class="h-1 flex-1 cursor-pointer appearance-none rounded-full bg-white/15"
                    @input="seek(parseFloat(($event.target as HTMLInputElement).value))"
                  />
                  <span>{{ formatTime(duration) }}</span>
                </div>

                <div class="flex flex-wrap items-center justify-between gap-4">
                  <div class="flex flex-wrap items-center gap-3">
                    <button class="action-button action-button-primary" @click="togglePlay">
                      {{ playing ? '暂停' : '播放' }}
                    </button>
                    <button class="action-button action-button-secondary" @click="toggleFullscreen">
                      {{ fullscreen ? '退出全屏' : '全屏' }}
                    </button>
                  </div>

                  <div class="flex items-center gap-3 rounded-full bg-white/8 px-4 py-2">
                    <span class="text-xs uppercase tracking-[0.28em] text-white/38">Volume</span>
                    <input
                      type="range"
                      :value="volume"
                      min="0"
                      max="1"
                      step="0.1"
                      class="h-1 w-24 cursor-pointer appearance-none rounded-full bg-white/15"
                      @input="handleVolumeChange"
                    />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </section>

        <aside class="surface-panel rounded-[2rem] px-5 py-5">
          <div class="section-title">线路面板</div>
          <p class="mt-2 text-sm text-white/48">线路切换、模式和失败反馈统一放到视频右侧，避免遮住关键画面。</p>

          <div class="mt-6 space-y-3">
            <button
              v-for="(_, index) in sources"
              :key="index"
              :class="[
                'w-full rounded-[1.2rem] border px-4 py-3 text-left transition',
                index === currentSourceIndex
                  ? 'border-[#d89a57]/40 bg-[#d89a57]/12 text-white'
                  : 'border-white/6 bg-white/[0.03] text-white/68 hover:bg-white/[0.06]'
              ]"
              @click="switchToSource(index)"
            >
              <div class="text-[10px] uppercase tracking-[0.28em] text-white/35">Line {{ index + 1 }}</div>
              <div class="mt-2 text-sm font-medium">{{ sources[index].url ? '内置地址' : '占位线路' }}</div>
            </button>
          </div>

          <div class="mt-6 rounded-[1.4rem] border border-white/6 bg-white/[0.03] p-4">
            <div class="text-[11px] uppercase tracking-[0.28em] text-white/34">Current Url</div>
            <div class="mt-3 break-all text-xs leading-6 text-white/52">
              {{ currentSource?.url || '当前没有可用地址' }}
            </div>
          </div>
        </aside>
      </div>
    </div>
  </div>
</template>
