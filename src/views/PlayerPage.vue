<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { open } from '@tauri-apps/plugin-shell'
import { invoke } from '@tauri-apps/api/core'
import { useLiveStore } from '@/stores/live'
import { usePlayerStore } from '@/stores/player'
import { usePlaybackStore } from '@/stores/playback'
import { useDetailStore } from '@/stores/detail'
import PlaybackDrawer from '@/components/player/PlaybackDrawer.vue'
import type { CatalogEpisodeGroup, PlaybackTarget, UnifiedEpisode } from '@/types'
import PlaybackNotice from '@/components/player/PlaybackNotice.vue'
import { describeMediaErrorCode, describePlaybackFailure, formatPlayerTitle, isAutoplayBlocked } from '@/utils/player'
import { mergeEpisodes } from '@/utils/episode'
import {
  createEpisodePlaybackSession,
  markCurrentCandidateFailed,
  markCurrentCandidatePlaying,
  nextCandidateToPlay,
  shouldFailoverAfterPlaybackError,
  startNextSourceAttempt,
  type EpisodePlaybackSession,
} from '@/utils/playbackSession'
import type Hls from 'hls.js'

const route = useRoute()
const router = useRouter()
const liveStore = useLiveStore()
const playerStore = usePlayerStore()
const playbackStore = usePlaybackStore()
const detailStore = useDetailStore()
const activeGroup = ref<CatalogEpisodeGroup | null>(null)

type PlayerSource = {
  url: string
  label: string
  kind: 'hls' | 'http' | 'external' | 'embed'
  headers?: Record<string, string>
  referer?: string
}

type PlaybackAttemptContext = {
  id: number
  generation: number
  url: string
  startedAt: number
}

const videoRef = ref<HTMLVideoElement | null>(null)
const videoWrapRef = ref<HTMLElement | null>(null)
const playing = ref(false)
const currentTime = ref(0)
const duration = ref(0)
const volume = ref(1)
const fullscreen = ref(false)
const fullscreenError = ref('')
const errorMsg = ref('')
const pendingAutoplay = ref(false)

const sources = ref<PlayerSource[]>([])
const currentSourceIndex = ref(0)
const failedSourceIndexes = ref<number[]>([])
const currentUnifiedEpisode = ref<UnifiedEpisode | null>(null)
const currentUnifiedSourceIndex = ref(0)
const playbackSession = ref<EpisodePlaybackSession | null>(null)
const currentEpisodeSourceAttempts = computed(() => playbackSession.value?.sourceAttempts ?? [])
let sessionFailoverPromise: Promise<void> | null = null
let lastSessionFailureKey: string | null = null
let sessionGeneration = 0
let playbackAttemptId = 0
let activePlaybackAttempt: PlaybackAttemptContext | null = null
let removeNativeVideoErrorHandler: (() => void) | null = null

const currentSource = computed(() => sources.value[currentSourceIndex.value] ?? null)
const mode = computed(() => String(route.params.mode ?? 'live'))
const itemId = computed(() => Number(route.params.id))
const sourceDetailUrl = computed(() => route.params.detailUrl as string | undefined)
const sourceName = computed(() => route.query.source as string | undefined)
const sourceTitle = computed(() => route.query.title as string | undefined)
const episodeUrl = computed(() => {
  const value = route.query.episode
  return typeof value === 'string' ? value : null
})
const episodeId = computed(() => {
  const value = route.query.episodeId
  const numeric = typeof value === 'string' ? Number(value) : NaN
  return Number.isFinite(numeric) && numeric > 0 ? numeric : undefined
})
const providerDetailUrl = computed(() => route.query.detailUrl as string | undefined)
const episodeLabelFromQuery = computed(() => route.query.episodeLabel as string | undefined)
const sourceLabel = computed(() => currentSource.value?.label ?? `线路 ${currentSourceIndex.value + 1}`)
const itemType = computed(() => {
  if (detailStore.item?.item_type) return detailStore.item.item_type
  // Infer from activeGroup / unifiedEpisodes when detailStore.item is not yet loaded
  if (activeGroup.value && activeGroup.value.episodes.length > 1) return 'series'
  if (unifiedEpisodes.value.length > 1) return 'series'
  return 'movie'
})

const currentEpisodeLabel = computed(() => {
  if (currentUnifiedEpisode.value?.displayLabel) return currentUnifiedEpisode.value.displayLabel
  return episodeLabelFromQuery.value ?? null
})

const pageTitle = computed(() =>
  formatPlayerTitle({
    title: detailStore.item?.title ?? sourceTitle.value ?? null,
    episodeLabel: currentEpisodeLabel.value,
    sourceLabel: currentSource.value?.label ?? null,
  })
)

watch(pageTitle, title => {
  document.title = title
}, { immediate: true })

const unifiedEpisodes = computed(() => {
  if (detailStore.episodeGroups.length > 0 && detailStore.item) {
    return mergeEpisodes(detailStore.episodeGroups, detailStore.item.item_type)
  }
  if (activeGroup.value) {
    return mergeEpisodes([activeGroup.value], 'series')
  }
  return []
})

const currentNormalizedIndex = computed(() => {
  if (episodeId.value) {
    const ue = unifiedEpisodes.value.find(u =>
      u.sources.some(s => s.episode.id === episodeId.value)
    )
    return ue?.normalizedIndex
  }
  if (episodeLabelFromQuery.value) {
    const ue = unifiedEpisodes.value.find(u =>
      u.sources.some(s => s.episode.episode_label === episodeLabelFromQuery.value)
    )
    return ue?.normalizedIndex
  }
  return undefined
})

async function loadSourceDetail() {
  playbackSession.value = null
  invalidateSessionFailover()
  if (!sourceDetailUrl.value || !sourceName.value) {
    errorMsg.value = '缺少播放地址参数'
    return
  }
  try {
    const playUrl = await invoke<string>('play_from_source_detail', {
      detailUrl: sourceDetailUrl.value,
      source: sourceName.value,
    })
    // For jpvod/jianpian, the returned URL is a play page that needs resolution
    // Pass through playbackStore to get actual stream URL
    if (sourceName.value === 'jpvod' || sourceName.value === 'jianpian') {
      const resolved = await playbackStore.resolve(playUrl, undefined)
      if (resolved.candidates.length > 0) {
        sources.value = resolved.candidates.map(c => ({
          url: c.url,
          label: c.label,
          kind: c.kind,
          referer: c.referer
        }))
        currentSourceIndex.value = 0
        if (resolved.status === 'ready' || resolved.status === 'external_required') {
          await playSource(sources.value[0])
        } else {
          errorMsg.value = resolved.errorMessage ?? '当前条目没有可用线路'
        }
      } else {
        errorMsg.value = resolved.errorMessage ?? '当前条目没有可用线路'
      }
    } else {
      // Direct stream URL for other sources
      sources.value = [{
        url: playUrl,
        label: sourceTitle.value || sourceName.value || '来源',
        kind: playUrl.includes('.m3u8') ? 'hls' : 'http'
      }]
      currentSourceIndex.value = 0
      await playSource(sources.value[0])
    }
  } catch (e) {
    errorMsg.value = `加载播放地址失败: ${e}`
  }
}

async function loadProviderEpisodes() {
  if (!providerDetailUrl.value || !sourceName.value) return
  try {
    const detail = await invoke<{ title: string | null; poster: string | null; summary: string | null; episodes: Array<{ episode_label: string; play_url: string; order_index: number }> }>('provider_detail', {
      source: sourceName.value,
      ids: providerDetailUrl.value,
    })
    activeGroup.value = {
      source_name: sourceName.value,
      episodes: detail.episodes.map((ep, index) => ({
        id: index + 1,
        episode_label: ep.episode_label,
        play_url: ep.play_url,
        order_index: ep.order_index,
      })),
    }
  } catch (e) {
    console.error('[PlayerPage] loadProviderEpisodes failed:', e)
  }
}
const playerStatusText = computed(() => {
  if (errorMsg.value) return '需要处理'
  if (pendingAutoplay.value) return '等待播放'
  if (playing.value) return '播放中'
  if (currentEpisodeSourceAttempts.value.some(attempt => attempt.status === 'resolving')) return '解析中'
  if (playbackStore.status === 'resolving') return '解析中'
  return playbackStore.status === 'idle' ? '就绪' : playbackStore.status
})
const playerModeLabel = computed(() => mode.value === 'live' ? '直播' : '点播')
const noticeTone = computed(() => playbackStore.status === 'failed' ? 'danger' : 'warning')

let hlsInstance: Hls | null = null
let hlsConstructorPromise: Promise<typeof import('hls.js').default> | null = null
let progressUpdateInterval: number | null = null
let fullscreenChangeHandler: (() => void) | null = null

const controlsVisible = ref(true)
let hideTimer: number | null = null

function startHideTimer() {
  if (hideTimer) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
  if (playing.value) {
    hideTimer = window.setTimeout(() => {
      controlsVisible.value = false
    }, 3000)
  }
}

function showControls() {
  controlsVisible.value = true
  startHideTimer()
}

function handleUserInteraction() {
  showControls()
}

onMounted(async () => {
  if (route.name === 'player-source') {
    await loadSourceDetail()
  } else if (mode.value === 'live') {
    playbackSession.value = null
    invalidateSessionFailover()
    await liveStore.fetchChannels()
    const channel = liveStore.channels.find(channel => channel.id === itemId.value)
    if (channel && channel.sources.length > 0) {
      sources.value = channel.sources.map((source, index) => ({
        url: source.url,
        label: `直播线路 ${index + 1}`,
        kind: source.url.includes('.m3u8') ? 'hls' : 'http'
      }))
      currentSourceIndex.value = 0
      await playSource(sources.value[0])
    } else {
      errorMsg.value = '当前频道没有可用线路'
    }
  } else if (mode.value === 'vod') {
    const pending = playerStore.pendingUnifiedEpisode
    if (pending) {
      playerStore.setPendingUnifiedEpisode(null)
      try {
        await playUnifiedEpisode(pending)
      } catch (e) {
        console.error('[PlayerPage] playUnifiedEpisode failed:', e)
      }
    } else if (episodeUrl.value) {
      try {
        await initVodPlayback(episodeUrl.value, episodeId.value)
      } catch (e) {
        console.error('[PlayerPage] initVodPlayback failed:', e)
      }
    }

    if (itemId.value) {
      try {
        await detailStore.fetchDetail(itemId.value)
        const group = detailStore.episodeGroups.find(g =>
          g.episodes.some(e => e.id === episodeId.value)
        )
        activeGroup.value = group ?? null
      } catch {
        activeGroup.value = null
      }
    } else if (providerDetailUrl.value && sourceName.value) {
      await loadProviderEpisodes()
    }
  } else {
    errorMsg.value = '缺少播放地址'
  }

  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }

  progressUpdateInterval = window.setInterval(() => {
    if (!videoRef.value) return
    currentTime.value = videoRef.value.currentTime
    duration.value = videoRef.value.duration || 0
  }, 1000)

  fullscreenChangeHandler = () => {
    fullscreen.value = !!document.fullscreenElement
  }
  document.addEventListener('fullscreenchange', fullscreenChangeHandler)
})

onUnmounted(() => {
  if (progressUpdateInterval) {
    window.clearInterval(progressUpdateInterval)
  }
  if (fullscreenChangeHandler) {
    document.removeEventListener('fullscreenchange', fullscreenChangeHandler)
  }
  if (hideTimer) {
    window.clearTimeout(hideTimer)
  }
  detachNativeVideoErrorHandler()
  activePlaybackAttempt = null

  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  if (mode.value === 'vod' && duration.value > 0) {
    const progress = (currentTime.value / duration.value) * 100
    void playerStore.saveHistory('vod', itemId.value, progress)
  }
})

function togglePlay() {
  if (!videoRef.value) return

  if (playing.value) {
    videoRef.value.pause()
    playing.value = false
    return
  }

  void attemptPlayback(true)
  handleUserInteraction()
}

function seek(time: number) {
  if (!videoRef.value) return
  videoRef.value.currentTime = time
  handleUserInteraction()
}

function handleVolumeChange(event: Event) {
  const target = event.target as HTMLInputElement
  volume.value = parseFloat(target.value)
  if (videoRef.value) {
    videoRef.value.volume = volume.value
  }
}

async function toggleFullscreen() {
  const video = videoRef.value
  const wrap = videoWrapRef.value
  if (!video || !wrap) return

  if (!fullscreen.value) {
    // 进入全屏：视频容器覆盖整个窗口，隐藏其他 UI
    fullscreen.value = true
    wrap.style.position = 'fixed'
    wrap.style.inset = '0'
    wrap.style.zIndex = '9999'
    wrap.style.background = '#000'
    wrap.style.width = '100vw'
    wrap.style.height = '100vh'

    // 隐藏顶部栏和右侧抽屉
    const topbar = document.querySelector('.player-topbar') as HTMLElement
    if (topbar) topbar.style.visibility = 'hidden'
    const drawer = document.querySelector('.player-stage > :last-child') as HTMLElement
    if (drawer && drawer.classList.contains('player-drawer')) drawer.style.visibility = 'hidden'
    document.body.style.overflow = 'hidden'

    // 确保视频全屏显示
    video.style.width = '100%'
    video.style.height = '100%'

    fullscreenError.value = ''
  } else {
    // 退出全屏：恢复原有样式
    fullscreen.value = false
    wrap.style.position = ''
    wrap.style.inset = ''
    wrap.style.zIndex = ''
    wrap.style.background = ''
    wrap.style.width = ''
    wrap.style.height = ''

    const topbar = document.querySelector('.player-topbar') as HTMLElement
    if (topbar) topbar.style.visibility = ''
    const drawer = document.querySelector('.player-stage > :last-child') as HTMLElement
    if (drawer && drawer.classList.contains('player-drawer')) drawer.style.visibility = ''
    document.body.style.overflow = ''

    video.style.width = ''
    video.style.height = ''
  }
}

function setVolume(v: number) {
  volume.value = v
  if (videoRef.value) videoRef.value.volume = v
}

function toggleMute() {
  if (videoRef.value) {
    videoRef.value.muted = !videoRef.value.muted
  }
}

function handleKeydown(e: KeyboardEvent) {
  // 忽略在 input/select 等元素上的按键
  if (['INPUT', 'SELECT', 'TEXTAREA'].includes((e.target as Element)?.tagName)) return

  switch (e.key) {
    case ' ':
    case 'k':
    case 'K':
      e.preventDefault()
      togglePlay()
      break
    case 'j':
    case 'J':
      e.preventDefault()
      seek(Math.max(0, currentTime.value - 10))
      break
    case 'l':
    case 'L':
      e.preventDefault()
      seek(Math.min(duration.value, currentTime.value + 10))
      break
    case 'ArrowLeft':
      e.preventDefault()
      seek(Math.max(0, currentTime.value - 5))
      break
    case 'ArrowRight':
      e.preventDefault()
      seek(Math.min(duration.value, currentTime.value + 5))
      break
    case 'ArrowUp':
      e.preventDefault()
      setVolume(Math.min(1, volume.value + 0.1))
      break
    case 'ArrowDown':
      e.preventDefault()
      setVolume(Math.max(0, volume.value - 0.1))
      break
    case 'f':
    case 'F':
      e.preventDefault()
      toggleFullscreen()
      break
    case 'm':
    case 'M':
      e.preventDefault()
      toggleMute()
      break
  }
}

onMounted(() => document.addEventListener('keydown', handleKeydown))
onUnmounted(() => document.removeEventListener('keydown', handleKeydown))

function formatTime(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)

  if (h > 0) {
    return `${h}:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`
  }

  return `${m}:${s.toString().padStart(2, '0')}`
}

function isDrpyProtocol(url: string) {
  return url.startsWith('drpy://')
}

function resetVideoElement() {
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }
  detachNativeVideoErrorHandler()
  activePlaybackAttempt = null
  if (videoRef.value) {
    videoRef.value.pause()
    videoRef.value.removeAttribute('src')
    videoRef.value.load()
  }
  playing.value = false
  pendingAutoplay.value = false
}

async function getHlsConstructor() {
  if (!hlsConstructorPromise) {
    hlsConstructorPromise = import('hls.js').then(module => module.default)
  }

  return hlsConstructorPromise
}

function resetSessionFailoverState() {
  sessionFailoverPromise = null
  lastSessionFailureKey = null
}

function invalidateSessionFailover() {
  sessionGeneration += 1
  resetSessionFailoverState()
}

function createPlaybackAttempt(source: PlayerSource): PlaybackAttemptContext {
  return {
    id: ++playbackAttemptId,
    generation: sessionGeneration,
    url: source.url,
    startedAt: performance.now(),
  }
}

function activatePlaybackAttempt(attempt: PlaybackAttemptContext) {
  activePlaybackAttempt = attempt
  attachNativeVideoErrorHandler(attempt)
}

function isCurrentPlaybackAttempt(attempt: PlaybackAttemptContext | null | undefined) {
  return (
    !!attempt &&
    activePlaybackAttempt?.id === attempt.id &&
    activePlaybackAttempt.generation === attempt.generation &&
    activePlaybackAttempt.url === attempt.url &&
    sessionGeneration === attempt.generation
  )
}

function attachNativeVideoErrorHandler(attempt: PlaybackAttemptContext) {
  detachNativeVideoErrorHandler()
  const video = videoRef.value
  if (!video) return

  const handler = (event: Event) => {
    handleVideoError(event, attempt)
  }
  video.addEventListener('error', handler)
  removeNativeVideoErrorHandler = () => video.removeEventListener('error', handler)
}

function detachNativeVideoErrorHandler() {
  if (removeNativeVideoErrorHandler) {
    removeNativeVideoErrorHandler()
    removeNativeVideoErrorHandler = null
  }
}

function headersMatch(
  left: Record<string, string> | undefined,
  right: Record<string, string> | undefined
) {
  const leftEntries = Object.entries(left ?? {})
  const rightEntries = Object.entries(right ?? {})
  if (leftEntries.length !== rightEntries.length) return false

  return leftEntries.every(([key, value]) => right?.[key] === value)
}

function candidateMatchesSource(candidate: PlayerSource, source: PlayerSource) {
  return (
    candidate.url === source.url &&
    candidate.kind === source.kind &&
    candidate.label === source.label &&
    (candidate.referer ?? '') === (source.referer ?? '') &&
    headersMatch(candidate.headers, source.headers)
  )
}

function findSessionCandidate(session: EpisodePlaybackSession, source: PlayerSource) {
  for (const [sourceIndex, sourceAttempt] of session.sourceAttempts.entries()) {
    const candidateIndex = sourceAttempt.candidates.findIndex(candidate =>
      candidateMatchesSource(candidate, source)
    )
    if (candidateIndex >= 0) {
      return { sourceIndex, candidateIndex, sourceAttempt }
    }
  }

  return null
}

function setManualSessionCandidate(
  session: EpisodePlaybackSession,
  sourceIndex: number,
  candidateIndex: number
) {
  const previousAttempt = session.sourceAttempts[session.activeSourceIndex]
  const nextAttempt = session.sourceAttempts[sourceIndex]
  if (!nextAttempt) return null

  if (
    previousAttempt &&
    previousAttempt !== nextAttempt &&
    (previousAttempt.status === 'playing' || previousAttempt.status === 'resolving')
  ) {
    previousAttempt.status = previousAttempt.candidates.length > 0 ? 'playable' : 'idle'
  }

  session.activeSourceIndex = sourceIndex
  session.activeCandidateIndex = candidateIndex
  session.status = 'playing'
  nextAttempt.status = 'playing'
  return nextAttempt
}

async function switchToSource(index: number) {
  if (index < 0 || index >= sources.value.length) return
  const source = sources.value[index]
  if (!source) return
  const session = playbackSession.value
  if (session) {
    invalidateSessionFailover()
    const owner = findSessionCandidate(session, source)
    if (!owner) {
      playbackSession.value = null
      currentSourceIndex.value = index
      failedSourceIndexes.value = []
      await playSource(source)
      return
    }

    const sourceAttempt = setManualSessionCandidate(session, owner.sourceIndex, owner.candidateIndex)
    if (!sourceAttempt) return
    const candidate = sourceAttempt.candidates[owner.candidateIndex]
    if (!candidate) return
    syncActiveSessionAttempt(session)
    await playSource(candidate)
    return
  }

  invalidateSessionFailover()
  currentSourceIndex.value = index
  await playSource(source)
}

function toPlayerSource(candidate: PlayerSource): PlayerSource {
  return {
    url: candidate.url,
    label: candidate.label,
    kind: candidate.kind,
    headers: candidate.headers,
    referer: candidate.referer,
  }
}

function syncActiveSessionAttempt(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  if (!attempt) {
    sources.value = []
    currentSourceIndex.value = 0
    failedSourceIndexes.value = []
    return
  }

  sources.value = attempt.candidates.map(toPlayerSource)
  currentSourceIndex.value = session.activeCandidateIndex >= 0 ? session.activeCandidateIndex : 0
  failedSourceIndexes.value = attempt.failedCandidateIndexes

  const unifiedSourceIndex = currentUnifiedEpisode.value?.sources.findIndex(source =>
    source.sourceKey === attempt.source.sourceKey &&
    source.episode.play_url === attempt.source.episode.play_url
  )
  if (unifiedSourceIndex !== undefined && unifiedSourceIndex >= 0) {
    currentUnifiedSourceIndex.value = unifiedSourceIndex
  }
}

function attachCandidatesToSessionAttempt(
  session: EpisodePlaybackSession,
  sourceIndex: number,
  candidates: PlayerSource[]
) {
  const attempt = session.sourceAttempts[sourceIndex]
  if (!attempt) return false

  attempt.candidates = candidates
  attempt.failedCandidateIndexes = []
  attempt.status = candidates.length > 0 ? 'playable' : 'failed'
  attempt.failureReason = candidates.length > 0 ? undefined : '当前源没有可用候选线路'

  if (session.activeSourceIndex === sourceIndex) {
    syncActiveSessionAttempt(session)
  }

  return true
}

function activeSessionFailureKey(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  const candidate = attempt?.candidates[session.activeCandidateIndex]
  if (!attempt || !candidate) return null
  return [
    session.activeSourceIndex,
    session.activeCandidateIndex,
    attempt.source.sourceKey,
    candidate.url,
  ].join('|')
}

function activeSessionCandidateUrl(session: EpisodePlaybackSession) {
  const attempt = session.sourceAttempts[session.activeSourceIndex]
  return attempt?.candidates[session.activeCandidateIndex]?.url ?? null
}

async function resolveActiveAttempt(
  session: EpisodePlaybackSession,
  sourceIndex = session.activeSourceIndex,
  expectedGeneration = sessionGeneration
) {
  const attempt = session.sourceAttempts[sourceIndex]
  if (!attempt) return false

  try {
    if (
      sessionGeneration !== expectedGeneration ||
      playbackSession.value !== session ||
      session.activeSourceIndex !== sourceIndex ||
      session.sourceAttempts[sourceIndex] !== attempt
    ) {
      return false
    }

    if (itemId.value > 0) {
      const resolved = await playbackStore.resolve(
        attempt.source.episode.play_url,
        attempt.source.episode.id
      )
      if (
        sessionGeneration !== expectedGeneration ||
        playbackSession.value !== session ||
        session.activeSourceIndex !== sourceIndex ||
        session.sourceAttempts[sourceIndex] !== attempt
      ) {
        return false
      }
      attachCandidatesToSessionAttempt(session, sourceIndex, resolved.candidates.map(toPlayerSource))
      return true
    }

    if (sourceName.value) {
      const targets = await invoke<PlaybackTarget[]>('provider_play', {
        source: sourceName.value,
        flag: 'auto',
        playUrl: attempt.source.episode.play_url,
      })
      if (
        sessionGeneration !== expectedGeneration ||
        playbackSession.value !== session ||
        session.activeSourceIndex !== sourceIndex ||
        session.sourceAttempts[sourceIndex] !== attempt
      ) {
        return false
      }
      const target = targets[0]
      if (!target) {
        attachCandidatesToSessionAttempt(session, sourceIndex, [])
        return true
      }

      if (target.target_kind === 'Direct') {
        attachCandidatesToSessionAttempt(session, sourceIndex, [{
          url: target.target_url,
          label: attempt.source.sourceName || sourceName.value || '来源',
          kind: target.target_url.includes('.m3u8') ? 'hls' : 'http',
          headers: target.headers ?? undefined,
          referer: target.referer ?? undefined,
        }])
        return true
      }

      const resolved = await playbackStore.resolve(target.target_url, attempt.source.episode.id)
      if (
        sessionGeneration !== expectedGeneration ||
        playbackSession.value !== session ||
        session.activeSourceIndex !== sourceIndex ||
        session.sourceAttempts[sourceIndex] !== attempt
      ) {
        return false
      }
      attachCandidatesToSessionAttempt(session, sourceIndex, resolved.candidates.map(toPlayerSource))
      return true
    }

    attachCandidatesToSessionAttempt(session, sourceIndex, [])
    return true
  } catch (error) {
    if (
      sessionGeneration !== expectedGeneration ||
      playbackSession.value !== session ||
      session.activeSourceIndex !== sourceIndex ||
      session.sourceAttempts[sourceIndex] !== attempt
    ) {
      return false
    }
    markCurrentCandidateFailed(session, String(error))
    return false
  }
}

async function runSessionFailover(
  session: EpisodePlaybackSession,
  reason?: string,
  expectedAttempt?: PlaybackAttemptContext | null,
  expectedGeneration = sessionGeneration
) {
  if (sessionGeneration !== expectedGeneration) {
    return
  }
  if (expectedAttempt && !isCurrentPlaybackAttempt(expectedAttempt)) {
    return
  }
  const expectedCandidateUrl = expectedAttempt?.url ?? null
  if (
    expectedCandidateUrl &&
    activeSessionCandidateUrl(session) &&
    activeSessionCandidateUrl(session) !== expectedCandidateUrl
  ) {
    return
  }

  const failureKey = reason ? activeSessionFailureKey(session) : null
  if (failureKey && failureKey === lastSessionFailureKey) {
    return
  }

  if (reason) {
    markCurrentCandidateFailed(session, reason)
    if (failureKey) {
      lastSessionFailureKey = failureKey
    }
  }

  const nextCandidate = nextCandidateToPlay(session)
  if (nextCandidate) {
    if (sessionGeneration !== expectedGeneration) return
    if (playbackSession.value !== session) return
    syncActiveSessionAttempt(session)
    await playSource(nextCandidate)
    return
  }

  for (;;) {
    const attempt = startNextSourceAttempt(session)
    if (!attempt) {
      if (sessionGeneration !== expectedGeneration) return
      errorMsg.value = session.lastError ?? '该集所有播放源均不可用'
      return
    }

    const resolved = await resolveActiveAttempt(session, session.activeSourceIndex, expectedGeneration)
    if (sessionGeneration !== expectedGeneration) return
    if (playbackSession.value !== session) return
    if (!resolved) {
      continue
    }

    const candidate = nextCandidateToPlay(session)
    if (candidate) {
      if (sessionGeneration !== expectedGeneration) return
      if (playbackSession.value !== session) return
      syncActiveSessionAttempt(session)
      await playSource(candidate)
      return
    }

    markCurrentCandidateFailed(session, attempt.failureReason ?? '当前源没有可用候选线路')
  }
}

async function playNextFromSession(reason?: string, expectedAttempt?: PlaybackAttemptContext | null) {
  const session = playbackSession.value
  if (!session) return

  const expectedGeneration = sessionGeneration
  if (expectedAttempt && !isCurrentPlaybackAttempt(expectedAttempt)) {
    return
  }
  const expectedCandidateUrl = expectedAttempt?.url ?? null
  const failureKey = reason ? activeSessionFailureKey(session) : null
  if (failureKey && failureKey === lastSessionFailureKey) {
    return
  }

  if (
    expectedCandidateUrl &&
    activeSessionCandidateUrl(session) &&
    activeSessionCandidateUrl(session) !== expectedCandidateUrl
  ) {
    return
  }

  if (sessionFailoverPromise) {
    await sessionFailoverPromise
    if (sessionGeneration !== expectedGeneration) return
    if (playbackSession.value !== session) return
    if (expectedAttempt && !isCurrentPlaybackAttempt(expectedAttempt)) {
      return
    }
    if (failureKey && failureKey === lastSessionFailureKey) {
      return
    }
    if (
      expectedCandidateUrl &&
      activeSessionCandidateUrl(session) &&
      activeSessionCandidateUrl(session) !== expectedCandidateUrl
    ) {
      return
    }
  }

  const promise = runSessionFailover(session, reason, expectedAttempt, expectedGeneration)
  sessionFailoverPromise = promise
  try {
    await promise
  } finally {
    if (sessionFailoverPromise === promise) {
      sessionFailoverPromise = null
    }
  }
}

async function initVodPlayback(url: string, id?: number) {
  playbackSession.value = null
  invalidateSessionFailover()
  const decodedUrl = decodeURIComponent(url)
  const resolved = await playbackStore.resolve(decodedUrl, id)
  sources.value = resolved.candidates.map(candidate => ({
    url: candidate.url,
    label: candidate.label,
    kind: candidate.kind,
    headers: candidate.headers,
    referer: candidate.referer
  }))
  currentSourceIndex.value = 0
  failedSourceIndexes.value = []

  if (resolved.status === 'ready' && sources.value.length > 0) {
    await playSource(sources.value[0])
  } else if (resolved.status === 'external_required' && sources.value.length > 0) {
    errorMsg.value = resolved.errorMessage ?? '当前资源需要外部处理'
    await playSource(sources.value[0])
  } else {
    errorMsg.value = resolved.errorMessage ?? '当前条目没有可用线路'
  }
}

async function playUnifiedEpisode(unifiedEpisode: UnifiedEpisode, sourceIndex = 0) {
  currentUnifiedEpisode.value = unifiedEpisode
  currentUnifiedSourceIndex.value = sourceIndex
  failedSourceIndexes.value = []

  if (sourceIndex >= unifiedEpisode.sources.length || unifiedEpisode.sources.length === 0) {
    errorMsg.value = '该集所有线路均不可用'
    return
  }

  const session = createEpisodePlaybackSession(unifiedEpisode)
  playbackSession.value = session
  invalidateSessionFailover()

  const preferredSource = unifiedEpisode.sources[sourceIndex]
  const attempt = startNextSourceAttempt(session, {
    sourceKey: preferredSource?.sourceKey,
    manual: sourceIndex > 0,
  })
  if (!attempt) {
    errorMsg.value = session.lastError ?? '该集所有播放源均不可用'
    return
  }

  const expectedGeneration = sessionGeneration
  const resolved = await resolveActiveAttempt(session, session.activeSourceIndex, expectedGeneration)
  if (!resolved) {
    if (sessionGeneration !== expectedGeneration || playbackSession.value !== session) {
      return
    }
    await playNextFromSession()
    return
  }

  const candidate = nextCandidateToPlay(session)
  if (candidate) {
    syncActiveSessionAttempt(session)
    await playSource(candidate)
    return
  }

  await playNextFromSession('当前源没有可用候选线路')
}

async function switchToEpisode(unifiedEpisode: UnifiedEpisode) {
  const firstSource = unifiedEpisode.sources[0]
  if (!firstSource) return

  router.replace(
    `/player/vod/${itemId.value}?episode=${encodeURIComponent(firstSource.episode.play_url)}&episodeId=${firstSource.episode.id}`
  )

  await playUnifiedEpisode(unifiedEpisode)
}

async function switchEpisodeSource(sourceKey: string) {
  const session = playbackSession.value
  if (!session) return

  invalidateSessionFailover()
  const attempt = startNextSourceAttempt(session, { sourceKey, manual: true })
  if (!attempt) {
    errorMsg.value = session.lastError ?? '该源不可用'
    return
  }

  const expectedGeneration = sessionGeneration
  const resolved = await resolveActiveAttempt(session, session.activeSourceIndex, expectedGeneration)
  if (!resolved) {
    if (sessionGeneration !== expectedGeneration || playbackSession.value !== session) {
      return
    }
    errorMsg.value = attempt.failureReason ?? '解析失败'
    await playNextFromSession(attempt.failureReason ?? '解析失败')
    return
  }

  const candidate = nextCandidateToPlay(session)
  if (!candidate) {
    errorMsg.value = '当前源没有可用候选线路'
    await playNextFromSession('当前源没有可用候选线路')
    return
  }

  syncActiveSessionAttempt(session)
  errorMsg.value = ''
  await playSource(candidate)
}

function markCurrentSourceFailed() {
  if (!failedSourceIndexes.value.includes(currentSourceIndex.value)) {
    failedSourceIndexes.value = [...failedSourceIndexes.value, currentSourceIndex.value]
  }
}

async function playSource(source: PlayerSource) {
  errorMsg.value = ''
  const url = source.url

  if (isDrpyProtocol(url) || source.kind === 'external') {
    resetVideoElement()
    errorMsg.value = source.kind === 'external' ? '该线路需要外部工具处理，已尝试交给系统打开' : '该地址需要外部解析，已尝试交给系统处理'
    await open(url)
    return
  }

  await initHlsPlayer(source)
}

async function initHlsPlayer(source: PlayerSource) {
  if (!videoRef.value) return
  const video = videoRef.value
  const { url, headers, referer } = source

  detachNativeVideoErrorHandler()
  activePlaybackAttempt = null
  if (hlsInstance) {
    hlsInstance.destroy()
    hlsInstance = null
  }

  const playbackAttempt = createPlaybackAttempt(source)

  if (url.includes('.m3u8')) {
    const Hls = await getHlsConstructor()
    if (playbackAttempt.generation !== sessionGeneration) return

    if (Hls.isSupported()) {
      // Custom loader to bypass CORS for CDN URLs
      // Custom loader for ad filtering and CORS bypass
      const CustomLoader = class extends Hls.DefaultConfig.loader {
        load(context: any, config: any, callbacks: any) {
          const url = context.url
          // All manifest and segment requests go through Rust proxy for ad filtering,
          // CORS bypass, and automatic Referer retry for auth-blocking CDNs.
          const cleanUrl = url.split('?')[0].split('#')[0]
          const isManifest = cleanUrl.includes('.m3u8')
          const isSegment = cleanUrl.endsWith('.ts') || cleanUrl.endsWith('.mp4')
          if (isManifest || isSegment) {
            invoke<string>('fetch_hls_manifest', { url, headers, referer })
              .then((data) => {
                const finalData: string | ArrayBuffer = isSegment
                  ? Uint8Array.from(atob(data), c => c.charCodeAt(0)).buffer
                  : data
                const finalLength = typeof finalData === 'string' ? finalData.length : finalData.byteLength
                const stats = { aborted: false, loaded: finalLength, retry: 0, total: finalLength, chunkCount: 0, bwEstimate: 0, loading: { start: 0, first: 0, end: 0 }, parsing: { start: 0, end: 0 }, buffering: { start: 0, end: 0 } }
                callbacks.onSuccess({ data: finalData, url, code: 200 }, stats, context, null)
              })
              .catch((err) => {
                callbacks.onError({ code: 0, text: String(err) }, context, null, { aborted: false, loaded: 0, retry: 0, total: 0, chunkCount: 0, bwEstimate: 0, loading: { start: 0, first: 0, end: 0 }, parsing: { start: 0, end: 0 }, buffering: { start: 0, end: 0 } })
              })
            return
          }
          // Default behavior for other URLs
          ;(super.load as any)(context, config, callbacks)
        }
      }

      const hls = new Hls({ loader: CustomLoader as any })
      hls.on(Hls.Events.ERROR, (_event, data) => {
        if (!data.fatal) return
        if (playbackSession.value) {
          void playNextFromSession(data.error?.message || 'HLS 播放失败', playbackAttempt)
          return
        }

        markCurrentSourceFailed()

        if (currentSourceIndex.value < sources.value.length - 1) {
          void switchToSource(currentSourceIndex.value + 1)
        } else if (
          currentUnifiedEpisode.value &&
          currentUnifiedSourceIndex.value < currentUnifiedEpisode.value.sources.length - 1
        ) {
          void playUnifiedEpisode(currentUnifiedEpisode.value, currentUnifiedSourceIndex.value + 1)
        } else {
          errorMsg.value = data.error?.message || '所有线路均不可用'
        }
      })

      hls.on(Hls.Events.MANIFEST_PARSED, () => {
        void attemptPlayback(false, playbackAttempt)
      })

      hlsInstance = hls
      hls.loadSource(url)
      hls.attachMedia(video)
      activatePlaybackAttempt(playbackAttempt)
      return
    }

    if (video.canPlayType('application/vnd.apple.mpegurl')) {
      video.src = url
      video.load()
      activatePlaybackAttempt(playbackAttempt)
      pendingAutoplay.value = true
      return
    }
  }

  video.src = url
  video.load()
  activatePlaybackAttempt(playbackAttempt)
  pendingAutoplay.value = true
}

async function attemptPlayback(
  _manual: boolean,
  playbackAttempt: PlaybackAttemptContext | null = activePlaybackAttempt
) {
  if (!videoRef.value) return
  if (playbackAttempt && !isCurrentPlaybackAttempt(playbackAttempt)) return

  try {
    await videoRef.value.play()
    if (playbackAttempt && !isCurrentPlaybackAttempt(playbackAttempt)) return
    errorMsg.value = ''
    playing.value = true
    pendingAutoplay.value = false
  } catch (error) {
    playing.value = false
    pendingAutoplay.value = false
    const message = describePlaybackFailure(error)
    errorMsg.value = message
    if (isAutoplayBlocked(error)) {
      return
    }
    if (playbackSession.value && shouldFailoverAfterPlaybackError(error)) {
      await playNextFromSession(message, playbackAttempt)
    }
  }
}

function handleCanPlay() {
  if (!pendingAutoplay.value) return
  void attemptPlayback(false, activePlaybackAttempt)
}

function handleVideoPlay() {
  playing.value = true
  errorMsg.value = ''
  if (playbackSession.value) {
    markCurrentCandidatePlaying(playbackSession.value)
  }
  showControls()
}

function handleVideoPause() {
  playing.value = false
  if (hideTimer) {
    window.clearTimeout(hideTimer)
    hideTimer = null
  }
  controlsVisible.value = true
}

function handleVideoError(event: Event, playbackAttempt: PlaybackAttemptContext) {
  if (event.timeStamp > 0 && event.timeStamp + 1 < playbackAttempt.startedAt) {
    return
  }
  if (!isCurrentPlaybackAttempt(playbackAttempt)) {
    return
  }

  pendingAutoplay.value = false
  const mediaError = videoRef.value?.error
  const message = describeMediaErrorCode(mediaError?.code)
  if (playbackSession.value) {
    void playNextFromSession(message, playbackAttempt)
    return
  }

  markCurrentSourceFailed()

  if (currentSourceIndex.value < sources.value.length - 1) {
    errorMsg.value = `${message}，正在切换下一条线路`
    void switchToSource(currentSourceIndex.value + 1)
  } else if (
    currentUnifiedEpisode.value &&
    currentUnifiedSourceIndex.value < currentUnifiedEpisode.value.sources.length - 1
  ) {
    errorMsg.value = `${message}，正在切换下一个源`
    void playUnifiedEpisode(currentUnifiedEpisode.value, currentUnifiedSourceIndex.value + 1)
  } else {
    errorMsg.value = message
  }
}
</script>

<template>
  <div class="player-shell">
    <div class="player-frame">
      <header class="player-topbar">
        <button class="action-button action-button-secondary" type="button" @click="router.back()">
          返回
        </button>
        <div class="player-title">
          <strong>{{ pageTitle }}</strong>
        </div>
        <div class="player-context">
          <span>{{ playerModeLabel }}</span>
          <span>{{ sourceLabel }}</span>
          <span>{{ playerStatusText }}</span>
        </div>
      </header>

      <div class="player-layout">
        <section class="player-stage">
          <div class="player-video-wrap" ref="videoWrapRef">
            <video
              ref="videoRef"
              class="player-video"
              playsinline
              @click="togglePlay"
              @canplay="handleCanPlay"
              @play="handleVideoPlay"
              @pause="handleVideoPause"
            ></video>

            <div class="player-vignette-top"></div>
            <div class="player-vignette-bottom"></div>

            <div
              class="player-overlay"
              @mouseenter="showControls"
              @mouseleave="startHideTimer"
            >
              <PlaybackNotice v-if="errorMsg" :message="errorMsg" :tone="noticeTone" />

              <div
                class="player-controls"
                :class="{ 'controls-hidden': !controlsVisible }"
              >
                <div class="player-progress">
                  <span>{{ formatTime(currentTime) }}</span>
                  <input
                    type="range"
                    :value="currentTime"
                    :max="duration || 100"
                    class="player-range"
                    @input="seek(parseFloat(($event.target as HTMLInputElement).value))"
                  />
                  <span>{{ formatTime(duration) }}</span>
                </div>

                <div class="player-control-row">
                  <div class="player-control-actions">
                    <button class="action-button action-button-primary" type="button" @click="togglePlay">
                      {{ playing ? '暂停' : '播放' }}
                    </button>
                    <button class="action-button action-button-secondary" type="button" @click="toggleFullscreen">
                      {{ fullscreen ? '退出全屏' : '全屏' }}
                    </button>
                  </div>

                  <div v-if="fullscreenError" style="color: #f87171; font-size: 0.75rem; margin-top: 4px;">
                    {{ fullscreenError }}
                  </div>

                  <label class="player-volume">
                    <span>Volume</span>
                    <input
                      type="range"
                      :value="volume"
                      min="0"
                      max="1"
                      step="0.1"
                      class="player-range player-volume-range"
                      @input="handleVolumeChange"
                    />
                  </label>
                </div>
              </div>
            </div>
          </div>
        </section>

        <PlaybackDrawer
          :sources="sources"
          :current-index="currentSourceIndex"
          :failed-indexes="failedSourceIndexes"
          :status="playerStatusText"
          :error-message="errorMsg || playbackStore.errorMessage"
          :unified-episodes="unifiedEpisodes"
          :current-normalized-index="currentNormalizedIndex"
          :item-type="itemType"
          :episode-source-attempts="currentEpisodeSourceAttempts"
          @select-episode="switchToEpisode"
          @switch-line="switchToSource"
          @switch-episode-source="switchEpisodeSource"
        />
      </div>
    </div>
  </div>
</template>
