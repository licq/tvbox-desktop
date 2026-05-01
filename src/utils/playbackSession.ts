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
  id: number
  episode: UnifiedEpisode
  sourceAttempts: PlaybackSourceAttempt[]
  activeSourceIndex: number
  activeCandidateIndex: number
  status: 'idle' | 'resolving' | 'playing' | 'failed'
  lastError?: string
}

const playbackHealth = new Map<string, PlaybackHealthEntry>()
let playbackSessionId = 0

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
    id: ++playbackSessionId,
    episode,
    sourceAttempts,
    activeSourceIndex: -1,
    activeCandidateIndex: -1,
    status: 'idle',
  }
}

export interface StartSourceOptions {
  sourceKey?: string
  playUrl?: string
  manual?: boolean
}

export function startNextSourceAttempt(
  session: EpisodePlaybackSession,
  options: StartSourceOptions = {}
) {
  const index = options.playUrl
    ? session.sourceAttempts.findIndex(attempt =>
        attempt.source.episode.play_url === options.playUrl &&
        (!options.sourceKey || attempt.source.sourceKey === options.sourceKey)
      )
    : options.sourceKey
      ? session.sourceAttempts.findIndex(attempt => attempt.source.sourceKey === options.sourceKey)
    : session.sourceAttempts.findIndex((attempt, attemptIndex) =>
        attemptIndex > session.activeSourceIndex &&
        (options.manual || attempt.status !== 'failed') &&
        (options.manual || attempt.status !== 'skipped')
      )

  if (index < 0) {
    session.status = 'failed'
    session.lastError = session.lastError ?? '该集所有播放源均不可用'
    return null
  }

  const attempt = session.sourceAttempts[index]
  if (!attempt) return null

  session.activeSourceIndex = index
  session.activeCandidateIndex = -1
  session.status = 'resolving'
  attempt.status = 'resolving'
  attempt.lastTriedAt = Date.now()
  return attempt
}

export function attachCandidatesToActiveSource(
  session: EpisodePlaybackSession,
  candidates: PlaybackCandidate[]
) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return

  attempt.candidates = candidates
  attempt.failedCandidateIndexes = []
  attempt.status = candidates.length > 0 ? 'playable' : 'failed'
  if (candidates.length === 0) {
    attempt.failureReason = '当前源没有可用候选线路'
  }
}

export function nextCandidateToPlay(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return null

  const nextIndex = attempt.candidates.findIndex((_, index) =>
    index > session.activeCandidateIndex && !attempt.failedCandidateIndexes.includes(index)
  )

  if (nextIndex < 0) return null

  session.activeCandidateIndex = nextIndex
  session.status = 'playing'
  attempt.status = 'playing'
  return attempt.candidates[nextIndex] ?? null
}

export function markCurrentCandidateFailed(session: EpisodePlaybackSession, reason: string) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) return

  if (
    session.activeCandidateIndex >= 0 &&
    !attempt.failedCandidateIndexes.includes(session.activeCandidateIndex)
  ) {
    attempt.failedCandidateIndexes = [...attempt.failedCandidateIndexes, session.activeCandidateIndex]
    const candidate = attempt.candidates[session.activeCandidateIndex]
    if (candidate) {
      markPlaybackHealth({
        scope: 'candidate',
        key: candidateHealthKey(candidate),
        status: 'failed',
        reason,
      })
    }
  }

  const hasRemainingCandidate = attempt.candidates.some((_, index) =>
    !attempt.failedCandidateIndexes.includes(index)
  )

  if (!hasRemainingCandidate) {
    attempt.status = 'failed'
    attempt.failureReason = reason
    session.lastError = reason
    markPlaybackHealth({
      scope: 'source',
      key: sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url),
      status: 'failed',
      reason,
    })
  }
}

export function markCurrentCandidatePlaying(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  const candidate = attempt?.candidates[session.activeCandidateIndex]
  if (!attempt || !candidate) return

  session.status = 'playing'
  attempt.status = 'playing'
  markPlaybackHealth({
    scope: 'source',
    key: sourceHealthKey(attempt.source.sourceKey, attempt.source.episode.play_url),
    status: 'success',
  })
  markPlaybackHealth({
    scope: 'candidate',
    key: candidateHealthKey(candidate),
    status: 'success',
  })
}

export function shouldFailoverAfterPlaybackError(error: unknown) {
  const name = typeof error === 'object' && error !== null && 'name' in error
    ? String((error as { name?: unknown }).name)
    : ''
  const message = error instanceof Error ? error.message : String(error)
  return !name.includes('NotAllowedError') && !message.includes('NotAllowedError')
}
