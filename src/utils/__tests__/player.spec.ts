import { describe, expect, it } from 'vitest'
import { describeMediaErrorCode, describePlaybackFailure, formatPlayerTitle, isAutoplayBlocked } from '@/utils/player'

describe('player utils', () => {
  it('detects autoplay blocking errors', () => {
    expect(isAutoplayBlocked({ name: 'NotAllowedError' })).toBe(true)
    expect(isAutoplayBlocked(new Error('NotAllowedError: play() failed'))).toBe(true)
    expect(isAutoplayBlocked(new Error('network failed'))).toBe(false)
  })

  it('maps media error codes to readable messages', () => {
    expect(describeMediaErrorCode(2)).toBe('网络错误')
    expect(describeMediaErrorCode(4)).toBe('浏览器不支持当前媒体格式')
    expect(describeMediaErrorCode(99)).toBe('媒体播放失败')
  })

  it('treats autoplay rejection as a non-fatal playback message', () => {
    expect(describePlaybackFailure({ name: 'NotAllowedError' })).toBe('线路已加载，点击播放开始')
    expect(describePlaybackFailure(new Error('network failed'))).toBe('network failed')
  })
})

describe('player title formatting', () => {
  it('formats series title with episode label', () => {
    expect(formatPlayerTitle({ title: '庆余年', episodeLabel: '第03集' })).toBe('庆余年 · 第03集')
  })

  it('falls back to episode label when title is missing', () => {
    expect(formatPlayerTitle({ episodeLabel: '第03集' })).toBe('第03集')
  })

  it('uses source label only as a final fallback', () => {
    expect(formatPlayerTitle({ sourceLabel: '非凡线路' })).toBe('非凡线路')
  })
})
