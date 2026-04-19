import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VodItem } from '@/types'

export const useVodStore = defineStore('vod', () => {
  const items = ref<VodItem[]>([])
  const currentItem = ref<VodItem | null>(null)
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

  async function fetchDetail(id: number) {
    loading.value = true
    error.value = null
    try {
      currentItem.value = await invoke<VodItem>('get_vod_detail', { id })
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

  return { items, currentItem, loading, error, fetchItems, fetchDetail, search }
})
