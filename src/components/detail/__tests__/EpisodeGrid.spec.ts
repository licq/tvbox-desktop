import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGrid from '../EpisodeGrid.vue'
import type { CatalogEpisode } from '@/types'

function makeEpisodes(count: number): CatalogEpisode[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    episode_label: `${i + 1}`,
    play_url: `http://test.com/ep${i + 1}`,
    order_index: i + 1,
  }))
}

describe('EpisodeGrid', () => {
  it('renders all episodes when count is within visible limit', () => {
    const episodes = makeEpisodes(8)
    const wrapper = mount(EpisodeGrid, {
      props: { episodes, visibleCount: 12 },
    })
    expect(wrapper.findAll('.episode-chip')).toHaveLength(8)
    expect(wrapper.find('.episode-chip-more').exists()).toBe(false)
  })

  it('truncates to visibleCount and shows more button', () => {
    const episodes = makeEpisodes(20)
    const wrapper = mount(EpisodeGrid, {
      props: { episodes, visibleCount: 12 },
    })
    expect(wrapper.findAll('.episode-chip:not(.episode-chip-more):not(.episode-chip-collapse)')).toHaveLength(12)
    expect(wrapper.find('.episode-chip-more').exists()).toBe(true)
  })

  it('expands to show all episodes when more button is clicked', async () => {
    const episodes = makeEpisodes(20)
    const wrapper = mount(EpisodeGrid, {
      props: { episodes, visibleCount: 12 },
    })
    await wrapper.find('.episode-chip-more').trigger('click')
    expect(wrapper.findAll('.episode-chip:not(.episode-chip-more):not(.episode-chip-collapse)')).toHaveLength(20)
    expect(wrapper.find('.episode-chip-collapse').exists()).toBe(true)
  })

  it('collapses back when collapse button is clicked', async () => {
    const episodes = makeEpisodes(20)
    const wrapper = mount(EpisodeGrid, {
      props: { episodes, visibleCount: 12 },
    })
    await wrapper.find('.episode-chip-more').trigger('click')
    await wrapper.find('.episode-chip-collapse').trigger('click')
    expect(wrapper.findAll('.episode-chip:not(.episode-chip-more):not(.episode-chip-collapse)')).toHaveLength(12)
    expect(wrapper.find('.episode-chip-more').exists()).toBe(true)
  })

  it('emits play event with episode when chip is clicked', async () => {
    const episodes = makeEpisodes(5)
    const wrapper = mount(EpisodeGrid, {
      props: { episodes, visibleCount: 12 },
    })
    await wrapper.findAll('.episode-chip')[2].trigger('click')
    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([episodes[2]])
  })
})
