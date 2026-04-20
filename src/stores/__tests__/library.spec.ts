import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { useLibraryStore } from '@/stores/library'

describe('library store', () => {
  it('normalizes home payload into store-owned copies and replaces prior state', () => {
    setActivePinia(createPinia())
    const store = useLibraryStore()

    const firstContinueWatching = { id: 7, title: '继续看', item_type: 'tv', progress: 42 }
    const firstLatestUpdate = { id: 8, title: '最新更新', item_type: 'movie' }
    const firstFeatured = { id: 9, title: '推荐内容', item_type: 'anime' }

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
      continue_watching: [{ id: 10, title: '新内容', item_type: 'tv' }],
      latest_updates: [],
      featured: []
    })

    expect(store.continueWatching).toHaveLength(1)
    expect(store.continueWatching[0].id).toBe(10)
    expect(store.latestUpdates).toHaveLength(0)
    expect(store.featured).toHaveLength(0)

    store.applyHomePayload({
      continueWatching: [{ id: 11, title: '旧字段兼容', itemType: 'movie' }],
      latestUpdates: [{ id: 12, title: '旧字段最新', itemType: 'tv' }],
      featured: [{ id: 13, title: '旧字段精选', itemType: 'anime' }]
    })

    expect(store.continueWatching[0].item_type).toBe('movie')
    expect(store.latestUpdates[0].item_type).toBe('tv')
    expect(store.featured[0].item_type).toBe('anime')
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
})
