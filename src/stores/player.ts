import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CatalogDetail, PlayHistory, UnifiedEpisode } from '@/types'

export const usePlayerStore = defineStore('player', () => {
  const currentUrl = ref<string | null>(null)
  const history = ref<PlayHistory[]>([])
  const loading = ref(false)
  const pendingUnifiedEpisode = ref<UnifiedEpisode | null>(null)
  const pendingPlaybackDetail = ref<CatalogDetail | null>(null)

  function setPendingUnifiedEpisode(ep: UnifiedEpisode | null) {
    pendingUnifiedEpisode.value = ep
  }

  function setPendingPlaybackDetail(detail: CatalogDetail | null) {
    pendingPlaybackDetail.value = detail
  }

  function takePendingPlaybackDetail() {
    const detail = pendingPlaybackDetail.value
    pendingPlaybackDetail.value = null
    return detail
  }

  async function saveHistory(itemType: string, itemId: number, progress: number) {
    try {
      await invoke('save_play_history', { itemType, itemId, progress })
    } catch (e) {
      console.error('保存播放历史失败:', e)
    }
  }

  async function fetchHistory() {
    loading.value = true
    try {
      history.value = await invoke<PlayHistory[]>('get_play_history')
    } catch (e) {
      console.error('获取播放历史失败:', e)
    } finally {
      loading.value = false
    }
  }

  return {
    pendingUnifiedEpisode,
    setPendingUnifiedEpisode,
    pendingPlaybackDetail,
    setPendingPlaybackDetail,
    takePendingPlaybackDetail,
    currentUrl,
    history,
    loading,
    saveHistory,
    fetchHistory
  }
})
