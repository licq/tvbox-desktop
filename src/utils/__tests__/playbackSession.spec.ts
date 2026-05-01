import { describe, expect, it, beforeEach } from 'vitest'
import type { PlaybackCandidate, UnifiedEpisode } from '@/types'
import {
  attachCandidatesToActiveSource,
  createEpisodePlaybackSession,
  clearPlaybackHealth,
  markCurrentCandidateFailed,
  markPlaybackHealth,
  nextCandidateToPlay,
  startNextSourceAttempt,
  shouldFailoverAfterPlaybackError,
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

const candidates = [
  { url: 'https://cdn.example/a.m3u8', label: '候选1', kind: 'hls' },
  { url: 'https://cdn.example/b.m3u8', label: '候选2', kind: 'hls' },
] satisfies PlaybackCandidate[]

describe('playback session advancement', () => {
  beforeEach(() => clearPlaybackHealth())

  it('tries another candidate in the same source before moving to the next source', () => {
    const session = createEpisodePlaybackSession(episode())
    const firstAttempt = startNextSourceAttempt(session)
    expect(firstAttempt?.source.sourceKey).toBe('slow')

    attachCandidatesToActiveSource(session, candidates)
    expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/a.m3u8')

    markCurrentCandidateFailed(session, 'manifest failed')
    expect(nextCandidateToPlay(session)?.url).toBe('https://cdn.example/b.m3u8')

    markCurrentCandidateFailed(session, 'segment failed')
    const nextAttempt = startNextSourceAttempt(session)
    expect(nextAttempt?.source.sourceKey).toBe('fast')
  })

  it('does not fail over for autoplay blocking', () => {
    expect(shouldFailoverAfterPlaybackError({ name: 'NotAllowedError' })).toBe(false)
    expect(shouldFailoverAfterPlaybackError(new Error('NotAllowedError: play() failed'))).toBe(false)
    expect(shouldFailoverAfterPlaybackError(new Error('media decode failed'))).toBe(true)
  })

  it('allows manual attempts for a skipped failed source', () => {
    const ep = episode()
    markPlaybackHealth({ scope: 'source', key: 'bad|https://bad.example/play', status: 'failed', reason: 'previous failure' })
    const session = createEpisodePlaybackSession(ep)

    const manual = startNextSourceAttempt(session, { sourceKey: 'bad', manual: true })

    expect(manual?.source.sourceKey).toBe('bad')
    expect(manual?.status).toBe('resolving')
  })

  it('prefers the exact playUrl when multiple attempts share the same sourceKey', () => {
    const ep: UnifiedEpisode = {
      normalizedIndex: 4,
      displayLabel: '第4集',
      sources: [
        {
          sourceKey: 'same',
          sourceName: '同源A',
          episode: { id: 41, episode_label: '第04集', play_url: 'https://same.example/a', order_index: 0 },
        },
        {
          sourceKey: 'same',
          sourceName: '同源B',
          episode: { id: 42, episode_label: '第04集', play_url: 'https://same.example/b', order_index: 1 },
        },
      ],
    }

    const session = createEpisodePlaybackSession(ep)
    const attempt = startNextSourceAttempt(session, {
      sourceKey: 'same',
      playUrl: 'https://same.example/b',
      manual: true,
    })

    expect(attempt?.source.episode.play_url).toBe('https://same.example/b')
  })
})
