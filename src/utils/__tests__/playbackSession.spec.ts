import { describe, expect, it, beforeEach } from 'vitest'
import type { UnifiedEpisode } from '@/types'
import {
  createEpisodePlaybackSession,
  clearPlaybackHealth,
  markPlaybackHealth,
} from '@/utils/playbackSession'

function episode(): UnifiedEpisode {
  return {
    normalizedIndex: 3,
    displayLabel: '第3集',
    sources: [
      {
        sourceKey: 'slow',
        sourceName: '慢线路',
        episode: { id: 31, episode_label: '第03集', play_url: 'https://slow.example/play', order_index: 0 },
      },
      {
        sourceKey: 'fast',
        sourceName: '快线路',
        episode: { id: 32, episode_label: '第03集', play_url: 'https://fast.example/play', order_index: 1 },
      },
      {
        sourceKey: 'bad',
        sourceName: '坏线路',
        episode: { id: 33, episode_label: '第03集', play_url: 'https://bad.example/play', order_index: 2 },
      },
    ],
  }
}

describe('playback session', () => {
  beforeEach(() => clearPlaybackHealth())

  it('orders recently successful sources before unknown and failed sources', () => {
    const ep = episode()
    markPlaybackHealth({ scope: 'source', key: 'fast|https://fast.example/play', status: 'success' })
    markPlaybackHealth({ scope: 'source', key: 'bad|https://bad.example/play', status: 'failed', reason: 'manifest failed' })

    const session = createEpisodePlaybackSession(ep)

    expect(session.sourceAttempts.map(attempt => attempt.source.sourceKey)).toEqual(['fast', 'slow', 'bad'])
    expect(session.sourceAttempts[0]?.status).toBe('idle')
    expect(session.sourceAttempts[2]?.status).toBe('skipped')
  })

  it('keeps original source order when no health is known', () => {
    const session = createEpisodePlaybackSession(episode())

    expect(session.sourceAttempts.map(attempt => attempt.source.sourceName)).toEqual(['慢线路', '快线路', '坏线路'])
  })
})
