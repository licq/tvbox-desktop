import { describe, expect, it } from 'vitest'
import {
  clearVodDetailSearchSnapshot,
  getVodDetailSearchSnapshot,
  normalizeVodDetailSearchKey,
  setVodDetailSearchSnapshot,
  type VodDetailSearchSnapshot,
} from '@/utils/vodDetailSearchCache'

describe('vod detail search cache', () => {
  it('normalizes cache keys', () => {
    expect(normalizeVodDetailSearchKey('  黑夜告白  ')).toBe('黑夜告白')
  })

  it('stores and returns cloned snapshots', () => {
    clearVodDetailSearchSnapshot()

    const snapshot: VodDetailSearchSnapshot = {
      searchResults: [
        {
          source_name: 'Source A',
          results: [
            {
              source: 'demo',
              source_name: 'Source A',
              detail_url: 'detail-1',
              item_type: 'series',
              title: '黑夜告白',
            },
          ],
        },
      ],
      providerDetailEntries: [
        [
          '黑夜告白-demo',
          {
            title: '黑夜告白',
            poster: null,
            summary: 'cached',
            episodes: [
              {
                id: 1,
                episode_label: '第1集',
                play_url: 'https://example.com/ep1',
                order_index: 0,
              },
            ],
          },
        ],
      ],
    }

    setVodDetailSearchSnapshot('黑夜告白', snapshot)

    const cached = getVodDetailSearchSnapshot('  黑夜告白 ')
    expect(cached).not.toBeNull()
    expect(cached?.searchResults[0].results[0].title).toBe('黑夜告白')
    expect(cached?.providerDetailEntries[0][1].episodes[0].episode_label).toBe('第1集')

    snapshot.searchResults[0].results[0].title = 'mutated'
    snapshot.providerDetailEntries[0][1].episodes[0].episode_label = 'changed'

    const cachedAgain = getVodDetailSearchSnapshot('黑夜告白')
    expect(cachedAgain?.searchResults[0].results[0].title).toBe('黑夜告白')
    expect(cachedAgain?.providerDetailEntries[0][1].episodes[0].episode_label).toBe('第1集')
  })

  it('clears cached snapshots', () => {
    setVodDetailSearchSnapshot('黑夜告白', {
      searchResults: [],
      providerDetailEntries: [],
    })

    clearVodDetailSearchSnapshot('黑夜告白')

    expect(getVodDetailSearchSnapshot('黑夜告白')).toBeNull()
  })
})
