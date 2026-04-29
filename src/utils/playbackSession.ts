import type {
  PlaybackCandidate,
  PlaybackSourceAttempt,
  PlaybackSourceAttemptStatus,
  UnifiedEpisode,
} from '@/types'

type HealthStatus = 'success' | 'failed'
type HealthScope = 'source' | 'candidate'

interface PlaybackHealthEntry {
  scope: HealthScope
  key: string
  status: HealthStatus
  reason?: string
  checkedAt: number
}

export interface PlaybackHealthInput {
  scope: HealthScope
  key: string
  status: HealthStatus
  reason?: string
}

export interface EpisodePlaybackSession {
  episode: UnifiedEpisode
  sourceAttempts: PlaybackSourceAttempt[]
  activeSourceIndex: number
  activeCandidateIndex: number
  status: 'idle' | 'resolving' | 'playing' | 'failed'
  lastError?: string
}

const playbackHealth = new Map<string, PlaybackHealthEntry>()

function healthMapKey(scope: HealthScope, key: string) {
  return `${scope}:${key}`
}

export function sourceHealthKey(sourceKey: string, playUrl: string) {
  return `${sourceKey}|${playUrl}`
}

export function candidateHealthKey(candidate: PlaybackCandidate) {
  const headers = candidate.headers ? JSON.stringify(candidate.headers) : ''
  return `${candidate.url}|${candidate.referer ?? ''}|${headers}`
}

export function markPlaybackHealth(input: PlaybackHealthInput) {
  playbackHealth.set(healthMapKey(input.scope, input.key), {
    ...input,
    checkedAt: Date.now(),
  })
}

export function getPlaybackHealth(scope: HealthScope, key: string) {
  return playbackHealth.get(healthMapKey(scope, key)) ?? null
}

export function clearPlaybackHealth() {
  playbackHealth.clear()
}

function sourceRank(attempt: PlaybackSourceAttempt) {
  const key = sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url)
  const health = getPlaybackHealth('source', key)
  if (health?.status === 'success') return 0
  if (health?.status === 'failed') return 2
  return 1
}

function statusForSource(sourceKey: string, playUrl: string): PlaybackSourceAttemptStatus {
  const health = getPlaybackHealth('source', sourceHealthKey(sourceKey, playUrl))
  return health?.status === 'failed' ? 'skipped' : 'idle'
}

export function createEpisodePlaybackSession(episode: UnifiedEpisode): EpisodePlaybackSession {
  const sourceAttempts = episode.sources
    .map<PlaybackSourceAttempt>(source => ({
      source,
      status: statusForSource(source.sourceKey, source.episode.play_url),
      candidates: [],
      failedCandidateIndexes: [],
      failureReason: getPlaybackHealth('source', sourceHealthKey(source.sourceKey, source.episode.play_url))?.reason,
    }))
    .map((attempt, originalIndex) => ({ attempt, originalIndex }))
    .sort((a, b) => sourceRank(a.attempt) - sourceRank(b.attempt) || a.originalIndex - b.originalIndex)
    .map(({ attempt }) => attempt)

  return {
    episode,
    sourceAttempts,
    activeSourceIndex: -1,
    activeCandidateIndex: -1,
    status: 'idle',
  }
}
