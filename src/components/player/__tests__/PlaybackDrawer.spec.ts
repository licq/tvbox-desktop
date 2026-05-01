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
    expect(wrapper.text()).toContain('量子线路')
    expect(wrapper.text()).toContain('manifest failed')
    expect(wrapper.text()).not.toContain('当前播放')
    expect(wrapper.text()).not.toContain('最近失败')
  })

  it('does not expose the raw empty-candidate failure reason in series mode', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放中',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: [
          {
            ...attempts[0]!,
            status: 'failed',
            failureReason: '当前源没有可用候选线路',
          },
        ],
      },
    })

    expect(wrapper.text()).not.toContain('当前源没有可用候选线路')
    expect(wrapper.text()).toContain('本次失败')
  })

  it('hides the status badge when the drawer is in a failed state', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '播放失败',
        statusTone: 'danger',
        itemType: 'movie',
      },
    })

    expect(wrapper.find('.playback-header .source-badge').exists()).toBe(false)
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

  it('shows loading state instead of empty playback text while loading', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: [],
        currentIndex: 0,
        failedIndexes: [],
        status: '加载中',
        errorMessage: '当前源没有可用候选线路',
        itemType: 'movie',
        loading: true,
      },
    })

    expect(wrapper.find('.playback-loading').exists()).toBe(true)
    expect(wrapper.findAll('.skeleton-card').length).toBeGreaterThan(0)
    expect(wrapper.text()).not.toContain('没有可用线路')
    expect(wrapper.text()).not.toContain('当前源没有可用候选线路')
  })

  it('keeps already loaded episode content visible while playback is still resolving', () => {
    const wrapper = mount(PlaybackDrawer, {
      props: {
        sources: attempts[0]!.candidates,
        currentIndex: 0,
        failedIndexes: [],
        status: '正在解析本集线路',
        unifiedEpisodes,
        currentNormalizedIndex: 3,
        itemType: 'series',
        episodeSourceAttempts: attempts,
        loading: true,
      },
    })

    expect(wrapper.find('.playback-loading').exists()).toBe(false)
    expect(wrapper.text()).toContain('第3集')
    expect(wrapper.text()).toContain('本集播放源')
    expect(wrapper.text()).toContain('非凡线路')
    expect(wrapper.text()).toContain('量子线路')
  })
})
