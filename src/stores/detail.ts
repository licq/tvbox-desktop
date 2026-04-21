import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CatalogDetail, CatalogDetailItem, CatalogEpisodeGroup } from '@/types'

export const useDetailStore = defineStore('detail', () => {
  const item = ref<CatalogDetailItem | null>(null)
  const episodeGroups = ref<CatalogEpisodeGroup[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchDetail(itemId: number) {
    loading.value = true
    error.value = null
    try {
      const detail = await invoke<CatalogDetail>('get_catalog_detail', { id: itemId })
      item.value = detail.item
      episodeGroups.value = detail.episode_groups
    } catch (e) {
      item.value = null
      episodeGroups.value = []
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  function reset() {
    item.value = null
    episodeGroups.value = []
    error.value = null
    loading.value = false
  }

  return {
    item,
    episodeGroups,
    loading,
    error,
    fetchDetail,
    reset
  }
})
