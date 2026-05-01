import { mount, flushPromises } from '@vue/test-utils'
import { beforeEach, afterEach, describe, expect, it, vi } from 'vitest'

const route = {
  params: {
    itemId: '0',
  },
  query: {
    search: '1',
    keyword: '黑夜告白',
  } as Record<string, string | undefined>,
}

const router = {
  back: vi.fn(),
  push: vi.fn(),
}

const detailStore = {
  item: null,
  episodeGroups: [],
  loading: false,
  fetchDetail: vi.fn(),
}

const libraryStore = {
  doubanHot: [],
  fetchDoubanHotByType: vi.fn(),
}

const playerStore = {
  pendingUnifiedEpisode: null,
  pendingVodDetail: null,
  setPendingUnifiedEpisode: vi.fn(),
  setPendingVodDetail: vi.fn(),
}

let providerDetailResolve: ((value: unknown) => void) | null = null

vi.mock('vue-router', () => ({
  useRoute: () => route,
  useRouter: () => router,
}))

vi.mock('@/stores/detail', () => ({
  useDetailStore: () => detailStore,
}))

vi.mock('@/stores/library', () => ({
  useLibraryStore: () => libraryStore,
}))

vi.mock('@/stores/player', () => ({
  usePlayerStore: () => playerStore,
}))

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === 'search_all_sources') {
      return [
        {
          source_key: 'demo',
          source_name: 'Demo 源',
          items: [
            {
              source_item_key: 'detail-1',
              title: '黑夜告白',
              item_type: 'series',
              poster: 'https://example.com/poster.jpg',
              episodes: [],
            },
          ],
        },
      ]
    }

    if (command === 'provider_detail') {
      return new Promise(resolve => {
        providerDetailResolve = resolve
      })
    }

    if (command === 'search_douban_subject_by_keyword') {
      return null
    }

    return null
  }),
}))

vi.mock('@/components/LoadingSpinner.vue', () => ({
  default: {
    name: 'LoadingSpinner',
    template: '<div class="loading-spinner-stub"></div>',
  },
}))

vi.mock('@/components/detail/DoubanMetaPanel.vue', () => ({
  default: {
    name: 'DoubanMetaPanel',
    template: '<div class="douban-meta-panel-stub"></div>',
  },
}))

vi.mock('@/components/detail/DetailMetaSkeleton.vue', () => ({
  default: {
    name: 'DetailMetaSkeleton',
    template: '<div class="detail-meta-skeleton-stub"></div>',
  },
}))

vi.mock('@/components/detail/EpisodeGroupSkeleton.vue', () => ({
  default: {
    name: 'EpisodeGroupSkeleton',
    template: '<div class="episode-group-skeleton-stub"></div>',
  },
}))

vi.mock('@/components/detail/EpisodeGroupPanel.vue', () => ({
  default: {
    name: 'EpisodeGroupPanel',
    props: ['groups', 'item_type'],
    template: '<div class="episode-group-panel-stub"></div>',
  },
}))

describe('VodDetail search loading', () => {
  beforeEach(() => {
    route.params.itemId = '0'
    route.query.search = '1'
    route.query.keyword = '黑夜告白'
    router.back.mockClear()
    router.push.mockClear()
    detailStore.fetchDetail.mockClear()
    libraryStore.fetchDoubanHotByType.mockClear()
    playerStore.setPendingUnifiedEpisode.mockClear()
    playerStore.setPendingVodDetail.mockClear()
    providerDetailResolve = null
  })

  afterEach(() => {
    providerDetailResolve = null
  })

  it('shows cached search cards before provider detail preloading finishes', async () => {
    const { default: VodDetail } = await import('@/views/VodDetail.vue')
    const wrapper = mount(VodDetail)

    await flushPromises()
    await Promise.resolve()

    expect(wrapper.find('.search-result-card').exists()).toBe(true)
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)

    providerDetailResolve?.({
      title: '黑夜告白',
      poster: null,
      summary: null,
      episodes: [
        {
          id: 1,
          episode_label: '第1集',
          play_url: 'https://example.com/ep1',
          order_index: 0,
        },
      ],
    })

    await flushPromises()

    expect(wrapper.find('.episode-grid').exists()).toBe(true)
  })
})
