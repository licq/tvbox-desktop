import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { VodItem, Episode } from '@/types'

// Backend returns episodes as JSON string, parse it to Episode[]
function parseEpisodes(episodes: string | Episode[]): Episode[] {
  if (!episodes) return []
  if (Array.isArray(episodes)) return episodes
  try {
    return JSON.parse(episodes) as Episode[]
  } catch {
    return []
  }
}

function parseVodItem(item: any): VodItem {
  return {
    ...item,
    episodes: parseEpisodes(item.episodes)
  }
}

export const useVodStore = defineStore('vod', () => {
  const items = ref<VodItem[]>([])
  const currentItem = ref<VodItem | null>(null)
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchItems(type?: string) {
    loading.value = true
    error.value = null
    try {
      const rawItems = await invoke<any[]>('get_vod_items', { vtype: type || null, page: 0 })
      items.value = rawItems.map(parseVodItem)
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
      const rawItem = await invoke<any>('get_vod_detail', { id })
      currentItem.value = parseVodItem(rawItem)
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
      const rawItems = await invoke<any[]>('search_vod', { keyword })
      items.value = rawItems.map(parseVodItem)
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  return { items, currentItem, loading, error, fetchItems, fetchDetail, search }
})
