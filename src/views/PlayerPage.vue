<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { useLiveStore } from '@/stores/live'
import { usePlayerStore } from '@/stores/player'

const router = useRouter()
const liveStore = useLiveStore()
const playerStore = usePlayerStore()

const videoRef = ref<HTMLVideoElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)

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
    if (channel) {
      playerStore.currentUrl = channel.sources[0]?.url
    }
  } else if (type === 'vod' && episodeUrl) {
    playerStore.currentUrl = decodeURIComponent(episodeUrl)
  }

  if (videoRef.value && playerStore.currentUrl) {
    videoRef.value.src = playerStore.currentUrl
    videoRef.value.volume = volume.value
    videoRef.value.play().then(() => {
      playing.value = true
    }).catch(console.error)
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
</script>

<template>
  <div class="player-page min-h-screen bg-black text-white">
    <!-- Video -->
    <div class="relative">
      <video
        ref="videoRef"
        class="w-full aspect-video bg-black"
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
          <button
            class="px-4 py-2 bg-white/20 rounded hover:bg-white/30 transition"
            @click="toggleFullscreen"
          >
            ⛶
          </button>
        </div>
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
