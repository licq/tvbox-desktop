import { describe, expect, it, vi } from 'vitest'
import { enterFullscreen } from '@/utils/fullscreen'

describe('fullscreen utility', () => {
  it('uses element fullscreen before native fallback', async () => {
    const requestFullscreen = vi.fn().mockResolvedValue(undefined)
    const nativeEnter = vi.fn()

    await enterFullscreen({ requestFullscreen } as unknown as HTMLElement, nativeEnter)

    expect(requestFullscreen).toHaveBeenCalled()
    expect(nativeEnter).not.toHaveBeenCalled()
  })

  it('falls back to native fullscreen when element fullscreen rejects', async () => {
    const requestFullscreen = vi.fn().mockRejectedValue(new Error('unsupported'))
    const nativeEnter = vi.fn().mockResolvedValue(undefined)

    await enterFullscreen({ requestFullscreen } as unknown as HTMLElement, nativeEnter)

    expect(nativeEnter).toHaveBeenCalled()
  })
})
