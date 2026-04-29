import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGroupPanel from '../EpisodeGroupPanel.vue'
import type { CatalogEpisodeGroup } from '@/types'

const mockGroups: CatalogEpisodeGroup[] = [
  {
    source_name: 'Source A',
    episodes: [
      { id: 1, episode_label: '第1集', play_url: 'http://a/1', order_index: 1 },
      { id: 2, episode_label: '第2集', play_url: 'http://a/2', order_index: 2 },
    ],
  },
  {
    source_name: 'Source B',
    episodes: [
      { id: 3, episode_label: '第01集', play_url: 'http://b/1', order_index: 1 },
      { id: 4, episode_label: '第02集', play_url: 'http://b/2', order_index: 2 },
    ],
  },
]

describe('EpisodeGroupPanel', () => {
  it('merges duplicate episodes across sources for series', () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    const chips = wrapper.findAll('.episode-chip')
    expect(chips.length).toBe(2)
  })

  it('shows source count badge when episode has multiple sources', () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    const badges = wrapper.findAll('.episode-chip-badge')
    expect(badges.length).toBe(2)
    expect(badges[0].text()).toBe('2源')
  })

  it('emits unified episode on chip click', async () => {
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: mockGroups, item_type: 'series' }
    })
    await wrapper.find('.episode-chip').trigger('click')
    expect(wrapper.emitted('play')).toHaveLength(1)
    const emitted = wrapper.emitted('play')![0][0] as { normalizedIndex: number; sources: unknown[] }
    expect(emitted.normalizedIndex).toBe(1)
    expect(emitted.sources).toHaveLength(2)
  })

  it('does not merge episodes for movies', () => {
    const movieGroups: CatalogEpisodeGroup[] = [
      {
        source_name: 'Source A',
        episodes: [
          { id: 1, episode_label: 'HD', play_url: 'http://a/hd', order_index: 1 },
          { id: 2, episode_label: '1080P', play_url: 'http://a/1080', order_index: 2 },
        ],
      },
    ]
    const wrapper = mount(EpisodeGroupPanel, {
      props: { groups: movieGroups, item_type: 'movie' }
    })
    const buttons = wrapper.findAll('.play-button')
    expect(buttons.length).toBe(2)
  })
})
