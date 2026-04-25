import { invoke } from '@tauri-apps/api/core'

/**
 * Get a Douban image URL, proxying through the backend if it's a doubanio.com URL.
 * Non-doubanio.com URLs are returned unchanged.
 * Returns empty string on error ( VodCard will show placeholder).
 */
export async function getDoubanImageUrl(poster: string | null | undefined): Promise<string> {
  if (!poster) return ''
  if (!poster.includes('doubanio.com')) return poster
  try {
    const base64 = await invoke<string>('proxy_image', { url: poster })
    return `data:image/jpeg;base64,${base64}`
  } catch (e) {
    console.warn('[getDoubanImageUrl] failed for:', poster, e)
    return ''
  }
}