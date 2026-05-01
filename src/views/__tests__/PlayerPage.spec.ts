import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'

const route = {
  name: 'player-vod',
  params: {
    mode: 'vod',
    id: '0',
  },
  query: {
    episode: encodeURIComponent('https://cdn.example.com/movie.mp4'),
    source: 'demo',
    title: '测试影片',
  },
}

const router = {
  back: vi.fn(),
  replace: vi.fn(),
}

const playbackStore = {
  status: 'idle',
  errorMessage: null,
  resolve: vi.fn(async () => ({
    status: 'ready',
    candidates: [
      {
        url: 'https://cdn.example.com/movie.mp4',
        label: '线路 1',
        kind: 'http',
      },
    ],
    errorMessage: null,
  })),
}

const playerStore = {
  pendingUnifiedEpisode: null,
  setPendingUnifiedEpisode: vi.fn(),
  saveHistory: vi.fn(),
}

const detailStore = {
  item: null,
  episodeGroups: [],
  loading: false,
  fetchDetail: vi.fn(),
}

const liveStore = {
  channels: [],
  fetchChannels: vi.fn(),
}

vi.mock('vue-router', () => ({
  useRoute: () => route,
  useRouter: () => router,
}))

vi.mock('@/stores/playback', () => ({
  usePlaybackStore: () => playbackStore,
}))

vi.mock('@/stores/player', () => ({
  usePlayerStore: () => playerStore,
}))

vi.mock('@/stores/detail', () => ({
  useDetailStore: () => detailStore,
}))

vi.mock('@/stores/live', () => ({
  useLiveStore: () => liveStore,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async () => null),
}))

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(async () => undefined),
}))

vi.mock('@/components/player/PlaybackDrawer.vue', () => ({
  default: {
    name: 'PlaybackDrawer',
    template: '<aside class="playback-drawer-stub"></aside>',
  },
}))

vi.mock('@/components/player/PlaybackNotice.vue', () => ({
  default: {
    name: 'PlaybackNotice',
    template: '<div class="playback-notice-stub"></div>',
  },
}))

vi.mock('hls.js', () => ({
  default: {
    isSupported: () => false,
    Events: {},
    DefaultConfig: { loader: class {} },
  },
}))

describe('PlayerPage fullscreen controls', () => {
  let loadSpy: ReturnType<typeof vi.spyOn> | null = null

  beforeEach(() => {
    router.back.mockClear()
    router.replace.mockClear()
    playbackStore.resolve.mockClear()
    playerStore.saveHistory.mockClear()
    detailStore.fetchDetail.mockClear()
    liveStore.fetchChannels.mockClear()
    document.body.removeAttribute('style')
    loadSpy = vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined)
  })

  afterEach(() => {
    loadSpy?.mockRestore()
    loadSpy = null
  })

  it('keeps the VOD progress bar visible in fullscreen playback', async () => {
    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          SourceBadge: true,
        },
      },
    })

    await flushPromises()
    await nextTick()

    await wrapper.find('video').trigger('play')
    await nextTick()

    const fullscreenButton = wrapper
      .findAll('button')
      .find(button => button.text().includes('全屏'))

    expect(fullscreenButton).toBeDefined()

    await fullscreenButton!.trigger('click')
    await nextTick()

    expect(wrapper.find('.player-video-wrap').attributes('style')).toContain('position: fixed')
    expect(wrapper.find('.player-progress').exists()).toBe(true)
  })
})
