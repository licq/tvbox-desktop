import { describe, expect, it } from 'vitest'
import {
  applyPlaybackAdCleanup,
  classifyPlaybackRequest,
  isPlaybackAdResource,
} from '@/utils/playbackAdBlocking'

describe('playback ad blocking helpers', () => {
  it('classifies HLS manifests and direct segments', () => {
    expect(classifyPlaybackRequest('https://cdn.example.com/live/index.m3u8')).toBe('manifest')
    expect(classifyPlaybackRequest('https://cdn.example.com/seg-1.ts')).toBe('segment')
    expect(classifyPlaybackRequest('https://cdn.example.com/video.mp4')).toBe('segment')
    expect(classifyPlaybackRequest('https://cdn.example.com/player.js')).toBeNull()
  })

  it('flags obvious ad resources but not player assets', () => {
    expect(isPlaybackAdResource('https://ads.example.com/adservice.js')).toBe(true)
    expect(isPlaybackAdResource('https://cdn.example.com/hls.js')).toBe(false)
  })

  it('removes only obvious ad overlays and keeps player controls', () => {
    document.body.innerHTML = `
      <div id="player-stage">
        <div class="player-controls"></div>
        <div class="banner-ad"></div>
        <iframe src="https://ads.example.com/ad/frame.html"></iframe>
      </div>
    `

    const removed = applyPlaybackAdCleanup(document)

    expect(removed).toBe(2)
    expect(document.querySelector('.banner-ad')).toBeNull()
    expect(document.querySelector('iframe')).toBeNull()
    expect(document.querySelector('.player-controls')).not.toBeNull()
  })
})
