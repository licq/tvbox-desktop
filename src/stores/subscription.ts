import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SourceSubscription } from '@/types'

export const useSubscriptionStore = defineStore('subscription', () => {
  const subscriptions = ref<SourceSubscription[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const refreshProgress = ref<{
    name: string
    live: number
    movie: number
    series: number
    variety: number
    anime: number
    other: number
  }[]>([])

  async function fetchSubscriptions() {
    loading.value = true
    error.value = null
    try {
      subscriptions.value = await invoke<SourceSubscription[]>('get_subscriptions')
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function addSubscription(name: string, url: string) {
    try {
      const sub = await invoke<SourceSubscription>('add_subscription', { name, url })
      subscriptions.value.push(sub)
      return sub
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function deleteSubscription(id: number) {
    try {
      await invoke('delete_subscription', { id })
      subscriptions.value = subscriptions.value.filter(s => s.id !== id)
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function refreshSubscription(id: number, reload = true) {
    try {
      const result = await invoke<any>('refresh_subscription', { id })
      if (result) {
        refreshProgress.value.push({
          name: result.subscription_name || '订阅',
          live: result.live_count || 0,
          movie: result.movie_count || 0,
          series: result.series_count || 0,
          variety: result.variety_count || 0,
          anime: result.anime_count || 0,
          other: result.other_count || 0,
        })
      }
      if (reload) {
        await fetchSubscriptions()
      }
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  async function toggleSubscription(id: number, enabled: boolean) {
    try {
      await invoke('toggle_subscription', { id, enabled })
      const sub = subscriptions.value.find(s => s.id === id)
      if (sub) sub.enabled = enabled
    } catch (e) {
      error.value = String(e)
      throw e
    }
  }

  return {
    subscriptions,
    loading,
    error,
    refreshProgress,
    fetchSubscriptions,
    addSubscription,
    deleteSubscription,
    refreshSubscription,
    toggleSubscription
  }
})
