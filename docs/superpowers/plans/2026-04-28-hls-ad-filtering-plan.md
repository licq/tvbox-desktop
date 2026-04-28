# HLS Ad Segment Filtering Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Filter out pre-roll and mid-roll ads from HLS (m3u8) streams by parsing the playlist in the Rust backend and removing segments whose URLs match an ad domain blacklist.

**Architecture:** New `ad_blocker.rs` module parses m3u8 playlists and removes ad segment lines. Existing `fetch_hls_manifest_internal()` in `resolver.rs` calls the filter before returning. The frontend hls.js custom loader intercepts all `.m3u8` requests (previously only specific CDNs) to route them through the filtering pipeline.

**Tech Stack:** Rust (no external crate needed — regex, string parsing), TypeScript/Vue 3

---

## File Structure

| File | Responsibility |
|------|---------------|
| `src-tauri/src/services/ad_blocker.rs` | **New.** HLS playlist parser, ad domain blacklist, segment filtering |
| `src-tauri/src/services/mod.rs` | Add `pub mod ad_blocker;` registration |
| `src-tauri/src/services/resolver.rs` | Call `HlsAdBlocker::filter_playlist()` in `fetch_hls_manifest_internal` |
| `src/views/PlayerPage.vue` | Custom loader intercepts all `.m3u8` requests, preserves CDN `.ts` proxy |

---

### Task 1: Create ad_blocker.rs with HLS playlist filter and tests

**Files:**
- Create: `src-tauri/src/services/ad_blocker.rs`
- Test: inline `#[cfg(test)] mod tests` at the bottom of the file

- [ ] **Step 1: Write failing tests for ad_blocker**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_ad_segments_by_domain() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://content-cdn.example.com/seg1.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // ad-cdn.example.com contains "/ad/" in path - should be filtered
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        // Content segment should be preserved
        assert!(result.contains("content-cdn.example.com/seg1.ts"));
    }

    #[test]
    fn passes_through_clean_playlist_unchanged() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn removes_discontinuity_before_ad_segment() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:15.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXT-X-DISCONTINUITY\n#EXTINF:10.0,\nhttps://cdn.example.com/seg2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(result.contains("cdn.example.com/seg1.ts"));
        assert!(!result.contains("ad-cdn.example.com/ad1.ts"));
        assert!(!result.contains("#EXT-X-DISCONTINUITY")); // first DISCONTINUITY removed with ad
        assert!(result.contains("cdn.example.com/seg2.ts"));
        // Second DISCONTINUITY before the real content should also be removed
        // (it was only relevant for the ad transition)
    }

    #[test]
    fn handles_master_playlist_with_variant_urls() {
        // Master playlists don't have EXTINF/segment URL pairs — they have #EXT-X-STREAM-INF
        // and variant URLs. The filter should pass these through unchanged.
        let playlist = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\nhttps://cdn.example.com/variant.m3u8\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert_eq!(result, playlist);
    }

    #[test]
    fn filters_ad_segments_by_url_pattern() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://cdn.example.com/ad-1001.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        assert!(!result.contains("ad-1001.ts"));
        assert!(result.contains("seg1.ts"));
    }

    #[test]
    fn handles_empty_playlist() {
        let result = HlsAdBlocker::filter_playlist("");
        assert_eq!(result, "");
    }

    #[test]
    fn handles_playlist_with_only_ads() {
        let playlist = "#EXTM3U\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://ad-cdn.example.com/ad2.ts\n";
        let result = HlsAdBlocker::filter_playlist(playlist);
        // Should keep header lines but remove all segments
        assert!(result.contains("#EXTM3U"));
        assert!(!result.contains("#EXTINF"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test ad_blocker::tests -- --nocapture 2>&1`
Expected: error[E0432] - unresolved import `crate::services::ad_blocker` (module not yet created)

- [ ] **Step 3: Write the ad_blocker module**

```rust
/// HLS ad segment filtering.
///
/// Parses HLS playlists (m3u8) and removes segments whose URLs
/// match known ad CDN domains or URL patterns.
pub struct HlsAdBlocker;

/// Known ad CDN domain fragments and URL path patterns.
const AD_PATTERNS: &[&str] = &[
    // URL path patterns
    "/ad/",
    "ad-",
    ".ad.",
    "adservice",
    "-ads-",
    "adtrack",
    "doubleclick",
    "googlesyndication",
];

impl HlsAdBlocker {
    /// Check if a URL belongs to a known ad CDN or matches an ad pattern.
    fn is_ad_url(url: &str) -> bool {
        let url_lower = url.to_lowercase();
        AD_PATTERNS.iter().any(|&pattern| url_lower.contains(pattern))
    }

    /// Remove ad segments from an HLS playlist.
    ///
    /// Scans the playlist for `#EXTINF:` + URL pairs. When a segment URL matches
    /// the ad blacklist, both the `#EXTINF:` line and the URL line are removed.
    /// Any `#EXT-X-DISCONTINUITY` line immediately preceding a removed ad segment
    /// is also removed.
    ///
    /// Master playlists (containing `#EXT-X-STREAM-INF`) are passed through
    /// unchanged — they contain variant URLs, not segment URLs.
    pub fn filter_playlist(playlist: &str) -> String {
        if playlist.is_empty() {
            return String::new();
        }

        // Master playlists don't have EXTINF/segment pairs — pass through
        if playlist.contains("#EXT-X-STREAM-INF") {
            return playlist.to_string();
        }

        let lines: Vec<&str> = playlist.lines().collect();
        let mut result: Vec<&str> = Vec::with_capacity(lines.len());
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            if line.starts_with("#EXTINF:") {
                // Look ahead for the segment URL (next non-comment, non-empty line)
                let mut url_line_index = i + 1;
                while url_line_index < lines.len() {
                    let next = lines[url_line_index].trim();
                    if next.is_empty() || next.starts_with('#') {
                        url_line_index += 1;
                    } else {
                        break;
                    }
                }

                if url_line_index < lines.len() {
                    let url = lines[url_line_index].trim();
                    if Self::is_ad_url(url) {
                        // Remove this ad segment. Also remove any DISCONTINUITY
                        // line that was right before the EXTINF.
                        if !result.is_empty() && result.last().map_or(false, |l| l.contains("#EXT-X-DISCONTINUITY")) {
                            result.pop();
                        }
                        // Skip EXTINF line, URL line, and any DISCONTINUITY after the segment
                        i = url_line_index + 1;
                        // Also skip DISCONTINUITY that might follow the ad segment
                        while i < lines.len() && lines[i].contains("#EXT-X-DISCONTINUITY") {
                            i += 1;
                        }
                        continue;
                    }
                }
            }

            result.push(line);
            i += 1;
        }

        result.join("\n")
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd src-tauri && cargo test ad_blocker::tests -- --nocapture`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/ad_blocker.rs
git commit -m "feat: add HLS ad segment filtering module with domain blacklist"
```

---

### Task 2: Register ad_blocker module in mod.rs

**Files:**
- Modify: `src-tauri/src/services/mod.rs`

- [ ] **Step 1: Add module declaration**

Add after the existing module declarations (line 9, before `pub mod storage;`):

```rust
pub mod ad_blocker;
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Compilation succeeds (may warn about unused module — that's fine, resolver.rs will use it in Task 3)

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/mod.rs
git commit -m "chore: register ad_blocker module"
```

---

### Task 3: Integrate ad filtering into resolver.rs

**Files:**
- Modify: `src-tauri/src/services/resolver.rs`

- [ ] **Step 1: Verify the integration point**

Read the `fetch_hls_manifest_internal` function in `resolver.rs` (lines 632-664). Note the flow:
1. For non-.m3u8 URLs → call `proxy_url()` (return base64)
2. For master playlists → fetch variant, normalize, return
3. For media playlists → rewrite relative URLs, return

After `rewrite_relative_urls()` returns the cleaned text, we'll call `HlsAdBlocker::filter_playlist()`.

- [ ] **Step 2: Add the import and filtering calls**

```rust
// Add at top of file with existing imports
use crate::services::ad_blocker::HlsAdBlocker;
```

In the master playlist path (around line 659), change:
```rust
        // Rewrite relative URLs in the normalized master to absolute
        return Ok(rewrite_relative_urls(&normalized, url));
```
to:
```rust
        // Rewrite relative URLs in the normalized master to absolute
        let rewritten = rewrite_relative_urls(&normalized, url);
        // Filter ad segments from the variant playlist (embedded in data URI)
        // The ad_blocker handles the playlist content — master playlists (#EXT-X-STREAM-INF)
        // pass through unchanged; the variant segments are already embedded.
        return Ok(HlsAdBlocker::filter_playlist(&rewritten));
```

In the media playlist path (line 663), change:
```rust
    // It's not a master playlist, just rewrite relative URLs to absolute
    Ok(rewrite_relative_urls(&body, url))
```
to:
```rust
    // It's not a master playlist, rewrite relative URLs to absolute then filter ads
    let rewritten = rewrite_relative_urls(&body, url);
    Ok(HlsAdBlocker::filter_playlist(&rewritten))
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Compilation succeeds

- [ ] **Step 4: Run existing tests**

Run: `cd src-tauri && cargo test resolver::tests -- --nocapture`
Expected: all existing tests PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/resolver.rs
git commit -m "feat: integrate HLS ad segment filtering into fetch_hls_manifest"
```

---

### Task 4: Update frontend custom loader to intercept all .m3u8 requests

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Read the current custom loader**

The custom loader is defined around lines 458-485 in `PlayerPage.vue`. Current logic:
```typescript
const CustomLoader = class extends Hls.DefaultConfig.loader {
  load(context: any, config: any, callbacks: any) {
    const url = context.url
    if (url.includes('baofeng10') || url.includes('bfllvip') || url.includes('baofeng') || url.includes('fengbao9') || url.includes('lzcdn')) {
      // Proxy through Rust
    }
    // Default behavior for other URLs
    ;(super.load as any)(context, config, callbacks)
  }
}
```

- [ ] **Step 2: Modify the custom loader**

Change the loader so that:
1. All `.m3u8` requests go through Rust proxy (for ad filtering + CORS bypass)
2. `.ts` segments from problematic CDNs still go through Rust proxy (for CORS bypass)
3. Everything else uses the default loader

```typescript
      // Custom loader for ad filtering and CORS bypass
      const CustomLoader = class extends Hls.DefaultConfig.loader {
        load(context: any, config: any, callbacks: any) {
          const url = context.url
          // All .m3u8 requests go through Rust proxy for ad filtering + CORS bypass
          if (url.includes('.m3u8')) {
            invoke<string>('fetch_hls_manifest', { url })
              .then((data) => {
                const stats = {
                  aborted: false, loaded: data.length, retry: 0,
                  total: data.length, chunkCount: 0, bwEstimate: 0,
                  loading: { start: 0, first: 0, end: 0 },
                  parsing: { start: 0, end: 0 },
                  buffering: { start: 0, end: 0 },
                }
                callbacks.onSuccess({ data, url, code: 200 }, stats, context, null)
              })
              .catch((err) => {
                callbacks.onError(
                  { code: 0, text: String(err) }, context, null,
                  { aborted: false, loaded: 0, retry: 0, total: 0, chunkCount: 0, bwEstimate: 0, loading: { start: 0, first: 0, end: 0 }, parsing: { start: 0, end: 0 }, buffering: { start: 0, end: 0 } }
                )
              })
            return
          }
          // .ts segments from known problematic CDNs still need Rust proxy for CORS
          if (url.includes('baofeng10') || url.includes('bfllvip') || url.includes('baofeng') || url.includes('fengbao9') || url.includes('lzcdn')) {
            invoke<string>('fetch_hls_manifest', { url })
              .then((data) => {
                const isSegment = !url.includes('.m3u8')
                const finalData: string | ArrayBuffer = isSegment
                  ? Uint8Array.from(atob(data), c => c.charCodeAt(0)).buffer
                  : data
                const finalLength = typeof finalData === 'string' ? finalData.length : finalData.byteLength
                const stats = { aborted: false, loaded: finalLength, retry: 0, total: finalLength, chunkCount: 0, bwEstimate: 0, loading: { start: 0, first: 0, end: 0 }, parsing: { start: 0, end: 0 }, buffering: { start: 0, end: 0 } }
                callbacks.onSuccess({ data: finalData, url, code: 200 }, stats, context, null)
              })
              .catch((err) => {
                callbacks.onError({ code: 0, text: String(err) }, context, null, { aborted: false, loaded: 0, retry: 0, total: 0, chunkCount: 0, bwEstimate: 0, loading: { start: 0, first: 0, end: 0 }, parsing: { start: 0, end: 0 }, buffering: { start: 0, end: 0 } })
              })
            return
          }
          // Default behavior for other URLs
          ;(super.load as any)(context, config, callbacks)
        }
      }
```

- [ ] **Step 3: Build frontend to verify syntax**

Run: `npm run build 2>&1`
Expected: TypeScript compilation succeeds (no errors)

- [ ] **Step 4: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: route all m3u8 requests through Rust proxy for ad filtering"
```

---

### Task 5: Verify full project builds

**Files:** None — verification only

- [ ] **Step 1: Verify Rust backend builds with all tests**

Run: `cd src-tauri && cargo test 2>&1`
Expected: All tests pass (including existing resolver tests and new ad_blocker tests)

- [ ] **Step 2: Verify frontend builds**

Run: `npm run build 2>&1`
Expected: TypeScript build succeeds with no errors

- [ ] **Step 3: Verify Tauri build**

Run: `npm run tauri build 2>&1`
Expected: Full Tauri build succeeds
