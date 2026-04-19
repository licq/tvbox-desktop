import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VodItem } from '@/types'

export const useVodStore = defineStore('vod', () => {
  const items = ref<VodItem[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchItems(type?: string) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('get_vod_items', { vtype: type || null, page: 0 })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function search(keyword: string) {
    loading.value = true
    error.value = null
    try {
      items.value = await invoke<VodItem[]>('search_vod', { keyword })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { items, loading, error, fetchItems, search }
})
