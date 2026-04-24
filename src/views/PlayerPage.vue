<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-shell'
import { useLiveStore } from '@/stores/live'
import { usePlayerStore } from '@/stores/player'
import { usePlaybackStore } from '@/stores/playback'
import PlaybackDrawer from '@/components/player/PlaybackDrawer.vue'
import PlaybackNotice from '@/components/player/PlaybackNotice.vue'
import { describeMediaErrorCode, describePlaybackFailure, isAutoplayBlocked } from '@/utils/player'
import { enterFullscreen, exitFullscreen } from '@/utils/fullscreen'
import type Hls from 'hls.js'

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
const videoWrapRef = ref<HTMLElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)
const errorMsg = ref('')
const pendingAutoplay = ref(false)

const sources = ref<PlayerSource[]>([])
const currentSourceIndex = ref(0)
const failedSourceIndexes = ref<number[]>([])

const currentSource = computed(() => sources.value[currentSourceIndex.value] ?? null)
const isEmbedSource = computed(() => currentSource.value?.kind === 'embed')
const mode = computed(() => String(route.params.mode ?? 'live'))
const itemId = computed(() => Number(route.params.id))
const episodeUrl = computed(() => {
  const value = route.query.episode
  return typeof value === 'string' ? value : null
})
const episodeId = computed(() => {
  const value = route.query.episodeId
  const numeric = typeof value === 'string' ? Number(value) : NaN
  return Number.isFinite(numeric) && numeric > 0 ? numeric : undefined
})
const sourceLabel = computed(() => currentSource.value?.label ?? `线路 ${currentSourceIndex.value + 1}`)
const playerStatusText = computed(() => {
  if (errorMsg.value) return '需要处理'
  if (pendingAutoplay.value) return '等待播放'
  if (playing.value) return '播放中'
  if (playbackStore.status === 'resolving') return '解析中'
  return playbackStore.status === 'idle' ? '就绪' : playbackStore.status
})
const playerModeLabel = computed(() => mode.value === 'live' ? '直播' : '点播')
const noticeTone = computed(() => playbackStore.status === 'failed' ? 'danger' : 'warning')

let hlsInstance: Hls | null = null
let hlsConstructorPromise: Promise<typeof import('hls.js').default> | null = null
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
    const resolved = await playbackStore.resolve(url, episodeId.value)
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

  void attemptPlayback(true)
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
    await enterFullscreen(
      videoWrapRef.value,
      () => document.documentElement.requestFullscreen()
    )
    fullscreen.value = true
  } else {
    await exitFullscreen(document, () => document.exitFullscreen())
    fullscreen.value = false
  }
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
  pendingAutoplay.value = false
}

async function getHlsConstructor() {
  if (!hlsConstructorPromise) {
    hlsConstructorPromise = import('hls.js').then(module => module.default)
  }

  return hlsConstructorPromise
}

async function switchToSource(index: number) {
  if (index < 0 || index >= sources.value.length) return
  currentSourceIndex.value = index
  await playSource(sources.value[index])
}

function markCurrentSourceFailed() {
  if (!failedSourceIndexes.value.includes(currentSourceIndex.value)) {
    failedSourceIndexes.value = [...failedSourceIndexes.value, currentSourceIndex.value]
  }
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

  await initHlsPlayer(url)
}

async function initHlsPlayer(url: string) {
  if (!videoRef.value) return

  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  if (url.includes('.m3u8')) {
    const Hls = await getHlsConstructor()

    if (Hls.isSupported()) {
      const hls = new Hls()
      hlsInstance = hls
      hls.loadSource(url)
      hls.attachMedia(videoRef.value)

      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (!data.fatal) return
        markCurrentSourceFailed()

        if (currentSourceIndex.value < sources.value.length - 1) {
          void switchToSource(currentSourceIndex.value + 1)
        } else {
          errorMsg.value = data.error?.message || '所有线路均不可用'
        }
      })

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        void attemptPlayback(false)
      })

      return
    }

    if (videoRef.value.canPlayType('application/vnd.apple.mpegurl')) {
      videoRef.value.src = url
      videoRef.value.load()
      pendingAutoplay.value = true
      return
    }
  }

  videoRef.value.src = url
  videoRef.value.load()
  pendingAutoplay.value = true
}

async function attemptPlayback(manual: boolean) {
  if (!videoRef.value) return

  try {
    await videoRef.value.play()
    errorMsg.value = ''
    playing.value = true
    pendingAutoplay.value = false
  } catch (error) {
    playing.value = false
    pendingAutoplay.value = false
    errorMsg.value = manual ? describePlaybackFailure(error) : describePlaybackFailure(error)
    if (!manual && isAutoplayBlocked(error)) {
      return
    }
  }
}

function handleCanPlay() {
  if (!pendingAutoplay.value) return
  void attemptPlayback(false)
}

function handleVideoPlay() {
  playing.value = true
  errorMsg.value = ''
}

function handleVideoPause() {
  playing.value = false
}

function handleVideoError() {
  pendingAutoplay.value = false
  const mediaError = videoRef.value?.error
  const message = describeMediaErrorCode(mediaError?.code)
  markCurrentSourceFailed()

  if (currentSourceIndex.value < sources.value.length - 1) {
    errorMsg.value = `${message}，正在切换下一条线路`
    void switchToSource(currentSourceIndex.value + 1)
    return
  }

  errorMsg.value = message
}
</script>

<template>
  <div class="player-shell">
    <div class="player-frame">
      <header class="player-topbar">
        <button class="action-button action-button-secondary" type="button" @click="router.back()">
          返回
        </button>
        <div class="player-context">
          <span>{{ playerModeLabel }}</span>
          <span>{{ sourceLabel }}</span>
          <span>{{ playerStatusText }}</span>
        </div>
      </header>

      <div class="player-layout">
        <section class="player-stage">
          <div class="player-video-wrap" ref="videoWrapRef">
            <video
              v-show="!isEmbedSource"
              ref="videoRef"
              class="player-video"
              :title="currentSource?.url || ''"
              playsinline
              @click="togglePlay"
              @canplay="handleCanPlay"
              @play="handleVideoPlay"
              @pause="handleVideoPause"
              @error="handleVideoError"
            ></video>
            <iframe
              v-if="isEmbedSource && currentSource"
              class="player-video"
              :src="currentSource.url"
              allow="autoplay; fullscreen"
              referrerpolicy="no-referrer"
            ></iframe>

            <div class="player-vignette-top"></div>
            <div class="player-vignette-bottom"></div>

            <div class="player-overlay">
              <PlaybackNotice v-if="errorMsg" :message="errorMsg" :tone="noticeTone" />

              <div class="player-controls">
                <div class="player-progress">
                  <span>{{ formatTime(currentTime) }}</span>
                  <input
                    type="range"
                    :value="currentTime"
                    :max="duration || 100"
                    class="player-range"
                    @input="seek(parseFloat(($event.target as HTMLInputElement).value))"
                  />
                  <span>{{ formatTime(duration) }}</span>
                </div>

                <div class="player-control-row">
                  <div class="player-control-actions">
                    <button class="action-button action-button-primary" type="button" @click="togglePlay">
                      {{ playing ? '暂停' : '播放' }}
                    </button>
                    <button class="action-button action-button-secondary" type="button" @click="toggleFullscreen">
                      {{ fullscreen ? '退出全屏' : '全屏' }}
                    </button>
                  </div>

                  <label class="player-volume">
                    <span>Volume</span>
                    <input
                      type="range"
                      :value="volume"
                      min="0"
                      max="1"
                      step="0.1"
                      class="player-range player-volume-range"
                      @input="handleVolumeChange"
                    />
                  </label>
                </div>
              </div>
            </div>
          </div>
        </section>

        <PlaybackDrawer
          :sources="sources"
          :current-index="currentSourceIndex"
          :failed-indexes="failedSourceIndexes"
          :status="playerStatusText"
          :error-message="errorMsg || playbackStore.errorMessage"
          @select="switchToSource"
        />
      </div>
    </div>
  </div>
</template>
