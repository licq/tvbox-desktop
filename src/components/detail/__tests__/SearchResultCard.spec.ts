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

describe('SearchResultCard', () => {
  it('renders MovieActionPanel for movies', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'movie' },
    })
    expect(wrapper.find('.movie-action-panel').exists()).toBe(true)
    expect(wrapper.find('.episode-grid').exists()).toBe(false)
  })

  it('renders EpisodeGrid for series when episodes are provided', () => {
    const episodes: CatalogEpisode[] = [
      { id: 1, episode_label: '01', play_url: 'http://a', order_index: 1 },
    ]
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series', episodes },
    })
    expect(wrapper.find('.episode-grid').exists()).toBe(true)
    expect(wrapper.find('.movie-action-panel').exists()).toBe(false)
  })

  it('shows load button for series when episodes are not loaded', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series' },
    })
    expect(wrapper.find('.load-episodes-btn').exists()).toBe(true)
  })

  it('shows loading placeholder when loadingEpisodes is true', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series', loadingEpisodes: true },
    })
    expect(wrapper.find('.loading-placeholder').exists()).toBe(true)
  })

  it('emits play-source when MovieActionPanel plays', async () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'movie' },
    })
    await wrapper.find('.play-btn-primary').trigger('click')
    expect(wrapper.emitted('play-source')).toHaveLength(1)
    expect(wrapper.emitted('play-source')![0]).toEqual(['s1', 'url1'])
  })

  it('emits play-episode when EpisodeGrid plays', async () => {
    const episodes: CatalogEpisode[] = [
      { id: 1, episode_label: '01', play_url: 'http://a', order_index: 1 },
    ]
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series', episodes },
    })
    await wrapper.find('.episode-chip').trigger('click')
    expect(wrapper.emitted('play-episode')).toHaveLength(1)
    expect(wrapper.emitted('play-episode')![0]).toEqual([episodes[0]])
  })

  it('emits load-episodes when load button is clicked', async () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'series' },
    })
    await wrapper.find('.load-episodes-btn').trigger('click')
    expect(wrapper.emitted('load-episodes')).toHaveLength(1)
  })

  it('displays correct type label', () => {
    const wrapper = mount(SearchResultCard, {
      props: { ...baseProps, itemType: 'variety' },
    })
    expect(wrapper.find('.card-type-tag').text()).toBe('综艺')
  })
})
