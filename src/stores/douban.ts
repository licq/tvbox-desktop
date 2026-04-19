import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DoubanHotItem } from '@/types'

export interface MatchedHotItem {
  douban: DoubanHotItem
  vod_id: number
  vod_name: string
}

export const useDoubanStore = defineStore('douban', () => {
  const items = ref<DoubanHotItem[]>([])
  const matchedItems = ref<MatchedHotItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchHot() {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<DoubanHotItem[]>('get_douban_hot')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchMatchedHot() {
    loading.value = true
    error.value = null
    try {
      matchedItems.value = await invoke<MatchedHotItem[]>('get_matched_hot_list')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function refreshHot() {
    loading.value = true
    error.value = null
    try {
      await invoke('fetch_douban_hot')
      await fetchMatchedHot()
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { items, matchedItems, loading, error, fetchHot, fetchMatchedHot, refreshHot }
})
