import { createPinia, setActivePinia } from 'pinia'
import { describe, expect, it } from 'vitest'
import { usePlaybackStore } from '@/stores/playback'

describe('playback store', () => {
  it('switches to the next candidate after a fatal playback error', () => {
    setActivePinia(createPinia())
    const store = usePlaybackStore()

    store.applyResolved({
      status: 'ready',
      candidates: [
        { url: 'https://a.example/1.m3u8', label: '线路1', kind: 'hls' },
        { url: 'https://b.example/1.m3u8', label: '线路2', kind: 'hls' }
      ]
    })

    store.handleFatalPlaybackError('network')

    expect(store.currentCandidate?.label).toBe('线路2')
    expect(store.status).toBe('ready')
  })

  it('keeps failed runtime state when no playable lines are returned', () => {
    setActivePinia(createPinia())
    const store = usePlaybackStore()

    store.applyResolved({
      status: 'failed',
      candidates: [],
      errorMessage: '当前集未找到通过探测的可播线路'
    })

    expect(store.status).toBe('failed')
    expect(store.candidates).toEqual([])
    expect(store.errorMessage).toContain('可播线路')
  })
})
