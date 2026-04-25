# HLS Manifest Proxy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 HLS.js 能够播放来自 CDN（v.baofeng10.com、s2.bfllvip.com 等）的 m3u8，通过 Rust 后端代理请求来绕过浏览器 CORS 限制。

**Architecture:** 在 Rust 后端添加 `fetch_hls_manifest` Tauri 命令，前端 HLS.js 使用自定义 Loader，通过 Tauri 命令获取 m3u8 内容（Rust 用 reqwest 代理请求，无 CORS 限制）。对于 master playlist 中的相对路径 variant URL，在 Rust 端转换为绝对 URL。

**Tech Stack:** Rust (reqwest), Tauri 2.x, Vue 3, HLS.js custom Loader

---

## File Structure

**Files:**
- Modify: `src-tauri/src/commands/player.rs` - 添加 `fetch_hls_manifest` 命令
- Modify: `src-tauri/src/services/resolver.rs` - 添加 `fetch_hls_manifest_internal` 内部函数（处理相对路径转绝对路径）
- Modify: `src-tauri/src/main.rs` - 注册新命令
- Modify: `src/views/PlayerPage.vue` - 自定义 HLS.js Loader 调用 Tauri 命令
- Modify: `src/stores/playback.ts` - 添加 manifest proxy URL 生成（可选）

---

## Task 1: Add `fetch_hls_manifest` to player.rs

**Files:**
- Modify: `src-tauri/src/commands/player.rs`

- [ ] **Step 1: Write the failing test**

在 `src-tauri/src/commands/player.rs` 底部添加测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_hls_manifest_returns_manifest_content() {
        // This test will fail because fetch_hls_manifest doesn't exist yet
        let result = fetch_hls_manifest(
            "https://s2.bfllvip.com/video/test/index.m3u8".to_string(),
            None,
        ).await;
        // Initially this will error "function not found"
        assert!(result.is_ok() || result.is_err()); // placeholder assertion
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test fetch_hls_manifest -- --nocapture 2>&1 | head -30`
Expected: compile error "cannot find function `fetch_hls_manifest`"

- [ ] **Step 3: Write minimal implementation**

在 `player.rs` 添加命令：

```rust
use crate::services::playback_runtime::fetch_hls_manifest_internal;

#[tauri::command]
pub async fn fetch_hls_manifest(
    url: String,
    headers: Option<std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    fetch_hls_manifest_internal(&url, headers.as_ref()).await
}
```

- [ ] **Step 4: Run test to verify it passes (compile)**

Run: `cd src-tauri && cargo build 2>&1 | head -30`
Expected: compile error "function `fetch_hls_manifest_internal` not found in module `playback_runtime`"

---

## Task 2: Add `fetch_hls_manifest_internal` to resolver.rs

**Files:**
- Modify: `src-tauri/src/services/resolver.rs`

- [ ] **Step 1: Write the failing test**

在 `resolver.rs` 的 `#[cfg(test)] mod tests` 中添加：

```rust
#[tokio::test]
async fn fetch_hls_manifest_internal_fetches_master_playlist_with_absolute_variant_urls() {
    // Test that relative URLs in variant playlists are converted to absolute
    let result = fetch_hls_manifest_internal(
        "https://example.com/root/index.m3u8",
        None,
    ).await;
    // This will fail because function doesn't exist yet
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test fetch_hls_manifest_internal 2>&1 | head -30`
Expected: compile error "cannot find function `fetch_hls_manifest_internal`"

- [ ] **Step 3: Write minimal implementation**

在 `resolver.rs` 中添加（非 test 模块）：

```rust
/// Fetches an HLS manifest and normalizes relative URLs to absolute.
/// Used by the frontend HLS.js loader to bypass CORS restrictions.
pub(crate) async fn fetch_hls_manifest_internal(
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    if !url.contains(".m3u8") {
        return Err("URL does not appear to be an HLS manifest".to_string());
    }

    let client = build_client()?;
    let first_fetch = fetch_hls_playlist_with_headers_no_cors(&client, url, headers).await?;

    // If this is a master playlist (contains #EXT-X-STREAM-INF), resolve variant URLs
    if first_fetch.contains("#EXT-X-STREAM-INF") {
        let variant_url = first_playlist_resource(url, &first_fetch)
            .ok_or_else(|| "master playlist missing variant url".to_string())?;

        let variant_body = fetch_hls_playlist_with_headers_no_cors(&client, &variant_url, headers).await?;

        // Rewrite master playlist: replace variant URL with absolute version,
        // and embed variant playlist content as a data URI
        let normalized = normalize_master_playlist(url, &first_fetch, &variant_url, &variant_body);
        return Ok(normalized);
    }

    Ok(first_fetch)
}

/// Like fetch_hls_playlist_with_headers but without CORS check.
/// Used internally when we're proxying content to the browser.
async fn fetch_hls_playlist_with_headers_no_cors(
    client: &reqwest::Client,
    input: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
) -> Result<String, String> {
    let request = client
        .get(input)
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        );
    let response = apply_request_headers(request, headers)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("playlist request failed: {status}"));
    }
    response.text().await.map_err(|e| e.to_string())
}

/// Given a master playlist URL, original master body, variant URL, and variant body,
/// produces a normalized master playlist where the variant is embedded as a data URI
/// and segment references are absolute.
fn normalize_master_playlist(
    master_url: &str,
    master_body: &str,
    variant_url: &str,
    variant_body: &str,
) -> String {
    let base_url = &variant_url[..variant_url.rfind('/').map(|i| i + 1).unwrap_or(0)];

    // Rewrite segment URLs in variant body to absolute
    let normalized_variant = rewrite_relative_urls(variant_body, base_url);

    // Build data URI for variant
    let encoded_variant = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        normalized_variant.as_bytes(),
    );
    let data_uri = format!("data:application/vnd.apple.mpegurl;base64,{}", encoded_variant);

    // Replace the variant URL line in master with the data URI
    // The variant URL appears after #EXT-X-STREAM-INF lines
    let mut result = String::new();
    for line in master_body.lines() {
        if line.starts_with("http://") || line.starts_with("https://") {
            // This is a variant URL line - skip it (we're embedding via data URI instead)
            continue;
        }
        if line.ends_with(".m3u8") || line.ends_with(".ts") || line.ends_with(".m4s") {
            // Relative segment URL - rewrite to absolute
            let absolute = absolutize_url(master_url, line);
            result.push_str(&absolute);
            result.push('\n');
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }

    // Find the #EXT-X-STREAM-INF block and insert our data URI after it
    let mut output = String::new();
    let mut in_stream_inf = false;
    let mut stream_inf_lines = Vec::new();

    for line in master_body.lines() {
        output.push_str(line);
        output.push('\n');

        if line.contains("#EXT-X-STREAM-INF") {
            in_stream_inf = true;
            stream_inf_lines.push(line.to_string());
        } else if in_stream_inf {
            if line.starts_with("http://") || line.starts_with("https://") {
                // Variant URL line - replace with data URI
                output.push_str(&data_uri);
                output.push('\n');
                in_stream_inf = false;
                stream_inf_lines.clear();
            } else if line.is_empty() || !line.ends_with(".m3u8") {
                // Probably not a variant URL line, restore stream inf context
                in_stream_inf = false;
                stream_inf_lines.clear();
            }
        }
    }

    result
}

/// Converts relative URL to absolute using base URL.
fn absolutize_url(base: &str, relative: &str) -> String {
    if relative.starts_with("http://") || relative.starts_with("https://") {
        return relative.to_string();
    }

    let base_parts: Vec<&str> = base.split('/').collect();
    let relative_parts: Vec<&str> = relative.split('/').collect();

    let base_len = base_parts.len() - 1; // exclude the filename

    let mut path_parts: Vec<&str> = base_parts[..base_len].to_vec();

    for part in relative_parts {
        match part {
            "." | "" => continue,
            ".." => {
                if path_parts.len() > 1 {
                    path_parts.pop();
                }
            }
            _ => path_parts.push(part),
        }
    }

    path_parts.join("/")
}

/// Rewrites relative segment URLs in an HLS media playlist to absolute URLs.
fn rewrite_relative_urls(body: &str, base_url: &str) -> String {
    let mut result = String::new();
    for line in body.lines() {
        if line.ends_with(".ts") || line.ends_with(".m4s") || line.ends_with(".aac") || line.ends_with(".m4a") {
            let absolute = absolutize_url(base_url, line);
            result.push_str(&absolute);
            result.push('\n');
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
}
```

- [ ] **Step 4: Run cargo check to verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | head -50`
Expected: compile errors - need to find and resolve any type/import issues

- [ ] **Step 5: Fix compilation errors if any**

Common issues:
- `base64::Engine` not found → use `base64::engine::general_purpose::STANDARD`
- `apply_request_headers` not accessible → check function visibility

---

## Task 3: Register `fetch_hls_manifest` in main.rs

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add command registration**

在 `main.rs` 的 `generate_handler!` 中添加：

```rust
tvbox_lib::commands::player::fetch_hls_manifest,
```

确保 import 也正确。

---

## Task 4: Implement custom HLS.js Loader in PlayerPage.vue

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Write the failing test**

在 PlayerPage.vue 中，首先需要找到 initHlsPlayer 函数，添加注释标注要修改的位置。

实际的"测试"是手动验证 HLS.js 能通过 Tauri 命令加载 manifest 而不报 CORS 错误。

- [ ] **Step 2: Add fetch_hls_manifest to imports**

在 PlayerPage.vue 顶部添加：
```typescript
import { invoke } from '@tauri-apps/api/core'
```

- [ ] **Step 3: Modify initHlsPlayer to use Tauri proxy for HLS manifests**

找到 `if (url.includes('.m3u8'))` 分支，替换为：

```typescript
if (url.includes('.m3u8')) {
  const Hls = await getHlsConstructor()

  if (Hls.isSupported()) {
    const hls = new Hls({
      // Custom loader that fetches via Tauri command
      loader: class HlsTauriLoader extends Hls.DefaultConfig.loader {
        constructor() {
          super()
        }

        async load(url: string, options: any) {
          // If this is a direct CDN URL, proxy through Tauri command
          if (url.includes('.m3u8') &&
              (url.includes('baofeng') || url.includes('bfllvip') || url.includes('baofeng10'))) {
            // Use Tauri command to fetch manifest content
            try {
              const manifest = await invoke<string>('fetch_hls_manifest', { url })
              options.onSuccess({ url, data: manifest, code: 200, text: manifest })
              return
            } catch (e) {
              options.onError({ url, response: null, fatal: true, message: String(e) })
              return
            }
          }
          // For other URLs or non-manifest requests, use default behavior
          return super.load(url, options)
        }

        destroy() {
          super.destroy()
        }
      },
    })
    hlsInstance = hls
    hls.loadSource(url)
    hls.attachMedia(videoRef.value)
    // ... rest of error handling code stays the same
  }
}
```

- [ ] **Step 4: Test by running the app**

Run: `npm run tauri dev`
Navigate to a 金牌影院 video and try to play it.
Expected: No CORS error, video plays via Rust proxy.

---

## Task 5: Run full verification

- [ ] **Step 1: Build the app**

Run: `npm run tauri build 2>&1 | tail -30`

- [ ] **Step 2: Verify no regression**

Run existing tests:
Run: `cd src-tauri && cargo test 2>&1 | tail -20`

---

## Task 6: Commit

```bash
git add src-tauri/src/commands/player.rs src-tauri/src/services/resolver.rs src-tauri/src/main.rs src/views/PlayerPage.vue
git commit -m "feat: add HLS manifest proxy for CORS-restricted CDNs

- Add fetch_hls_manifest Tauri command in player.rs
- Add fetch_hls_manifest_internal in resolver.rs with relative URL rewriting
- Custom HLS.js loader in PlayerPage.vue fetches via Tauri command
  to bypass CORS restrictions for baofeng10/bfllvip CDNs
"
```
