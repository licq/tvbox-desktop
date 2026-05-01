import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it, beforeEach } from 'vitest'
import { usePlayerStore } from '@/stores/player'
import type { CatalogDetail } from '@/types'

describe('player store pending playback detail', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('consumes pending playback detail once', () => {
    const store = usePlayerStore()
    const detail: CatalogDetail = {
      item: {
        id: 42,
        title: '测试剧集',
        item_type: 'series',
      },
      episode_groups: [
        {
          source_name: 'source-a',
          episodes: [
            { id: 1, episode_label: '第1集', play_url: 'https://example.com/1', order_index: 1 },
          ],
        },
      ],
    }

    store.setPendingPlaybackDetail(detail)

    expect(store.pendingPlaybackDetail).toEqual(detail)
    expect(store.takePendingPlaybackDetail()).toEqual(detail)
    expect(store.pendingPlaybackDetail).toBeNull()
    expect(store.takePendingPlaybackDetail()).toBeNull()
  })
})
