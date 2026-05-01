import { describe, expect, it } from 'vitest'
import {
  describeMediaErrorCode,
  describePlaybackFailure,
  formatPlayerTitle,
  isAutoplayBlocked,
  isProviderDirectPlaybackRoute,
  parsePlaybackHeaders,
  isDirectMediaUrl,
  isPlaybackPageUrl,
  shouldFallbackToBrowserHls,
} from '@/utils/player'

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

describe('provider playback routing', () => {
  it('detects a provider direct playback route', () => {
    expect(isProviderDirectPlaybackRoute({
      mode: 'vod',
      itemId: 0,
      source: 'ypanso',
      detailUrl: 'https://example.com/detail',
      episodeUrl: 'https://example.com/video/index.m3u8',
    })).toBe(true)
  })

  it('does not treat a normal vod route as provider direct playback', () => {
    expect(isProviderDirectPlaybackRoute({
      mode: 'vod',
      itemId: 123,
      source: 'ypanso',
      detailUrl: 'https://example.com/detail',
      episodeUrl: 'https://example.com/video/index.m3u8',
    })).toBe(false)
  })

  it('parses provider playback headers from JSON', () => {
    expect(parsePlaybackHeaders('{"Referer":"https://example.com","Origin":"https://example.com"}')).toEqual({
      Referer: 'https://example.com',
      Origin: 'https://example.com',
    })
    expect(parsePlaybackHeaders('{bad json')).toBeNull()
  })

  it('detects direct media urls but not play pages', () => {
    expect(isDirectMediaUrl('https://cdn.example.com/live/index.m3u8')).toBe(true)
    expect(isDirectMediaUrl('https://www.xb6v.com/e/DownSys/play/?classid=6&id=28451&pathid1=0&bf=0')).toBe(false)
  })

  it('detects play page urls', () => {
    expect(isPlaybackPageUrl('https://www.xb6v.com/e/DownSys/play/?classid=6&id=28451&pathid1=0&bf=0')).toBe(true)
    expect(isPlaybackPageUrl('https://www.zxzjhd.com/vodplay/4627-1-1.html')).toBe(true)
    expect(isPlaybackPageUrl('https://cdn.example.com/live/index.m3u8')).toBe(false)
  })

  it('falls back to browser hls only for transport-style failures', () => {
    expect(shouldFallbackToBrowserHls(new Error('error sending request for url (https://example.com)'))).toBe(true)
    expect(shouldFallbackToBrowserHls(new Error('tls handshake eof'))).toBe(true)
    expect(shouldFallbackToBrowserHls(new Error('A network error (status 0) occurred while loading manifest'))).toBe(true)
    expect(shouldFallbackToBrowserHls(new Error('playlist request failed: 403 Forbidden'))).toBe(false)
  })
})
