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
      props: { ...baseProps, itemType: 'movie', sourceDetails },
    })
    expect(wrapper.find('.movie-action-panel').exists()).toBe(false)
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })

  it('renders EpisodeGrid for series when source detail provided', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails,
      },
    })
    expect(wrapper.find('.episode-grid').exists()).toBe(true)
    expect(wrapper.find('.movie-action-panel').exists()).toBe(false)
  })

  it('shows load button for series when no detail for selected source', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series' },
    })
    expect(wrapper.find('.load-episodes-btn').exists()).toBe(true)
    expect(wrapper.find('.episode-grid').exists()).toBe(false)
  })

  it('shows loading placeholder when selected source is loading', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        loadingSources: ['s1'],
      },
    })
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
  })

  it('deduplicates source name from movie episode label', () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'movie',
        sources: [{ source: 's1', source_name: '文才影视', detail_url: 'url1' }],
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
    const firstBtn = wrapper.find('.source-btn')
    expect(firstBtn.text()).toBe('文才HD')
    expect(firstBtn.text()).not.toContain('文才 ·')
  })

  it('prefixes source short name when episode label does not contain it', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'movie', sourceDetails },
    })
    const firstBtn = wrapper.find('.source-btn')
    expect(firstBtn.text()).toBe('来源 · 01')
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
  })

  it('skips empty sources for movies', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'movie', sourceDetails },
    })
    expect(wrapper.findAll('.source-btn').length).toBe(2)
  })

  it('emits play-episode when a movie episode button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'movie', sourceDetails },
    })
    await wrapper.find('.source-btn').trigger('click')
    expect(wrapper.emitted('play-episode')).toHaveLength(1)
    expect(wrapper.emitted('play-episode')![0]).toEqual([episodes[0], 's1'])
  })

  it('emits play-episode with sourceKey when EpisodeGrid plays', async () => {
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

  it('emits select-source when source button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails,
      },
    })
    const buttons = wrapper.findAll('.source-btn')
    expect(buttons.length).toBe(2)
    await buttons[1].trigger('click')
    expect(wrapper.emitted('select-source')).toHaveLength(1)
    expect(wrapper.emitted('select-source')![0]).toEqual(['s2'])
  })

  it('emits select-source when load button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series' },
    })
    await wrapper.find('.load-episodes-btn').trigger('click')
    expect(wrapper.emitted('select-source')).toHaveLength(1)
    expect(wrapper.emitted('select-source')![0]).toEqual(['s1'])
  })

  it('displays correct type label', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'variety' },
    })
    expect(wrapper.find('.card-type-tag').text()).toBe('综艺')
  })

  it('switches source and updates displayed episodes', async () => {
    const wrapper = mount(SearchResultCard, {
      props: {
        ...baseProps,
        itemType: 'series',
        sourceDetails,
      },
    })
    // Initially shows s1 episodes
    expect(wrapper.findAll('.episode-chip').length).toBe(episodes.length)

    // Click s2 source button
    const buttons = wrapper.findAll('.source-btn')
    await buttons[1].trigger('click')

    // s2 has empty episodes, so should show load button
    expect(wrapper.find('.episode-grid').exists()).toBe(false)
    expect(wrapper.find('.load-episodes-btn').exists()).toBe(true)
  })
})
