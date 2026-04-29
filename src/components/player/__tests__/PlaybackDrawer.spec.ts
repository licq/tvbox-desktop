import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import PlaybackDrawer from '@/components/player/PlaybackDrawer.vue'
import type { PlaybackSourceAttempt, UnifiedEpisode } from '@/types'

const unifiedEpisodes: UnifiedEpisode[] = [{
  normalizedIndex: 3,
  displayLabel: '第3集',
  sources: [
    { sourceKey: 'a', sourceName: '非凡线路', episode: { id: 1, episode_label: '第03集', play_url: 'a', order_index: 0 } },
    { sourceKey: 'b', sourceName: '量子线路', episode: { id: 2, episode_label: '第03集', play_url: 'b', order_index: 1 } },
  ],
}]

const attempts: PlaybackSourceAttempt[] = [
  {
    source: unifiedEpisodes[0]!.sources[0]!,
    status: 'playing',
    candidates: [{ url: 'https://a.example/1.m3u8', label: 'HLS', kind: 'hls' }],
    failedCandidateIndexes: [],
  },
  {
    source: unifiedEpisodes[0]!.sources[1]!,
    status: 'failed',
    candidates: [],
    failedCandidateIndexes: [],
    failureReason: 'manifest failed',
  },
]

describe('PlaybackDrawer', () => {
  it('renders current episode source attempts in series mode', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放中',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: attempts,
      },
    })

    expect(wrapper.text()).toContain('本集播放源')
    expect(wrapper.text()).toContain('非凡线路')
    expect(wrapper.text()).toContain('当前播放')
    expect(wrapper.text()).toContain('量子线路')
    expect(wrapper.text()).toContain('manifest failed')
  })

  it('emits switchEpisodeSource when clicking an episode source', async () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放中',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: attempts,
      },
    })

    await wrapper.find('[data-testid="episode-source-b"]').trigger('click')

    expect(wrapper.emitted('switchEpisodeSource')?.[0]).toEqual(['b'])
  })
})
