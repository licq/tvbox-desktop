import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { CatalogCard, CatalogCardInput, CatalogItemType, DoubanHot, HomePayloadInput } from '@/types'

function resolveItemType(card: CatalogCardInput) {
  const itemType = card.item_type ?? card.itemType
  if (!itemType) {
    throw new Error(`Catalog card item type is required for "${card.title}"`)
  }
  return itemType
}

function normalizeCatalogCard(card: CatalogCardInput): CatalogCard {
  return {
    id: card.id,
    title: card.title,
    item_type: resolveItemType(card),
    poster: card.poster,
    progress: card.progress,
    source_badge: card.source_badge ?? card.sourceBadge,
    update_badge: card.update_badge ?? card.updateBadge
  }
}

function normalizeCards(cards?: CatalogCardInput[]) {
  return (cards ?? []).map(card => normalizeCatalogCard(card))
}

function sliceRail(items: CatalogCard[], limit = 12) {
  return items.slice(0, limit)
}

function normalizeDoubanHot(items?: DoubanHot[]): DoubanHot[] {
  return items ?? []
}

export const useLibraryStore = defineStore('library', () => {
  const continueWatching = ref<CatalogCard[]>([])
  const latestUpdates = ref<CatalogCard[]>([])
  const featured = ref<CatalogCard[]>([])
  const doubanHot = ref<DoubanHot[]>([])
  const doubanHotByType = ref<Record<string, { items: DoubanHot[]; updated_at: string }>>({})
  const catalogItems = ref<CatalogCard[]>([])
  const availableTypes = ref<string[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const hero = computed(() => featured.value[0] ?? latestUpdates.value[0] ?? continueWatching.value[0] ?? null)

  function applyHomePayload(payload: HomePayloadInput) {
    continueWatching.value = normalizeCards(payload.continue_watching ?? payload.continueWatching)
    latestUpdates.value = normalizeCards(payload.latest_updates ?? payload.latestUpdates)
    featured.value = normalizeCards(payload.featured)
    doubanHot.value = normalizeDoubanHot(payload.douban_hot ?? payload.doubanHot)
  }

  async function fetchHome() {
    loading.value = true
    error.value = null
    try {
      const payload = await invoke<HomePayloadInput>('get_library_home')
      applyHomePayload(payload)
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  async function fetchCatalog(itemType?: string, keyword?: string) {
    loading.value = true
    error.value = null
    try {
      const payload = await invoke<CatalogCardInput[]>('get_catalog_items', {
        itemType: itemType || null,
        keyword: keyword || null
      })
      catalogItems.value = normalizeCards(payload)
      // Update availableTypes only on unfiltered fetch
      if (!itemType && !keyword) {
        const allTypes = new Set(payload.map(item => item.item_type))
        availableTypes.value = [...allTypes] as string[]
      }
    } catch (e) {
      error.value = String(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  function getRail(itemType: CatalogItemType) {
    return sliceRail(catalogItems.value.filter(card => card.item_type === itemType))
  }

  async function fetchDoubanHotByType(itemType: string): Promise<DoubanHot[]> {
    const cached = doubanHotByType.value[itemType]
    const isStale = cached && cached.updated_at
      ? Date.now() - Number(cached.updated_at) > 24 * 60 * 60 * 1000
      : true

    if (cached && !isStale) {
      return cached.items
    }

    try {
      const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType })
      doubanHotByType.value[itemType] = {
        items,
        updated_at: String(Date.now())
      }
      return items
    } catch {
      if (isStale) {
        fetchAllDoubanHot().catch(console.error)
      }
      return cached?.items ?? []
    }
  }

  async function fetchAllDoubanHot() {
    try {
      await invoke<DoubanHot[]>('fetch_all_douban_hot')
      for (const type of ['movie', 'series', 'variety', 'anime']) {
        const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType: type })
        doubanHotByType.value[type] = {
          items,
          updated_at: String(Date.now())
        }
      }
    } catch (e) {
      console.error('fetchAllDoubanHot failed:', e)
    }
  }

  function getDoubanHotByType(type: string): DoubanHot[] {
    return doubanHotByType.value[type]?.items ?? []
  }

  return {
    continueWatching,
    latestUpdates,
    featured,
    doubanHot,
    catalogItems,
    availableTypes,
    hero,
    loading,
    error,
    applyHomePayload,
    fetchHome,
    fetchCatalog,
    fetchDoubanHotByType,
    fetchAllDoubanHot,
    getDoubanHotByType,
    getRail
  }
})
