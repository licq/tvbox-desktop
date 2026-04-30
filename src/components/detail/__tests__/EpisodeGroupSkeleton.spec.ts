import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGroupSkeleton from '../EpisodeGroupSkeleton.vue'

describe('EpisodeGroupSkeleton', () => {
  it('mirrors the episode group card layout while loading', () => {
    const wrapper = mount(EpisodeGroupSkeleton, {
      props: { count: 6 },
    })

    expect(wrapper.find('.source-group-card').exists()).toBe(true)
    expect(wrapper.find('.source-group-header').exists()).toBe(true)
    expect(wrapper.find('.source-group-body').exists()).toBe(true)
    expect(wrapper.findAll('.skeleton-chip').length).toBe(6)
  })
})
