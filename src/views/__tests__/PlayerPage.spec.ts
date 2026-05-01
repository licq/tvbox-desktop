import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import type { CatalogDetailItem, CatalogEpisodeGroup, UnifiedEpisode } from '@/types'
import { clearPlaybackHealth, markPlaybackHealth } from '@/utils/playbackSession'

const hlsState = {
  supported: false,
}

const invokeMock = vi.fn(async () => null)

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
  } as Record<string, string | undefined>,
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
  pendingUnifiedEpisode: null as UnifiedEpisode | null,
  pendingVodDetail: null as {
    item: CatalogDetailItem
    episode_groups: CatalogEpisodeGroup[]
  } | null,
  setPendingUnifiedEpisode: vi.fn(),
  setPendingVodDetail: vi.fn(),
  saveHistory: vi.fn(),
}

const detailStore = {
  item: null as CatalogDetailItem | null,
  episodeGroups: [] as CatalogEpisodeGroup[],
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
  invoke: invokeMock,
}))

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn(async () => undefined),
}))

vi.mock('@/components/player/PlaybackDrawer.vue', () => ({
  default: {
    name: 'PlaybackDrawer',
    props: ['loading'],
    template: '<aside class="playback-drawer-stub" :data-loading="String(loading)"></aside>',
  },
}))

vi.mock('@/components/player/PlaybackNotice.vue', () => ({
  default: {
    name: 'PlaybackNotice',
    template: '<div class="playback-notice-stub"></div>',
  },
}))

vi.mock('hls.js', () => ({
  default: class HlsMock {
    static Events = {
      ERROR: 'ERROR',
      MANIFEST_PARSED: 'MANIFEST_PARSED',
      MEDIA_ATTACHED: 'MEDIA_ATTACHED',
    }

    static DefaultConfig = {
      loader: class {
        load() {}
      },
    }

    static isSupported() {
      return hlsState.supported
    }

    private readonly listeners = new Map<string, Array<(...args: any[]) => void>>()

    constructor(private readonly options?: { loader?: new () => { load: (...args: any[]) => void } }) {}

    on(event: string, callback: (...args: any[]) => void) {
      const callbacks = this.listeners.get(event) ?? []
      callbacks.push(callback)
      this.listeners.set(event, callbacks)
    }

    private emit(event: string, ...args: any[]) {
      for (const callback of this.listeners.get(event) ?? []) {
        callback(...args)
      }
    }

    loadSource(url: string) {
      const Loader = this.options?.loader
      if (Loader) {
        const loader = new Loader()
        loader.load({ url }, {}, { onError() {}, onSuccess() {} })
      }
      this.emit(HlsMock.Events.MEDIA_ATTACHED)
      this.emit(HlsMock.Events.MANIFEST_PARSED)
    }

    attachMedia() {}

    destroy() {}
  },
}))

describe('PlayerPage fullscreen controls', () => {
  let loadSpy: ReturnType<typeof vi.spyOn> | null = null
  let pauseSpy: ReturnType<typeof vi.spyOn> | null = null
  let canPlaySpy: any = null

  beforeEach(() => {
    hlsState.supported = false
    invokeMock.mockReset()
    invokeMock.mockResolvedValue(null)
    route.params.mode = 'vod'
    route.params.id = '0'
    route.query.episode = encodeURIComponent('https://cdn.example.com/movie.mp4')
    route.query.source = 'demo'
    route.query.title = '测试影片'
    delete route.query.detailUrl
    delete route.query.episodeId
    delete route.query.episodeLabel
    delete route.query.episodeHeaders
    delete route.query.episodeReferer
    delete route.query.episodeTargets
    router.back.mockClear()
    router.replace.mockClear()
    playbackStore.resolve.mockClear()
    playerStore.saveHistory.mockClear()
    playerStore.setPendingUnifiedEpisode.mockClear()
    playerStore.setPendingVodDetail.mockClear()
    detailStore.fetchDetail.mockClear()
    liveStore.fetchChannels.mockClear()
    clearPlaybackHealth()
    document.body.removeAttribute('style')
    pauseSpy = vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => undefined)
    loadSpy = vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(() => undefined)
    canPlaySpy = vi.spyOn(HTMLMediaElement.prototype, 'canPlayType').mockReturnValue('')
  })

  afterEach(() => {
    pauseSpy?.mockRestore()
    pauseSpy = null
    loadSpy?.mockRestore()
    loadSpy = null
    canPlaySpy?.mockRestore()
    canPlaySpy = null
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

  it('prefers native hls for supported m3u8 playback', async () => {
    hlsState.supported = true
    route.query.episode = encodeURIComponent('https://cdn.example.com/live/index.m3u8')
    playbackStore.resolve.mockResolvedValue({
      status: 'ready',
      candidates: [
        {
          url: 'https://cdn.example.com/live/index.m3u8',
          label: '线路 1',
          kind: 'hls',
        },
      ],
      errorMessage: null,
    })
    invokeMock.mockRejectedValueOnce(new Error('A network error (status 0) occurred while loading manifest'))
    canPlaySpy?.mockRestore()
    canPlaySpy = vi.spyOn(HTMLMediaElement.prototype, 'canPlayType').mockReturnValue('probably')

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

    expect(wrapper.find('video').element.getAttribute('src')).toContain('https://cdn.example.com/live/index.m3u8')
    expect(invokeMock).not.toHaveBeenCalled()
  })

  it('retries native hls playback through browser hls when the media decoder rejects the stream', async () => {
    hlsState.supported = true
    route.query.episode = encodeURIComponent('https://cdn.example.com/live/index.m3u8')
    playbackStore.resolve.mockResolvedValue({
      status: 'ready',
      candidates: [
        {
          url: 'https://cdn.example.com/live/index.m3u8',
          label: '线路 1',
          kind: 'hls',
          referer: 'https://www.ypanso.com/vod/play/id/9CnHHHHR/sid/1/nid/1.html',
        } as any,
      ],
      errorMessage: null,
    })
    canPlaySpy?.mockRestore()
    canPlaySpy = vi.spyOn(HTMLMediaElement.prototype, 'canPlayType').mockReturnValue('probably')

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

    Object.defineProperty(wrapper.find('video').element, 'error', {
      value: { code: 4 },
      configurable: true,
    })
    await wrapper.find('video').trigger('error')
    await flushPromises()
    await nextTick()

    expect(invokeMock).toHaveBeenCalledTimes(1)
    expect(wrapper.find('video').element.getAttribute('src')).toContain('https://cdn.example.com/live/index.m3u8')
    expect(invokeMock).toHaveBeenCalled()
  })

  it('reuses cached detail data instead of refetching on player entry', async () => {
    route.params.id = '42'
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 1,
            episode_label: '第1集',
            play_url: 'https://cdn.example.com/ep1',
            order_index: 0,
          },
        ],
      },
    ]

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    mount(PlayerPage, {
      global: {
        stubs: {
          SourceBadge: true,
        },
      },
    })

    await flushPromises()

    expect(detailStore.fetchDetail).not.toHaveBeenCalled()
  })

  it('hydrates detail data from the pending playback context', async () => {
    playerStore.pendingVodDetail = {
      item: {
        id: 42,
        title: '测试影片',
        item_type: 'series',
      } as CatalogDetailItem,
      episode_groups: [
        {
          source_name: 'demo',
          episodes: [
            {
              id: 1,
              episode_label: '第1集',
              play_url: 'https://cdn.example.com/ep1',
              order_index: 0,
            },
          ],
        },
      ],
    }

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    mount(PlayerPage, {
      global: {
        stubs: {
          SourceBadge: true,
        },
      },
    })

    await flushPromises()

    expect(detailStore.item?.id).toBe(42)
    expect(detailStore.episodeGroups).toHaveLength(1)
    expect(detailStore.fetchDetail).not.toHaveBeenCalled()
    expect(playerStore.setPendingVodDetail).toHaveBeenCalledWith(null)
  })

  it('uses preloaded direct playback targets without resolving again', async () => {
    route.query.detailUrl = 'https://example.com/detail'
    route.query.episode = encodeURIComponent('https://example.com/play-page')
    route.query.episodeTargets = JSON.stringify([
      {
        episode_id: 1,
        source_key: 'demo',
        target_url: 'https://cdn.example.com/movie.mp4',
        target_kind: 'Direct',
        resolver_key: null,
        headers: null,
        referer: null,
        sort_hint: 0,
        meta: '线路 1',
      },
    ])

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    mount(PlayerPage, {
      global: {
        stubs: {
          SourceBadge: true,
        },
      },
    })

    await flushPromises()

    expect(playbackStore.resolve).not.toHaveBeenCalled()
  })

  it('keeps the episode drawer interactive while the active episode is resolving', async () => {
    route.params.id = '42'
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 1,
            episode_label: '第1集',
            play_url: 'https://cdn.example.com/ep1',
            order_index: 0,
          },
        ],
      },
    ]
    playbackStore.status = 'resolving'
    playerStore.pendingUnifiedEpisode = {
      normalizedIndex: 1,
      displayLabel: '第1集',
      sources: [
        {
          sourceKey: 'demo',
          sourceName: 'demo',
          episode: detailStore.episodeGroups[0]!.episodes[0]!,
        },
      ],
    }

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          SourceBadge: true,
        },
      },
    })

    await flushPromises()

    expect(wrapper.find('.playback-drawer-stub').attributes('data-loading')).toBe('false')
  })

  it('starts playback for the selected episode when switching episodes', async () => {
    route.params.id = '42'
    route.query.detailUrl = 'https://example.com/detail'
    route.query.episodeTargets = JSON.stringify([
      {
        episode_id: 2,
        source_key: 'demo',
        target_url: 'https://cdn.example.com/movie.mp4',
        target_kind: 'Direct',
        resolver_key: null,
        headers: null,
        referer: null,
        sort_hint: 0,
        meta: '线路 1',
      },
    ])
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 1,
            episode_label: '第1集',
            play_url: 'https://cdn.example.com/ep1',
            order_index: 0,
          },
          {
            id: 2,
            episode_label: '第2集',
            play_url: 'https://cdn.example.com/ep2',
            order_index: 1,
          },
        ],
      },
    ]
    markPlaybackHealth({
      scope: 'source',
      key: 'demo|https://cdn.example.com/ep2',
      status: 'failed',
      reason: 'stale failure',
    })
    playbackStore.resolve.mockResolvedValue({
      status: 'ready',
      candidates: [
        {
          url: 'https://cdn.example.com/movie.mp4',
          label: '线路 1',
          kind: 'http',
        },
      ],
      errorMessage: null,
    })

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          PlaybackDrawer: {
            props: ['unifiedEpisodes'],
            emits: ['selectEpisode'],
            template: `
              <aside class="playback-drawer-stub">
                <button class="select-next-episode" type="button" @click="$emit('selectEpisode', unifiedEpisodes[1])">
                  next
                </button>
              </aside>
            `,
          },
          SourceBadge: true,
        },
      },
    })

    await flushPromises()
    playbackStore.resolve.mockClear()
    router.replace.mockClear()

    await wrapper.find('.select-next-episode').trigger('click')
    await flushPromises()

    expect(router.replace).toHaveBeenCalled()
    expect(router.replace).toHaveBeenCalledWith({
      path: '/player/vod/42',
      query: expect.objectContaining({
        source: 'demo',
        title: '测试影片',
        detailUrl: 'https://example.com/detail',
        episodeTargets: JSON.stringify([
          {
            episode_id: 2,
            source_key: 'demo',
            target_url: 'https://cdn.example.com/movie.mp4',
            target_kind: 'Direct',
            resolver_key: null,
            headers: null,
            referer: null,
            sort_hint: 0,
            meta: '线路 1',
          },
        ]),
        episode: 'https://cdn.example.com/ep2',
        episodeId: '2',
      }),
    })
    expect(playbackStore.resolve).toHaveBeenCalledWith('https://cdn.example.com/ep2', 2, true)
  })

  it('re-resolves when switching back to a previously played episode', async () => {
    route.params.id = '42'
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 1,
            episode_label: '第1集',
            play_url: 'https://cdn.example.com/ep1',
            order_index: 0,
          },
          {
            id: 2,
            episode_label: '第2集',
            play_url: 'https://cdn.example.com/ep2',
            order_index: 1,
          },
        ],
      },
    ]
    playbackStore.resolve.mockResolvedValue({
      status: 'ready',
      candidates: [
        {
          url: 'https://cdn.example.com/movie.mp4',
          label: '线路 1',
          kind: 'http',
        },
      ],
      errorMessage: null,
    })

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          PlaybackDrawer: {
            props: ['unifiedEpisodes'],
            emits: ['selectEpisode'],
            template: `
              <aside class="playback-drawer-stub">
                <button class="select-first-episode" type="button" @click="$emit('selectEpisode', unifiedEpisodes[0])">
                  first
                </button>
                <button class="select-second-episode" type="button" @click="$emit('selectEpisode', unifiedEpisodes[1])">
                  second
                </button>
              </aside>
            `,
          },
          SourceBadge: true,
        },
      },
    })

    await flushPromises()
    playbackStore.resolve.mockClear()
    router.replace.mockClear()

    await wrapper.find('.select-second-episode').trigger('click')
    await flushPromises()
    await wrapper.find('.select-first-episode').trigger('click')
    await flushPromises()

    expect(playbackStore.resolve).toHaveBeenNthCalledWith(1, 'https://cdn.example.com/ep2', 2, true)
    expect(playbackStore.resolve).toHaveBeenNthCalledWith(2, 'https://cdn.example.com/ep1', 1, true)
  })

  it('forces refresh through play-page indirection when switching episodes', async () => {
    route.params.id = '42'
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 1,
            episode_label: '第1集',
            play_url: 'https://cdn.example.com/ep1',
            order_index: 0,
          },
          {
            id: 2,
            episode_label: '第2集',
            play_url: 'https://cdn.example.com/ep2',
            order_index: 1,
          },
        ],
      },
    ]
    playbackStore.resolve.mockImplementation(async (...args: unknown[]) => {
      const input = String(args[0] ?? '')
      void args[1]
      void args[2]

      if (input === 'https://cdn.example.com/ep2') {
        return {
          status: 'ready',
          candidates: [
            {
              url: 'https://www.zxzjhd.com/vodplay/4627-1-1.html',
              label: '线路 1',
              kind: 'http',
            },
          ],
          errorMessage: null,
        }
      }

      return {
        status: 'ready',
        candidates: [
          {
            url: 'https://cdn.example.com/final.m3u8',
            label: '线路 2',
            kind: 'hls',
          },
        ],
        errorMessage: null,
      }
    })

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          PlaybackDrawer: {
            props: ['unifiedEpisodes'],
            emits: ['selectEpisode'],
            template: `
              <aside class="playback-drawer-stub">
                <button class="select-second-episode" type="button" @click="$emit('selectEpisode', unifiedEpisodes[1])">
                  second
                </button>
              </aside>
            `,
          },
          SourceBadge: true,
        },
      },
    })

    await flushPromises()
    playbackStore.resolve.mockClear()
    router.replace.mockClear()

    await wrapper.find('.select-second-episode').trigger('click')
    await flushPromises()

    expect(playbackStore.resolve).toHaveBeenNthCalledWith(1, 'https://cdn.example.com/ep2', 2, true)
    expect(playbackStore.resolve).toHaveBeenNthCalledWith(2, 'https://www.zxzjhd.com/vodplay/4627-1-1.html', undefined, false)
  })

  it('keeps episode switching working across repeated jumps between episodes', async () => {
    route.params.id = '42'
    detailStore.item = {
      id: 42,
      title: '测试影片',
      item_type: 'series',
    } as CatalogDetailItem
    detailStore.episodeGroups = [
      {
        source_name: 'demo',
        episodes: [
          {
            id: 4,
            episode_label: '第4集',
            play_url: 'https://cdn.example.com/ep4',
            order_index: 0,
          },
          {
            id: 5,
            episode_label: '第5集',
            play_url: 'https://cdn.example.com/ep5',
            order_index: 1,
          },
          {
            id: 6,
            episode_label: '第6集',
            play_url: 'https://cdn.example.com/ep6',
            order_index: 2,
          },
        ],
      },
    ]
    playbackStore.resolve.mockImplementation(async (...args: unknown[]) => {
      const input = String(args[0] ?? '')

      if (input === 'https://cdn.example.com/ep6') {
        return {
          status: 'ready',
          candidates: [
            {
              url: 'https://www.zxzjhd.com/vodplay/4627-1-1.html',
              label: '线路 6',
              kind: 'http',
            },
          ],
          errorMessage: null,
        }
      }

      if (input === 'https://www.zxzjhd.com/vodplay/4627-1-1.html') {
        return {
          status: 'ready',
          candidates: [
            {
              url: 'https://cdn.example.com/final6.m3u8',
              label: '线路 6-1',
              kind: 'hls',
            },
          ],
          errorMessage: null,
        }
      }

      return {
        status: 'ready',
        candidates: [
          {
            url: `${input}.m3u8`,
            label: '线路',
            kind: 'hls',
          },
        ],
        errorMessage: null,
      }
    })

    const { default: PlayerPage } = await import('@/views/PlayerPage.vue')
    const wrapper = mount(PlayerPage, {
      global: {
        stubs: {
          PlaybackDrawer: {
            props: ['unifiedEpisodes'],
            emits: ['selectEpisode'],
            template: `
              <aside class="playback-drawer-stub">
                <button class="select-5" type="button" @click="$emit('selectEpisode', unifiedEpisodes[1])">5</button>
                <button class="select-6" type="button" @click="$emit('selectEpisode', unifiedEpisodes[2])">6</button>
                <button class="select-4" type="button" @click="$emit('selectEpisode', unifiedEpisodes[0])">4</button>
              </aside>
            `,
          },
          SourceBadge: true,
        },
      },
    })

    await flushPromises()
    playbackStore.resolve.mockClear()

    await wrapper.find('.select-5').trigger('click')
    await flushPromises()
    await wrapper.find('.select-6').trigger('click')
    await flushPromises()
    await wrapper.find('.select-4').trigger('click')
    await flushPromises()

    expect(playbackStore.resolve).toHaveBeenNthCalledWith(1, 'https://cdn.example.com/ep5', 5, true)
    expect(playbackStore.resolve).toHaveBeenNthCalledWith(2, 'https://cdn.example.com/ep6', 6, true)
    expect(playbackStore.resolve).toHaveBeenNthCalledWith(3, 'https://www.zxzjhd.com/vodplay/4627-1-1.html', undefined, false)
    expect(playbackStore.resolve).toHaveBeenNthCalledWith(4, 'https://cdn.example.com/ep4', 4, true)
  })
})
