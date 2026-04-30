import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke } from '@tauri-apps/api/core'
import { useLibraryStore } from '@/stores/library'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}))

const invokeMock = vi.mocked(invoke)

describe('library store', () => {
  beforeEach(() => {
    invokeMock.mockReset()
  })

  it('normalizes home payload into store-owned copies and replaces prior state', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const firstContinueWatching = { id: 7, title: '继续看', item_type: 'series' as const, progress: 42 }
    const firstLatestUpdate = { id: 8, title: '最新更新', item_type: 'movie' as const }
    const firstFeatured = { id: 9, title: '推荐内容', item_type: 'anime' as const }

    store.applyHomePayload({
      continue_watching: [firstContinueWatching],
      latest_updates: [firstLatestUpdate],
      featured: [firstFeatured]
    })

    expect(store.continueWatching[0]).not.toBe(firstContinueWatching)
    expect(store.latestUpdates[0]).not.toBe(firstLatestUpdate)
    expect(store.featured[0]).not.toBe(firstFeatured)

    firstContinueWatching.title = '被外部修改'
    firstLatestUpdate.title = '被外部修改'
    firstFeatured.title = '被外部修改'

    expect(store.continueWatching[0].title).toBe('继续看')
    expect(store.latestUpdates[0].title).toBe('最新更新')
    expect(store.featured[0].title).toBe('推荐内容')

    store.applyHomePayload({
      continue_watching: [{ id: 10, title: '新内容', item_type: 'series' }],
      latest_updates: [],
      featured: []
    })

    expect(store.continueWatching).toHaveLength(1)
    expect(store.continueWatching[0].id).toBe(10)
    expect(store.latestUpdates).toHaveLength(0)
    expect(store.featured).toHaveLength(0)

    store.applyHomePayload({
      continueWatching: [{ id: 11, title: '旧字段兼容', itemType: 'movie' }],
      latestUpdates: [{ id: 12, title: '旧字段最新', itemType: 'series' }],
      featured: [{ id: 13, title: '旧字段精选', itemType: 'anime' }]
    })

    expect(store.continueWatching[0].item_type).toBe('movie')
    expect(store.latestUpdates[0].item_type).toBe('series')
    expect(store.featured[0].item_type).toBe('anime')
  })

  it('normalizes home payload into hero and rail-friendly card fields', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const payload = {
      continue_watching: [
        {
          id: 1,
          title: 'Arcane',
          item_type: 'series' as const,
          poster: 'https://img.test/arcane.jpg',
          progress: 52,
          source_badge: '荐片',
          update_badge: '继续观看'
        }
      ],
      latest_updates: [],
      featured: [
        {
          id: 2,
          title: 'Dune',
          item_type: 'movie' as const,
          poster: 'https://img.test/dune.jpg',
          sourceBadge: 'Auete',
          updateBadge: '推荐'
        }
      ]
    }

    store.applyHomePayload(payload)

    expect(store.continueWatching[0].item_type).toBe('series')
    expect(store.continueWatching[0].progress).toBe(52)
    expect(store.continueWatching[0].source_badge).toBe('荐片')
    expect(store.continueWatching[0].update_badge).toBe('继续观看')
    expect(store.featured[0].item_type).toBe('movie')
    expect(store.featured[0].source_badge).toBe('Auete')
    expect(store.featured[0].update_badge).toBe('推荐')
  })

  it('keeps featured card available as hero source and continue watching as separate rail', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const payload = {
      continue_watching: [
        { id: 1, title: 'Arcane', item_type: 'series' as const, progress: 40 }
      ],
      latest_updates: [
        { id: 2, title: 'The Bear', item_type: 'series' as const }
      ],
      featured: [
        { id: 3, title: 'Dune', item_type: 'movie' as const }
      ]
    }

    store.applyHomePayload(payload)
    store.catalogItems = [
      ...store.continueWatching,
      ...store.latestUpdates,
      ...store.featured,
      { id: 4, title: 'Series Rail', item_type: 'series' },
      { id: 5, title: 'Series Rail 2', item_type: 'series' },
      { id: 6, title: 'Series Rail 3', item_type: 'series' },
      { id: 7, title: 'Series Rail 4', item_type: 'series' },
      { id: 8, title: 'Series Rail 5', item_type: 'series' },
      { id: 9, title: 'Series Rail 6', item_type: 'series' },
      { id: 10, title: 'Series Rail 7', item_type: 'series' },
      { id: 11, title: 'Series Rail 8', item_type: 'series' },
      { id: 12, title: 'Series Rail 9', item_type: 'series' },
      { id: 13, title: 'Series Rail 10', item_type: 'series' },
      { id: 14, title: 'Series Rail 11', item_type: 'series' },
      { id: 15, title: 'Series Rail 12', item_type: 'series' },
      { id: 16, title: 'Series Rail 13', item_type: 'series' }
    ]

    expect(store.hero?.title).toBe('Dune')
    expect(store.continueWatching[0].title).toBe('Arcane')
    expect(store.latestUpdates[0].title).toBe('The Bear')
    expect(store.getRail('series')).toHaveLength(12)
    expect(store.getRail('series')[0].title).toBe('Arcane')
    expect(store.getRail('series')[1].title).toBe('The Bear')
  })

  it('rejects malformed cards without an item type', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    expect(() =>
      store.applyHomePayload({
        continue_watching: [{ id: 14, title: '坏数据' } as never],
        latest_updates: [],
        featured: []
      })
    ).toThrow('Catalog card item type is required')
  })

  it('hydrates the home payload without forcing douban hot batch refresh', async () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    let homeFetchCount = 0
    invokeMock.mockImplementation(async (command: string) => {
      switch (command) {
        case 'get_library_home':
          homeFetchCount += 1
          return { continue_watching: [], latest_updates: [], featured: [], douban_hot: [] }
        default:
          return []
      }
    })

    await store.hydrateHome()

    expect(homeFetchCount).toBe(1)
    expect(store.doubanHot).toHaveLength(0)
  })

  it('fetches douban hot by type on demand and caches the result', async () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const hotItems = [
      {
        id: 1,
        name: 'Movie A',
        year: 2025,
        poster: null,
        rating: 9.1,
        rank: 1,
        updated_at: '2026-04-30 10:00:00',
        item_type: 'movie' as const
      }
    ]

    invokeMock.mockImplementation(async (command: string, args?: unknown) => {
      const itemType = typeof args === 'object' && args !== null && 'itemType' in args
        ? (args as { itemType?: string }).itemType
        : undefined

      switch (command) {
        case 'get_douban_hot_by_type':
          return itemType === 'movie' ? hotItems : []
        default:
          return []
      }
    })

    const items = await store.fetchDoubanHotByType('movie')

    expect(items).toEqual(hotItems)
    expect(store.getDoubanHotByType('movie')).toEqual(hotItems)
    expect(store.doubanHot).toEqual(hotItems)
  })

  it('does not cache empty douban hot responses as fresh data', async () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    let callCount = 0
    invokeMock.mockImplementation(async (command: string) => {
      if (command === 'get_douban_hot_by_type') {
        callCount += 1
        return []
      }
      return []
    })

    await store.fetchDoubanHotByType('movie')
    await store.fetchDoubanHotByType('movie')

    expect(callCount).toBe(2)
    expect(store.getDoubanHotByType('movie')).toHaveLength(0)
  })

  it('deduplicates concurrent douban hot requests by type', async () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    let callCount = 0
    invokeMock.mockImplementation(async (command: string, args?: unknown) => {
      if (command !== 'get_douban_hot_by_type') {
        return []
      }

      callCount += 1
      const itemType = typeof args === 'object' && args !== null && 'itemType' in args
        ? (args as { itemType?: string }).itemType
        : undefined

      return itemType === 'movie'
        ? [
            {
              id: 1,
              name: 'Movie A',
              year: 2025,
              poster: null,
              rating: 9.1,
              rank: 1,
              updated_at: '2026-04-30 10:00:00',
              item_type: 'movie' as const
            }
          ]
        : []
    })

    const [first, second] = await Promise.all([
      store.fetchDoubanHotByType('movie'),
      store.fetchDoubanHotByType('movie')
    ])

    expect(callCount).toBe(1)
    expect(first).toEqual(second)
    expect(store.getDoubanHotByType('movie')).toHaveLength(1)
  })

  it('deduplicates concurrent douban hot refreshes', async () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const hotItems = [
      {
        id: 1,
        name: 'Movie A',
        year: 2025,
        poster: null,
        rating: 9.1,
        rank: 1,
        updated_at: '2026-04-30 10:00:00',
        item_type: 'movie' as const
      }
    ]

    let fetchAllCount = 0
    invokeMock.mockImplementation(async (command: string, args?: unknown) => {
      const itemType = typeof args === 'object' && args !== null && 'itemType' in args
        ? (args as { itemType?: string }).itemType
        : undefined

      switch (command) {
        case 'fetch_all_douban_hot':
          fetchAllCount += 1
          return hotItems
        case 'get_douban_hot':
          return hotItems
        case 'get_douban_hot_by_type':
          return itemType === 'movie' ? hotItems : []
        default:
          return []
      }
    })

    await Promise.all([store.fetchAllDoubanHot(), store.fetchAllDoubanHot()])

    expect(fetchAllCount).toBe(1)
    expect(store.doubanHot).toEqual(hotItems)
  })
})
