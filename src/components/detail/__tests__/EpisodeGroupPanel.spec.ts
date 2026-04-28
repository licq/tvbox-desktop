import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGroupPanel from '../EpisodeGroupPanel.vue'
import type { CatalogEpisode, CatalogEpisodeGroup, CatalogItemType } from '@/types'

function makeEpisodes(count: number): CatalogEpisode[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    episode_label: `第${i + 1}集`,
    play_url: `http://test.com/ep${i + 1}`,
    order_index: i + 1,
  }))
}

function makeGroup(episodes: CatalogEpisode[], sourceName = '测试源'): CatalogEpisodeGroup {
  return { source_name: sourceName, episodes }
}

describe('EpisodeGroupPanel', () => {
  it('renders play buttons directly for movies', () => {
    const group = makeGroup([
      { id: 1, episode_label: 'HD', play_url: 'http://a', order_index: 1 },
      { id: 2, episode_label: '1080P', play_url: 'http://b', order_index: 2 },
    ])

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    const buttons = wrapper.findAll('.play-button')
    expect(buttons).toHaveLength(2)
    expect(buttons[0].text()).toContain('HD')
    expect(buttons[1].text()).toContain('1080P')

    expect(wrapper.find('.episode-chip-grid').exists()).toBe(false)
    expect(wrapper.find('.expand-toggle-button').exists()).toBe(false)
  })

  it('renders all chips directly for series with <=24 episodes', () => {
    const episodes = makeEpisodes(12)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(12)

    expect(wrapper.find('.expand-toggle-button').exists()).toBe(false)
  })

  it('renders first 24 chips + expand button for series with >24 episodes', () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(24)

    const expandBtn = wrapper.find('.expand-toggle-button')
    expect(expandBtn.exists()).toBe(true)
    expect(expandBtn.text()).toContain('展开剩余 6 集')
  })

  it('expands to show all chips when expand button is clicked', async () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.find('.expand-toggle-button').trigger('click')

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(30)

    const collapseBtn = wrapper.find('.expand-toggle-button')
    expect(collapseBtn.text()).toContain('收起')
  })

  it('collapses back to 24 chips when collapse button is clicked', async () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.find('.expand-toggle-button').trigger('click') // expand
    await wrapper.find('.expand-toggle-button').trigger('click') // collapse

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(24)

    expect(wrapper.find('.expand-toggle-button').text()).toContain('展开剩余 6 集')
  })

  it('emits play event with episode when play button is clicked (movie)', async () => {
    const group = makeGroup([
      { id: 1, episode_label: 'HD', play_url: 'http://a', order_index: 1 },
    ])

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    await wrapper.find('.play-button').trigger('click')

    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([group.episodes[0]])
  })

  it('emits play event with episode when chip is clicked (series)', async () => {
    const group = makeGroup(makeEpisodes(5))

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.findAll('.episode-chip')[2].trigger('click')

    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([group.episodes[2]])
  })

  it('shows source name and type tag in header', () => {
    const group = makeGroup(makeEpisodes(5), '来源A')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-name').text()).toBe('来源A')
    expect(wrapper.find('.source-group-type-tag').text()).toBe('剧集')
  })

  it('shows correct count badge for movies', () => {
    const group = makeGroup(makeEpisodes(3), '来源B')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-count-badge').text()).toBe('3 个播放源')
  })

  it('shows correct count badge for series', () => {
    const group = makeGroup(makeEpisodes(8), '来源C')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-count-badge').text()).toBe('8 集')
  })
})
