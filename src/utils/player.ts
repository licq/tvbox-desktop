export function isAutoplayBlocked(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const maybeError = error as { name?: string; message?: string }
  return maybeError.name === 'NotAllowedError' || maybeError.message?.includes('NotAllowedError') === true
}

export function describeMediaErrorCode(code?: number | null): string {
  switch (code) {
    case 1:
      return '播放被中止'
    case 2:
      return '网络错误'
    case 3:
      return '媒体解码失败'
    case 4:
      return '浏览器不支持当前媒体格式'
    default:
      return '媒体播放失败'
  }
}

export function describePlaybackFailure(error: unknown): string {
  if (isAutoplayBlocked(error)) {
    return '线路已加载，点击播放开始'
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message
  }

  return '无法直接播放当前地址'
}

export interface PlayerTitleInput {
  title?: string | null
  episodeLabel?: string | null
  sourceLabel?: string | null
}

export function formatPlayerTitle(input: PlayerTitleInput) {
  const title = input.title?.trim()
  const episodeLabel = input.episodeLabel?.trim()
  const sourceLabel = input.sourceLabel?.trim()

  if (title && episodeLabel) return `${title} · ${episodeLabel}`
  if (title) return title
  if (episodeLabel) return episodeLabel
  if (sourceLabel) return sourceLabel
  return 'TVBox'
}

export interface ProviderPlaybackRouteInput {
  mode: string
  itemId: number
  source?: string | null
  detailUrl?: string | null
  episodeUrl?: string | null
}

export function isProviderDirectPlaybackRoute(input: ProviderPlaybackRouteInput): boolean {
  return (
    input.mode === 'vod' &&
    input.itemId === 0 &&
    typeof input.source === 'string' &&
    input.source.trim().length > 0 &&
    typeof input.detailUrl === 'string' &&
    input.detailUrl.trim().length > 0 &&
    typeof input.episodeUrl === 'string' &&
    input.episodeUrl.trim().length > 0
  )
}

export function parsePlaybackHeaders(raw?: string | null): Record<string, string> | null {
  if (!raw) return null

  try {
    const parsed = JSON.parse(raw) as unknown
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return null

    const headers: Record<string, string> = {}
    for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
      if (key.trim().length === 0 || typeof value !== 'string') continue
      headers[key] = value
    }

    return Object.keys(headers).length > 0 ? headers : null
  } catch {
    return null
  }
}

export function isDirectMediaUrl(url: string): boolean {
  const normalized = url.toLowerCase()
  return [
    '.m3u8',
    '.mp4',
    '.m4v',
    '.webm',
    '.mov',
  ].some(ext => normalized.includes(ext))
}

export function isPlaybackPageUrl(url: string): boolean {
  const normalized = url.toLowerCase()
  if (isDirectMediaUrl(normalized)) return false

  return (
    normalized.includes('xb6v.com/e/downsys/play/') ||
    normalized.includes('/vodplay/') ||
    normalized.includes('/vod/play/') ||
    (normalized.includes('/play/') && !normalized.includes('/player/'))
  )
}

export function shouldFallbackToBrowserHls(error: unknown): boolean {
  const message =
    error instanceof Error
      ? error.message
      : typeof error === 'string'
        ? error
        : String(error)

  const normalized = message.toLowerCase()
  return [
    'tls handshake eof',
    'connection closed via error',
    'error sending request',
    'unexpected eof',
    'connection reset',
    'timed out',
    'timeout',
    'dns error',
    'could not resolve host',
  ].some(pattern => normalized.includes(pattern))
}
