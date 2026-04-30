export type PlaybackRequestKind = 'manifest' | 'segment' | null

const AD_RESOURCE_MARKERS: readonly string[] = [
  'doubleclick',
  'googlesyndication',
  'adservice',
  '/ad/',
  'banner-ad',
  'overlay-ad',
  'player-ad',
]

const AD_SELECTORS: readonly string[] = [
  'iframe[src*="doubleclick"]',
  'iframe[src*="googlesyndication"]',
  'iframe[src*="/ad/"]',
  '[class*="banner-ad"]',
  '[class*="overlay-ad"]',
  '[class*="player-ad"]',
  '[id*="banner-ad"]',
  '[id*="overlay-ad"]',
]

export function classifyPlaybackRequest(url: string): PlaybackRequestKind {
  const cleanUrl: string = url.split('?')[0].split('#')[0].toLowerCase()

  if (cleanUrl.endsWith('.m3u8')) {
    return 'manifest'
  }

  if (
    cleanUrl.endsWith('.ts') ||
    cleanUrl.endsWith('.mp4') ||
    cleanUrl.endsWith('.m4v') ||
    cleanUrl.endsWith('.webm') ||
    cleanUrl.endsWith('.mov')
  ) {
    return 'segment'
  }

  return null
}

export function isPlaybackAdResource(url: string): boolean {
  const cleanUrl: string = url.split('?')[0].split('#')[0].toLowerCase()
  return AD_RESOURCE_MARKERS.some((marker: string) => cleanUrl.includes(marker))
}

export function applyPlaybackAdCleanup(root: ParentNode): number {
  let removed: number = 0

  for (const selector of AD_SELECTORS) {
    const matches: NodeListOf<Element> = root.querySelectorAll(selector)

    matches.forEach((node: Element) => {
      const element: HTMLElement = node as HTMLElement

      if (element.closest('.player-controls, .playback-header, .playback-drawer')) {
        return
      }

      element.remove()
      removed += 1
    })
  }

  return removed
}
