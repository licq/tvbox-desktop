import { defineStore } from 'pinia'
import { ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { SourceSubscription } from '@/types'

export const useSubscriptionStore = defineStore('subscription', () => {
  const subscriptions = ref<SourceSubscription[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)

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

  async function refreshSubscription(id: number) {
    try {
      await invoke('refresh_subscription', { id })
      await fetchSubscriptions()
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
    fetchSubscriptions,
    addSubscription,
    deleteSubscription,
    refreshSubscription,
    toggleSubscription
  }
})
