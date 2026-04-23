import { computed, ref } from 'vue'
import { defineStore } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import type { PlaybackCandidate, ResolvedPlayback } from '@/types'

type PlaybackStatus = 'idle' | 'resolving' | 'ready' | 'failed' | 'external_required'

export const usePlaybackStore = defineStore('playback', () => {
  const status = ref<PlaybackStatus>('idle')
  const candidates = ref<PlaybackCandidate[]>([])
  const currentIndex = ref(0)
  const errorMessage = ref<string | null>(null)

  const currentCandidate = computed(() => candidates.value[currentIndex.value] ?? null)

  async function resolve(input: string, episodeId?: number) {
    status.value = 'resolving'
    errorMessage.value = null
    try {
      const resolved = await invoke<ResolvedPlayback>('resolve_playback', {
        input,
        episodeId
      })
      applyResolved(resolved)
      return resolved
    } catch (e) {
      status.value = 'failed'
      candidates.value = []
      currentIndex.value = 0
      errorMessage.value = String(e)
      throw e
    }
  }

  function applyResolved(resolved: ResolvedPlayback) {
    status.value = resolved.status
    candidates.value = resolved.candidates
    currentIndex.value = 0
    errorMessage.value = resolved.errorMessage ?? null
  }

  function handleFatalPlaybackError(reason: string) {
    if (currentIndex.value < candidates.value.length - 1) {
      currentIndex.value += 1
      return
    }

    status.value = 'failed'
    errorMessage.value = `All playback candidates failed: ${reason}`
  }

  function selectCandidate(index: number) {
    if (index >= 0 && index < candidates.value.length) {
      currentIndex.value = index
    }
  }

  function reset() {
    status.value = 'idle'
    candidates.value = []
    currentIndex.value = 0
    errorMessage.value = null
  }

  return {
    status,
    candidates,
    currentIndex,
    currentCandidate,
    errorMessage,
    resolve,
    applyResolved,
    handleFatalPlaybackError,
    selectCandidate,
    reset
  }
})
