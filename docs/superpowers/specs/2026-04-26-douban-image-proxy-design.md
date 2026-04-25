# Douban Image Proxy Design

## Problem

Douban CDN (`img*.doubanio.com`) returns HTTP 418 ACL denial for requests without proper `Referer` header. When users browse Douban hot content in the TVBox app, images fail to load because Tauri WebView doesn't send the required Referer.

However, images load fine in a regular browser because the Referer is automatically set when navigating from `movie.douban.com`.

## Solution

Implement a backend image proxy in Rust that forwards image requests with the correct Referer header. This proxy will be reusable for both hot list images and future search results.

## Architecture

### Backend (Rust)

**New command in `src-tauri/src/commands/douban.rs`:**

```rust
#[tauri::command]
pub async fn proxy_image(url: String) -> Result<String, String> {
    // Validate URL is doubanio.com
    // Fetch with Referer: https://movie.douban.com/
    // Return base64 encoded image
}
```

**Image caching:**
- Use a simple `std::collections::HashMap` with URL as key and base64 as value
- Cache is in-memory only (lifetime = app session)
- Prevents duplicate proxy requests for the same image URL

### Frontend (Vue)

**New utility function in `src/utils/douban.ts`:**

```typescript
export async function getDoubanImageUrl(poster: string | null): Promise<string> {
  if (!poster) return ''
  if (!poster.includes('doubanio.com')) return poster
  try {
    const base64 = await invoke<string>('proxy_image', { url: poster })
    return `data:image/jpeg;base64,${base64}`
  } catch {
    return ''  // fallback to empty on error
  }
}
```

**Usage in components:**
- `VodCard.vue`: Use `getDoubanImageUrl(item.poster)` instead of `item.poster` directly
- Any other component displaying Douban poster images

## Data Flow

```
User sees:    <img :src="getDoubanImageUrl(hot.poster)">
                    ↓ (if doubanio.com URL)
Frontend:     invoke('proxy_image', { url })
                    ↓ IPC
Backend:      reqwest GET url
                    + Header: Referer: https://movie.douban.com/
                    ↓
Douban CDN:   HTTP 200 + image bytes
                    ↓
Backend:      base64 encode → return string
                    ↓ IPC
Frontend:     data:image/jpeg;base64,... → <img src=...>
```

## Error Handling

- If URL is not doubanio.com: return original URL unchanged
- If proxy request fails: return empty string (VodCard shows placeholder emoji)
- If URL is null/undefined: return empty string

## Security Considerations

- Only proxy URLs from `doubanio.com` domain (validation in backend)
- No caching to disk (in-memory only, lifetime = app session)
- Base64 encoding increases size ~33%, but Douban images are small (~20-50KB)

## Testing

- Integration test: verify `proxy_image` returns valid base64 for known doubanio.com URL
- Manual test: open movie tab, verify images load correctly

## Files to Modify

1. `src-tauri/src/commands/douban.rs` - Add `proxy_image` command
2. `src/utils/douban.ts` - Add `getDoubanImageUrl` utility function
3. `src/components/VodCard.vue` - Use `getDoubanImageUrl` for poster
4. Any other component showing Douban posters (e.g., HotDetail)
