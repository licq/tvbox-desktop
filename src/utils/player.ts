export function isAutoplayBlocked(error: unknown): boolean {
  if (!error || typeof error !== 'object') return false
  const maybeError = error as { name?: string; message?: string }
  return maybeError.name === 'NotAllowedError' || maybeError.message?.includes('NotAllowedError') === true
}

export function describeMediaErrorCode(code?: number | null): string {
  switch (code) {
    case 1:
      return '播放被中止'
    case 2:
      return '网络错误'
    case 3:
      return '媒体解码失败'
    case 4:
      return '浏览器不支持当前媒体格式'
    default:
      return '媒体播放失败'
  }
}

export function describePlaybackFailure(error: unknown): string {
  if (isAutoplayBlocked(error)) {
    return '线路已加载，点击播放开始'
  }

  if (error instanceof Error && error.message.trim().length > 0) {
    return error.message
  }

  return '无法直接播放当前地址'
}
