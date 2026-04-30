# Playback Ad Blocking Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce playback-time ads in HLS streams and playback overlays while keeping playback stability and player controls intact.

**Architecture:** Keep HLS sanitation in Rust, keep playback-page cleanup in a small testable TypeScript helper, and make `PlayerPage.vue` a thin integrator. The Rust layer fixes playlist handling first, because it is the lowest-risk place to remove ad segments. The frontend layer then classifies playback requests and removes only obvious overlay nodes, with fail-open behavior if the page structure is unfamiliar.

**Tech Stack:** Vue 3, hls.js, Tauri 2.x, Rust, DOM APIs, Vitest

---

## File Map

- `src-tauri/src/services/ad_blocker.rs`
  - Playlist ad filtering rules and Rust unit tests

- `src-tauri/src/services/resolver.rs`
  - HLS manifest fetch flow, master playlist handling, and regression tests

- `src/utils/playbackAdBlocking.ts`
  - Pure TypeScript helpers for request classification and DOM cleanup

- `src/utils/__tests__/playbackAdBlocking.spec.ts`
  - Vitest coverage for helper behavior

- `src/views/PlayerPage.vue`
  - Integrate the helper into the hls.js loader and the page cleanup lifecycle

## Task 1: Fix master-playlist ad filtering in Rust

**Files:**
- Modify: `src-tauri/src/services/resolver.rs`
- Modify: `src-tauri/src/services/ad_blocker.rs`

- [ ] **Step 1: Write the failing regression test for master playlists**

Add a resolver test that proves ad segments inside a fetched variant playlist are filtered before the variant is embedded back into the master playlist.

```rust
#[tokio::test]
async fn fetch_hls_manifest_internal_filters_ads_inside_master_playlist() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let master_body = "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1280000\n/variant.m3u8\n";
    let variant_body = "#EXTM3U\n#EXTINF:10.0,\nhttps://ads.example.com/ad1.ts\n#EXTINF:10.0,\nhttps://cdn.example.com/seg1.ts\n";

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0_u8; 4096];
            let n = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            let body = if request.contains("/variant.m3u8") {
                variant_body
            } else {
                master_body
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/vnd.apple.mpegurl\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });

    let url = format!("http://{}/master.m3u8", addr);
    let result = fetch_hls_manifest_internal(&url, None, None).await.unwrap();

    assert!(!result.contains("ad1.ts"));
    assert!(result.contains("seg1.ts"));
}
```

- [ ] **Step 2: Run the targeted resolver test and confirm it fails before the fix**

Run: `cd src-tauri && cargo test resolver::tests::fetch_hls_manifest_internal_filters_ads_inside_master_playlist -- --nocapture`

Expected: FAIL, because the current master-playlist path returns before the embedded variant is filtered.

- [ ] **Step 3: Change the master-playlist flow to filter the variant body before embedding**

Update the `if body.contains("#EXT-X-STREAM-INF")` branch so it filters the fetched variant playlist first, then passes the cleaned variant into `normalize_master_playlist()`.

```rust
if body.contains("#EXT-X-STREAM-INF") {
    let Some(variant_url) = first_playlist_resource(url, &body) else {
        return Err("master playlist missing variant url".to_string());
    };

    let variant_body = fetch_hls_playlist_with_headers_no_cors_and_retry(
        &client,
        &variant_url,
        request_headers.as_ref(),
        referer,
    )
    .await?;

    let rewritten_variant = rewrite_relative_urls(&variant_body, &variant_url);
    let cleaned_variant = HlsAdBlocker::filter_playlist(&rewritten_variant);
    let normalized = normalize_master_playlist(&body, url, &cleaned_variant, &variant_url);
    let rewritten = rewrite_relative_urls(&normalized, url);
    return Ok(rewritten);
}
```

- [ ] **Step 4: Keep the media-playlist fallback path unchanged**

Preserve the current non-master behavior so plain `m3u8` playlists still get rewritten and filtered exactly once.

```rust
let rewritten = rewrite_relative_urls(&body, url);
Ok(HlsAdBlocker::filter_playlist(&rewritten))
```

- [ ] **Step 5: Re-run the focused Rust test and confirm the regression is fixed**

Run: `cd src-tauri && cargo test resolver::tests::fetch_hls_manifest_internal_filters_ads_inside_master_playlist -- --nocapture`

Expected: PASS, and the returned playlist should no longer contain the ad segment URL.

- [ ] **Step 6: Commit the Rust fix**

```bash
git add src-tauri/src/services/resolver.rs src-tauri/src/services/ad_blocker.rs
git commit -m "feat: filter ads from embedded HLS variants"
```

## Task 2: Extract playback ad-blocking helpers into a pure utility module

**Files:**
- Create: `src/utils/playbackAdBlocking.ts`
- Create: `src/utils/__tests__/playbackAdBlocking.spec.ts`

- [ ] **Step 1: Write the failing Vitest cases for request classification and DOM cleanup**

Use jsdom-friendly tests so the logic can be verified without mounting the full player page.

```ts
import { describe, expect, it } from 'vitest'
import {
  applyPlaybackAdCleanup,
  classifyPlaybackRequest,
  isPlaybackAdResource,
} from '@/utils/playbackAdBlocking'

describe('playback ad blocking helpers', () => {
  it('classifies HLS manifests and direct segments', () => {
    expect(classifyPlaybackRequest('https://cdn.example.com/live/index.m3u8')).toBe('manifest')
    expect(classifyPlaybackRequest('https://cdn.example.com/seg-1.ts')).toBe('segment')
    expect(classifyPlaybackRequest('https://cdn.example.com/video.mp4')).toBe('segment')
    expect(classifyPlaybackRequest('https://cdn.example.com/player.js')).toBeNull()
  })

  it('flags obvious ad resources but not player assets', () => {
    expect(isPlaybackAdResource('https://ads.example.com/banner.js')).toBe(true)
    expect(isPlaybackAdResource('https://cdn.example.com/hls.js')).toBe(false)
  })

  it('removes only obvious ad overlays and keeps player controls', () => {
    document.body.innerHTML = `
      <div id="player-stage">
        <div class="player-controls"></div>
        <div class="banner-ad"></div>
        <iframe src="https://ads.example.com/ad.html"></iframe>
      </div>
    `

    const removed = applyPlaybackAdCleanup(document)

    expect(removed).toBe(2)
    expect(document.querySelector('.banner-ad')).toBeNull()
    expect(document.querySelector('iframe')).toBeNull()
    expect(document.querySelector('.player-controls')).not.toBeNull()
  })
})
```

- [ ] **Step 2: Run the new helper test and confirm it fails before the module exists**

Run: `npm run test -- src/utils/__tests__/playbackAdBlocking.spec.ts`

Expected: FAIL with an import or missing symbol error until the helper module is created.

- [ ] **Step 3: Implement the helper module with narrow, fail-open rules**

Create a small pure module that exports request classification and DOM cleanup functions.

```ts
export type PlaybackRequestKind = 'manifest' | 'segment' | null

const AD_RESOURCE_MARKERS: readonly string[] = [
  'doubleclick',
  'googlesyndication',
  'adservice',
  '/ad/',
  'banner-ad',
  'overlay-ad',
  'player-ad',
]

const AD_SELECTORS: readonly string[] = [
  'iframe[src*="doubleclick"]',
  'iframe[src*="googlesyndication"]',
  'iframe[src*="/ad/"]',
  '[class*="banner-ad"]',
  '[class*="overlay-ad"]',
  '[class*="player-ad"]',
  '[id*="banner-ad"]',
  '[id*="overlay-ad"]',
]

export function classifyPlaybackRequest(url: string): PlaybackRequestKind {
  const cleanUrl = url.split('?')[0].split('#')[0].toLowerCase()
  if (cleanUrl.includes('.m3u8')) return 'manifest'
  if (
    cleanUrl.endsWith('.ts') ||
    cleanUrl.endsWith('.mp4') ||
    cleanUrl.endsWith('.m4v') ||
    cleanUrl.endsWith('.webm') ||
    cleanUrl.endsWith('.mov')
  ) {
    return 'segment'
  }
  return null
}

export function isPlaybackAdResource(url: string): boolean {
  const cleanUrl = url.toLowerCase()
  return AD_RESOURCE_MARKERS.some(marker => cleanUrl.includes(marker))
}

export function applyPlaybackAdCleanup(root: ParentNode): number {
  let removed = 0

  for (const selector of AD_SELECTORS) {
    const matches = root.querySelectorAll(selector)
    matches.forEach(node => {
      const element = node as HTMLElement
      if (element.closest('.player-controls, .playback-header, .playback-drawer')) {
        return
      }
      element.remove()
      removed += 1
    })
  }

  return removed
}
```

- [ ] **Step 4: Re-run the helper tests and confirm the module behaves as expected**

Run: `npm run test -- src/utils/__tests__/playbackAdBlocking.spec.ts`

Expected: PASS, with the cleanup test removing only the ad nodes and leaving player controls intact.

- [ ] **Step 5: Commit the helper module**

```bash
git add src/utils/playbackAdBlocking.ts src/utils/__tests__/playbackAdBlocking.spec.ts
git commit -m "feat: add playback ad blocking helpers"
```

## Task 3: Wire `PlayerPage.vue` to the helper module and keep playback fail-open

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Replace inline request classification with helper imports**

Import the helper and use it in the hls.js custom loader instead of hardcoding the extension checks in the component.

```ts
import {
  applyPlaybackAdCleanup,
  classifyPlaybackRequest,
  isPlaybackAdResource,
} from '@/utils/playbackAdBlocking'
```

```ts
const stats = {
  aborted: false,
  loaded: 0,
  retry: 0,
  total: 0,
  chunkCount: 0,
  bwEstimate: 0,
  loading: { start: 0, first: 0, end: 0 },
  parsing: { start: 0, end: 0 },
  buffering: { start: 0, end: 0 },
}

const requestKind = classifyPlaybackRequest(url)
if (requestKind === 'manifest' || requestKind === 'segment') {
  invoke<string>('fetch_hls_manifest', { url, headers, referer })
    .then((data) => {
      const finalData: string | ArrayBuffer = requestKind === 'segment'
        ? Uint8Array.from(atob(data), c => c.charCodeAt(0)).buffer
        : data
      callbacks.onSuccess({ data: finalData, url, code: 200 }, stats, context, null)
    })
    .catch((err) => {
      if (shouldFallbackToBrowserHls(err)) {
        ;(super.load as any)(context, config, callbacks)
        return
      }
      callbacks.onError({ code: 0, text: String(err) }, context, null, stats)
    })
  return
}

if (isPlaybackAdResource(url)) {
  callbacks.onError({ code: 0, text: 'blocked playback ad resource' }, context, null, stats)
  return
}
```

- [ ] **Step 2: Add a DOM cleanup lifecycle hook that fails open**

Use a `MutationObserver` to remove obvious overlay nodes after they appear, and disconnect it on unmount.

```ts
let adCleanupObserver: MutationObserver | null = null

function startAdCleanupObserver() {
  if (adCleanupObserver) {
    adCleanupObserver.disconnect()
  }

  adCleanupObserver = new MutationObserver(() => {
    applyPlaybackAdCleanup(document)
  })

  adCleanupObserver.observe(document.body, {
    childList: true,
    subtree: true,
  })

  applyPlaybackAdCleanup(document)
}
```

```ts
onMounted(() => {
  startAdCleanupObserver()
})

onUnmounted(() => {
  if (adCleanupObserver) {
    adCleanupObserver.disconnect()
    adCleanupObserver = null
  }
})
```

- [ ] **Step 3: Preserve the existing playback fallback paths**

Keep `shouldFallbackToBrowserHls`, native HLS fallback, source failover, and fullscreen behavior untouched. The ad-blocking layer must not change the existing error handling order.

```ts
if (shouldFallbackToBrowserHls(err)) {
  ;(super.load as any)(context, config, callbacks)
  return
}
```

- [ ] **Step 4: Run the existing player tests plus the new helper test**

Run: `npm run test -- src/utils/__tests__/playbackAdBlocking.spec.ts src/components/player/__tests__/PlaybackDrawer.spec.ts src/utils/__tests__/player.spec.ts`

Expected: PASS, and the player page should still satisfy the current playback-drawer and player-utility regressions.

- [ ] **Step 5: Commit the PlayerPage wiring**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat: wire playback ad blocking into player page"
```

## Task 4: Verify playback stability end-to-end

**Files:**
- No new code files expected

- [ ] **Step 1: Run the Rust test suite for the resolver and ad blocker paths**

Run: `cd src-tauri && cargo test ad_blocker::tests resolver::tests -- --nocapture`

Expected: PASS, including the master-playlist regression added in Task 1.

- [ ] **Step 2: Run the frontend unit test suite**

Run: `npm run test`

Expected: PASS.

- [ ] **Step 3: Build the app**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 4: Manually validate one HLS source and one overlay-heavy playback page**

Confirm all of the following in the running app:

```text
1. HLS playback still starts.
2. A known ad segment no longer appears in the stream.
3. Play/pause, seek, volume, fullscreen, and line switching still work.
4. Obvious overlays and banner containers disappear or stay hidden.
5. If the ad cleanup misses a node, playback still continues.
```

- [ ] **Step 5: Commit the verification-ready state**

```bash
git add src-tauri/src/services/resolver.rs src-tauri/src/services/ad_blocker.rs src/utils/playbackAdBlocking.ts src/utils/__tests__/playbackAdBlocking.spec.ts src/views/PlayerPage.vue
git commit -m "feat: complete playback ad blocking pipeline"
```

## Self-Review

- Spec coverage:
  - HLS stream ads are covered by Task 1.
  - Playback-page overlays and iframes are covered by Task 2 and Task 3.
  - Playback stability and rollback behavior are covered by Tasks 1 through 4.

- Placeholder scan:
  - No `TBD`, `TODO`, or other placeholders.
  - Every code step includes concrete code.

- Type consistency:
  - `classifyPlaybackRequest`, `isPlaybackAdResource`, and `applyPlaybackAdCleanup` are used consistently across the helper tests and `PlayerPage.vue`.
  - The Rust resolver change uses `fetch_hls_manifest_internal`, `normalize_master_playlist`, and `rewrite_relative_urls` consistently with the current service code.

- Scope check:
  - This plan stays focused on playback ad blocking only.
  - It does not introduce a whole-app network interceptor or unrelated refactors.
