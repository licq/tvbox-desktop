import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LiveChannel, LiveChannelGroup } from '@/types'

export const useLiveStore = defineStore('live', () => {
  const channels = ref<LiveChannel[]>([])
  const groups = ref<LiveChannelGroup[]>([])
  const categories = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

  async function fetchChannels(category?: string) {
    loading.value = true
    error.value = null
    try {
      channels.value = await invoke<LiveChannel[]>('get_live_channels', { category: category || null })
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchGroups() {
    loading.value = true
    error.value = null
    try {
      groups.value = await invoke<LiveChannelGroup[]>('get_live_channel_groups')
    } catch (e) {
      error.value = String(e)
    } finally {
      loading.value = false
    }
  }

  async function fetchCategories() {
    try {
      categories.value = await invoke<string[]>('get_live_categories')
    } catch (e) {
      error.value = String(e)
    }
  }

  return { channels, groups, categories, loading, error, fetchChannels, fetchGroups, fetchCategories }
})
