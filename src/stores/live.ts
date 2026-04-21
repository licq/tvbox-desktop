import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { LiveChannel, LiveChannelGroup } from '@/types'

interface GroupedLiveChannel {
  name: string
  source_count: number
}

interface GroupedLiveChannelGroup {
  category: string
  channels: GroupedLiveChannel[]
}

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
      const rawGroups = await invoke<GroupedLiveChannelGroup[]>('get_live_channel_groups')
      const hydratedGroups = await Promise.all(
        rawGroups.map(async (group) => {
          const groupChannels = await invoke<LiveChannel[]>('get_live_channels', {
            category: group.category
          })
          const sourceCountByName = new Map(
            group.channels.map((channel) => [channel.name, channel.source_count])
          )

          const hydratedChannels = groupChannels.map((channel) => ({
            ...channel,
            sources:
              channel.sources.length > 0
                ? channel.sources
                : Array.from({ length: sourceCountByName.get(channel.name) ?? 0 }, () => ({
                    url: '',
                    subscription_id: 0
                  }))
          }))

          return {
            category: group.category,
            channels: hydratedChannels
          }
        })
      )

      groups.value = hydratedGroups
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
