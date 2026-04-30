import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import SearchResultCard from '../SearchResultCard.vue'
import type { CatalogEpisode, CatalogItemType } from '@/types'

const baseProps = {
  title: '测试影片',
  itemType: 'movie' as CatalogItemType,
  sources: [
    { source: 's1', source_name: '来源A', detail_url: 'url1' },
    { source: 's2', source_name: '来源B', detail_url: 'url2' },
  ],
}

const episodes: CatalogEpisode[] = [
  { id: 1, episode_label: '01', play_url: 'http://a/1', order_index: 1 },
  { id: 2, episode_label: '02', play_url: 'http://a/2', order_index: 2 },
]

const sourceDetails = {
  s1: { title: '测试影片', poster: null, summary: null, episodes },
  s2: { title: '测试影片', poster: null, summary: null, episodes: [] },
}

describe('SearchResultCard', () => {
  it('renders episode buttons for movies when source details provided', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    expect(wrapper.find('.movie-action-panel').exists()).toBe(false)
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })

  it('deduplicates source name from movie episode label', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        title: '测试影片',
        itemType: 'movie',
        sources: [
          { source: 's1', source_name: '文才', detail_url: 'url1' },
        ],
        sourceDetails: {
          s1: {
            title: '测试影片',
            poster: null,
            summary: null,
            episodes: [
              { id: 1, episode_label: '文才HD', play_url: 'http://a/1', order_index: 1 },
            ],
          },
        },
      },
    })
    const btn = wrapper.find('.source-btn')
    expect(btn.text()).toBe('文才HD')
    expect(btn.text()).not.toContain('文才 ·')
  })

  it('prefixes source short name when episode label does not contain it', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    const btn = wrapper.find('.source-btn')
    expect(btn.text()).toBe('来源A · 01')
  })

  it('shows loading placeholder for movie when a source is loading', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
        loadingSources: ['s1'],
      },
    })
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
    expect(wrapper.findAll('.loading-chip').length).toBeGreaterThan(0)
    expect(wrapper.text()).not.toContain('加载中')
  })

  it('skips empty sources for movies', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    // s1 has 2 episodes, s2 has 0 episodes => 2 buttons
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })

  it('emits play-episode when a movie episode button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sourceDetails,
      },
    })
    await wrapper.find('.source-btn').trigger('click')
    expect(wrapper.emitted('play-episode')).toHaveLength(1)
    expect(wrapper.emitted('play-episode')![0]).toEqual([episodes[0], 's1'])
  })

  it('renders EpisodeGrid when source detail provided (series)', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails,
      },
    })
    expect(wrapper.find('.episode-grid').exists()).toBe(true)
    expect(wrapper.findAll('.episode-chip').length).toBe(2)
  })

  it('shows loading placeholder when selected source is loading (series)', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        loadingSources: ['s1'],
      },
    })
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
    expect(wrapper.findAll('.loading-chip').length).toBeGreaterThan(0)
    expect(wrapper.text()).not.toContain('加载中')
  })

  it('keeps the series card in loading state until the current source is resolved', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
      },
    })

    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('暂无播放链接')
  })

  it('keeps the movie card in loading state until at least one source resolves', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
      },
    })

    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
    expect(wrapper.text()).not.toContain('暂无播放链接')
  })

  it('emits play-episode with sourceKey when EpisodeGrid plays (series)', async () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails,
      },
    })
    await wrapper.find('.episode-chip').trigger('click')
    expect(wrapper.emitted('play-episode')).toHaveLength(1)
    expect(wrapper.emitted('play-episode')![0]).toEqual([episodes[0], 's1'])
  })

  it('emits select-source when source button is clicked (series)', async () => {
    const twoSourceDetails = {
      s1: { title: '测试影片', poster: null, summary: null, episodes },
      s2: { title: '测试影片', poster: null, summary: null, episodes: [episodes[0]] },
    }
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails: twoSourceDetails,
      },
    })
    const buttons = wrapper.findAll('.source-btn')
    expect(buttons.length).toBe(2)
    await buttons[1].trigger('click')
    expect(wrapper.emitted('select-source')).toHaveLength(1)
    expect(wrapper.emitted('select-source')![0]).toEqual(['s2'])
  })

  it('displays correct type label', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'variety' },
    })
    expect(wrapper.find('.card-type-tag').text()).toBe('综艺')
  })

  it('switches source and updates displayed episodes (series)', async () => {
    const twoSourceDetails = {
      s1: { title: '测试影片', poster: null, summary: null, episodes },
      s2: { title: '测试影片', poster: null, summary: null, episodes: [] },
    }
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails: twoSourceDetails,
      },
    })
    // Initially shows s1 episodes
    expect(wrapper.findAll('.episode-chip').length).toBe(episodes.length)

    // s2 is hidden (0 episodes), so only 1 source button
    const buttons = wrapper.findAll('.source-btn')
    expect(buttons.length).toBe(1)
  })
})
