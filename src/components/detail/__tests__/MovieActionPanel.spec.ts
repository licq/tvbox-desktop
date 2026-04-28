import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import MovieActionPanel from '../MovieActionPanel.vue'

function makeSources(count: number) {
  return Array.from({ length: count }, (_, i) => ({
    source: `src${i + 1}`,
    source_name: `来源${i + 1}`,
    detail_url: `url${i + 1}`,
  }))
}

describe('MovieActionPanel', () => {
  it('renders primary play button and extra source buttons', () => {
    const wrapper = mount(MovieActionPanel, {
      props: { sources: makeSources(3) },
    })
    expect(wrapper.find('.play-btn-primary').exists()).toBe(true)
    expect(wrapper.find('.play-btn-primary').text()).toContain('立即播放')
    expect(wrapper.findAll('.play-btn-secondary')).toHaveLength(2)
  })

  it('shows +N when there are more than 3 sources', () => {
    const wrapper = mount(MovieActionPanel, {
      props: { sources: makeSources(5) },
    })
    const moreBtn = wrapper.find('.play-btn-more')
    expect(moreBtn.exists()).toBe(true)
    expect(moreBtn.text()).toBe('+2')
  })

  it('emits play with first source when primary button is clicked', async () => {
    const sources = makeSources(2)
    const wrapper = mount(MovieActionPanel, {
      props: { sources },
    })
    await wrapper.find('.play-btn-primary').trigger('click')
    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([sources[0].source, sources[0].detail_url])
  })

  it('emits play with correct source when secondary button is clicked', async () => {
    const sources = makeSources(3)
    const wrapper = mount(MovieActionPanel, {
      props: { sources },
    })
    await wrapper.findAll('.play-btn-secondary')[0].trigger('click')
    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([sources[1].source, sources[1].detail_url])
  })
})
