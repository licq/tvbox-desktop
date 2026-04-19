<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-shell'
import Hls from 'hls.js'
import { useLiveStore } from '@/stores/live'
import { usePlayerStore } from '@/stores/player'
import type { ChannelSource } from '@/types'

const router = useRouter()
const liveStore = useLiveStore()
const playerStore = usePlayerStore()

const videoRef = ref<HTMLVideoElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)
const errorMsg = ref('')

// Multi-source state
const sources = ref<ChannelSource[]>([])
const currentSourceIndex = ref(0)

const currentSource = computed(() => sources.value[currentSourceIndex.value])

let hlsInstance: Hls | null = null
let progressUpdateInterval: number | null = null

// Parse URL params
const params = new URLSearchParams(globalThis.location.search)
const episodeUrl = params.get('episode')

// Get type and id from path
const pathParts = globalThis.location.pathname.split('/')
const type = pathParts[2] // 'live' or 'vod'
const id = parseInt(pathParts[3])

onMounted(async () => {
  if (type === 'live') {
    await liveStore.fetchChannels()
    const channel = liveStore.channels.find(c => c.id === id)
    if (channel && channel.sources.length > 0) {
      sources.value = channel.sources
      currentSourceIndex.value = 0
      playSource(channel.sources[0].url)
    }
  } else if (type === 'vod' && episodeUrl) {
    const url = decodeURIComponent(episodeUrl)
    sources.value = [{ url, subscription_id: 0 }]
    currentSourceIndex.value = 0
    playSource(url)
  }

  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }

  progressUpdateInterval = globalThis.setInterval(() => {
    if (videoRef.value) {
      currentTime.value = videoRef.value.currentTime
      duration.value = videoRef.value.duration || 0
    }
  }, 1000)
})

onUnmounted(() => {
  if (progressUpdateInterval) {
    globalThis.clearInterval(progressUpdateInterval)
  }
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }
  // Save play history
  if (type === 'vod' && duration.value > 0) {
    const progress = (currentTime.value / duration.value) * 100
    playerStore.saveHistory('vod', id, progress)
  }
})

function togglePlay() {
  if (!videoRef.value) return
  if (playing.value) {
    videoRef.value.pause()
  } else {
    videoRef.value.play()
  }
  playing.value = !playing.value
}

function seek(time: number) {
  if (!videoRef.value) return
  videoRef.value.currentTime = time
}

function handleVolumeChange(e: Event) {
  const target = e.target as HTMLInputElement
  volume.value = parseFloat(target.value)
  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }
}

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen()
    fullscreen.value = true
  } else {
    document.exitFullscreen()
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

function isDrpyProtocol(url: string): boolean {
  return url.startsWith('drpy://')
}

function switchToSource(index: number) {
  if (index >= 0 && index < sources.value.length) {
    currentSourceIndex.value = index
    playSource(sources.value[index].url)
  }
}

async function playSource(url: string) {
  errorMsg.value = ''
  if (isDrpyProtocol(url)) {
    await open(url)
    return
  }
  initHlsPlayer(url)
}

function initHlsPlayer(url: string) {
  if (!videoRef.value) return

  // Clean up existing HLS instance
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  // Check if URL is HLS (.m3u8)
  if (url.includes('.m3u8')) {
    if (Hls.isSupported()) {
      const hls = new Hls()
      hlsInstance = hls
      hls.loadSource(url)
      hls.attachMedia(videoRef.value)

      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (data.fatal) {
          console.error('HLS fatal error:', data)
          // Try next source
          if (currentSourceIndex.value < sources.value.length - 1) {
            switchToSource(currentSourceIndex.value + 1)
          } else {
            errorMsg.value = '所有源均不可用'
          }
        }
      })

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        videoRef.value?.play().then(() => {
          playing.value = true
        }).catch(console.error)
      })

      return hls
    } else if (videoRef.value.canPlayType('application/vnd.apple.mpegurl')) {
      // Safari native HLS support
      videoRef.value.src = url
      videoRef.value.play().then(() => {
        playing.value = true
      }).catch(console.error)
      return null
    }
  }

  // Regular video source
  videoRef.value.src = url
  videoRef.value.play().then(() => {
    playing.value = true
  }).catch(console.error)
  return null
}
</script>

<template>
  <div class="player-page min-h-screen bg-black text-white">
    <!-- Video -->
    <div class="relative">
      <video
        ref="videoRef"
        class="w-full aspect-video bg-black"
        :title="currentSource?.url || ''"
        @click="togglePlay"
      ></video>

      <!-- Controls -->
      <div class="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black/80 to-transparent p-4">
        <!-- Progress -->
        <div class="flex items-center gap-2 mb-2">
          <span class="text-sm">{{ formatTime(currentTime) }}</span>
          <input
            type="range"
            :value="currentTime"
            :max="duration || 100"
            class="flex-1 h-1 bg-gray-600 rounded-lg appearance-none cursor-pointer"
            @input="seek(parseFloat(($event.target as HTMLInputElement).value))"
          />
          <span class="text-sm">{{ formatTime(duration) }}</span>
        </div>

        <!-- Buttons -->
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-4">
            <button
              class="px-4 py-2 bg-white/20 rounded hover:bg-white/30 transition"
              @click="togglePlay"
            >
              {{ playing ? '⏸️' : '▶️' }}
            </button>
            <div class="flex items-center gap-2">
              <span>🔊</span>
              <input
                type="range"
                :value="volume"
                min="0"
                max="1"
                step="0.1"
                class="w-20 h-1 bg-gray-600 rounded-lg appearance-none cursor-pointer"
                @input="handleVolumeChange"
              />
            </div>
          </div>

          <!-- Source selector -->
          <div v-if="sources.length > 1" class="source-selector flex items-center gap-2">
            <span class="text-sm">{{ currentSourceIndex + 1 }}/{{ sources.length }}</span>
            <button
              v-for="(_, i) in sources"
              :key="i"
              :class="['px-2 py-1 text-xs rounded', i === currentSourceIndex ? 'bg-primary' : 'bg-gray-700']"
              @click="switchToSource(i)"
            >
              源{{ i + 1 }}
            </button>
          </div>

          <button
            class="px-4 py-2 bg-white/20 rounded hover:bg-white/30 transition"
            @click="toggleFullscreen"
          >
            ⛶
          </button>
        </div>
      </div>

      <!-- Error message -->
      <div v-if="errorMsg" class="absolute inset-0 flex items-center justify-center bg-black/60">
        <span class="text-red-400 text-lg">{{ errorMsg }}</span>
      </div>
    </div>

    <!-- Info -->
    <div class="p-4">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="router.back()"
      >
        ← 返回
      </button>
    </div>
  </div>
</template>
