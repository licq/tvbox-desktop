import { describe, it, expect } from 'vitest'
import { extractEpisodeIndex, mergeEpisodes, formatDisplayLabel } from '../episode'
import type { CatalogEpisodeGroup } from '@/types'

describe('extractEpisodeIndex', () => {
  it('extracts from 第1集', () => expect(extractEpisodeIndex('第1集')).toBe(1))
  it('extracts from 第01集', () => expect(extractEpisodeIndex('第01集')).toBe(1))
  it('extracts from 第1期', () => expect(extractEpisodeIndex('第1期')).toBe(1))
  it('extracts from S01E01', () => expect(extractEpisodeIndex('S01E01')).toBe(1))
  it('extracts from E01', () => expect(extractEpisodeIndex('E01')).toBe(1))
  it('extracts from pure number 01', () => expect(extractEpisodeIndex('01')).toBe(1))
  it('returns null for HD', () => expect(extractEpisodeIndex('HD')).toBeNull())
  it('returns null for 蓝光', () => expect(extractEpisodeIndex('蓝光')).toBeNull())
})

describe('mergeEpisodes', () => {
  const groups: CatalogEpisodeGroup[] = [
    {
      source_name: 'A',
      episodes: [
        { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
        { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
      ],
    },
    {
      source_name: 'B',
      episodes: [
        { id: 3, episode_label: '第01集', play_url: 'http://b/1', order_index: 1 },
        { id: 4, episode_label: '第02集', play_url: 'http://b/2', order_index: 2 },
      ],
    },
  ]

  it('merges duplicate episodes for series', () => {
    const result = mergeEpisodes(groups, 'series')
    expect(result).toHaveLength(2)
    expect(result[0].normalizedIndex).toBe(1)
    expect(result[0].sources).toHaveLength(2)
    expect(result[1].normalizedIndex).toBe(2)
  })

  it('sorts by normalizedIndex ascending', () => {
    const shuffled: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
          { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
        ],
      },
    ]
    const result = mergeEpisodes(shuffled, 'series')
    expect(result[0].normalizedIndex).toBe(1)
    expect(result[1].normalizedIndex).toBe(2)
  })

  it('does not merge for movies', () => {
    const movieGroups: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 1, episode_label: 'HD', play_url: 'http://a/hd', order_index: 1 },
          { id: 2, episode_label: '1080P', play_url: 'http://a/1080', order_index: 2 },
        ],
      },
    ]
    const result = mergeEpisodes(movieGroups, 'movie')
    expect(result).toHaveLength(2)
    expect(result[0].sources).toHaveLength(1)
  })

  it('treats unnormalizable labels as independent items', () => {
    const g: CatalogEpisodeGroup[] = [
      {
        source_name: 'A',
        episodes: [
          { id: 1, episode_label: '预告片', play_url: 'http://a/trailer', order_index: 1 },
        ],
      },
    ]
    const result = mergeEpisodes(g, 'series')
    expect(result).toHaveLength(1)
    expect(result[0].displayLabel).toBe('预告片')
  })
})

describe('formatDisplayLabel', () => {
  it('formats 第1集', () => expect(formatDisplayLabel('第1集')).toBe('第1集'))
  it('formats 第01集', () => expect(formatDisplayLabel('第01集')).toBe('第1集'))
  it('formats S01E01', () => expect(formatDisplayLabel('S01E01')).toBe('第1集'))
  it('formats 第1期 for variety', () => expect(formatDisplayLabel('第1期', 'variety')).toBe('第1期'))
  it('returns original for HD', () => expect(formatDisplayLabel('HD')).toBe('HD'))
})
