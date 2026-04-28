# HLS Ad Segment Filtering Design

## Overview

Filter out pre-roll and mid-roll video ads from HLS (m3u8) streams by proxying the playlist through the Rust backend, parsing it to identify ad segments via domain blacklist matching, and removing them before passing the clean playlist to hls.js.

## Background

Chinese streaming sources (YPanSo, xb6v, etc.) embed ad segments directly into HLS playlists. The existing codebase already has a `fetch_hls_manifest` Tauri command that proxies HLS playlists through the Rust backend (for CORS bypass), but it does not perform ad filtering. The frontend hls.js custom loader only intercepts requests to specific CDN hostnames.

## Architecture

```
hls.js ──→ Custom Loader ──→ invoke('fetch_hls_manifest') ──→ Rust Backend
                │                                                    │
                │                                              fetch m3u8 URL
                │                                                    │
                │                                              parse playlist
                │                                                    │
                │                                         match ad domains
                │                                                    │
                │                                              remove ad segs
                │                                                    │
                ◄──────── clean playlist ────────────────────────────┘
                │
          load .ts segments directly
          (no proxy needed)
```

## Implementation

### 1. New module: `src-tauri/src/services/ad_blocker.rs`

Parses HLS playlists and filters out segments whose URLs match known ad CDN domains.

```rust
pub struct HlsAdBlocker;

impl HlsAdBlocker {
    /// Check if a URL belongs to a known ad CDN.
    fn is_ad_url(url: &str) -> bool;

    /// Remove ad segments from an HLS playlist.
    /// Input: raw m3u8 text → Output: cleaned m3u8 text.
    pub fn filter_playlist(playlist: &str) -> String;
}
```

**Filtering algorithm** (`filter_playlist`):

1. Split the playlist into lines
2. Iterate through lines; when encountering an `#EXTINF:` line, look at the **next non-empty, non-comment line** (the segment URL)
3. If that URL matches `is_ad_url()`, also skip any `#EXT-X-DISCONTINUITY` lines preceding the ad segment
4. Otherwise, keep both the `#EXTINF:` line and the URL line
5. Rejoin remaining lines

**Domain blacklist** (initial set, extensible):

```rust
const AD_DOMAINS: &[&str] = &[
    // Ad URL path patterns (checked against full URL)
    // Examples that will be expanded based on observed ad traffic:
    "/ad/",
    "ad-",
    ".ad.",
    "adservice",
    "-ads-",
    "adtrack",
    "doubleclick",
    "googlesyndication",
    // Specific ad CDNs (to be populated during testing)
];
```

Note: The blacklist requires real-world testing with YPanSo and other sources to identify actual ad domains. The initial implementation should be designed for easy extension (simple list append).

### 2. Modify `src-tauri/src/services/resolver.rs`

Add ad filtering to `fetch_hls_manifest_internal()`:

```
Current flow:
  fetch m3u8 body → rewrite relative URLs → return

New flow:
  fetch m3u8 body → rewrite relative URLs → [filter ad segments] → return
```

For master playlists (multi-bitrate): the existing `normalize_master_playlist()` already fetches the variant playlist. Apply ad filtering to the variant playlist before embedding it into the master.

### 3. Modify `src/views/PlayerPage.vue`

Change the hls.js custom loader to intercept **all `.m3u8` requests** for ad filtering, while **keeping the existing per-CDN `.ts` proxy** for CORS bypass:

```typescript
const CustomLoader = class extends Hls.DefaultConfig.loader {
  load(context: any, config: any, callbacks: any) {
    const url = context.url
    // All .m3u8 requests go through Rust proxy for ad filtering + CORS bypass
    if (url.includes('.m3u8')) {
      invoke<string>('fetch_hls_manifest', { url })
        .then((data) => {
          // data is the cleaned playlist (ads removed by Rust backend)
          const stats = {
            aborted: false, loaded: data.length, retry: 0,
            total: data.length, chunkCount: 0, bwEstimate: 0,
            loading: { start: 0, first: 0, end: 0 },
            parsing: { start: 0, end: 0 },
            buffering: { start: 0, end: 0 },
          };
          callbacks.onSuccess({ data, url, code: 200 }, stats, context, null);
        })
        .catch((err) => {
          callbacks.onError(
            { code: 0, text: String(err) }, context, null,
            { aborted: false, loaded: 0, retry: 0, total: 0, ... }
          );
        })
      return;
    }
    // .ts segments from known problematic CDNs still need Rust proxy for CORS
    if (url.includes('baofeng10') || url.includes('bfllvip') || url.includes('baofeng')
        || url.includes('fengbao9') || url.includes('lzcdn')) {
      // Same base64 proxy logic as current implementation
      invoke<string>('fetch_hls_manifest', { url })
        .then((data) => { /* decode base64 to ArrayBuffer */ })
        .catch((err) => { /* error handling */ });
      return;
    }
    // Default behavior for other URLs
    (super.load as any)(context, config, callbacks);
  }
}
```

Note: The `.m3u8` check must come BEFORE the per-CDN check, since some `.m3u8` URLs from sources like YPanSo may not match the specific CDN hostnames but still need ad filtering.

### 4. Registration

Add `pub mod ad_blocker;` to `src-tauri/src/services/mod.rs` or `src-tauri/src/lib.rs`.

## Edge Cases

| Case | Handling |
|------|----------|
| All segments are ads | Return empty playlist; hls.js triggers error → auto-fallback to next candidate |
| Master playlist with ads | Filter the variant playlist (which contains the actual segments) |
| Non-HLS playback (MP4) | Unaffected — only the m3u8 code path is modified |
| Relative URLs in playlist | `rewrite_relative_urls()` is called before `filter_playlist()`, so URLs are absolute for domain matching |
| `#EXT-X-DISCONTINUITY` before ad segments | Remove discontinuity marker together with the ad segment |
| Ad domains change over time | Blacklist is a simple const array, easy to update |
| No ads in stream | `filter_playlist()` passes through unchanged (no matches) |
| `.ts` segments from CORS-restricted CDNs | Per-CDN check in custom loader preserved alongside new m3u8 interception |
| `.m3u8` URL also matches a CORS CDN | m3u8 check comes first, handles both ad filtering and CORS bypass |

## Files Changed

| File | Change |
|------|--------|
| `src-tauri/src/services/ad_blocker.rs` | **New** — ad domain blacklist + m3u8 filter |
| `src-tauri/src/services/resolver.rs` | Add ad filtering call in `fetch_hls_manifest_internal` |
| `src-tauri/src/services/mod.rs` (or `lib.rs`) | Register `ad_blocker` module |
| `src/views/PlayerPage.vue` | Change custom loader to intercept all `.m3u8` requests |

## Out of Scope

- UI-level ad detection feedback ("skip ad" button)
- User-configurable ad domain list (hardcoded initially)
- Ad blocking for non-HLS streams (MP4, iframe embeds)
- Server-side ad detection beyond domain blacklist
