# Douban Image Proxy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a backend proxy to fetch Douban CDN images with correct Referer header, enabling images to display in the Tauri WebView.

**Architecture:** A Rust Tauri command (`proxy_image`) receives an image URL, validates it's from `doubanio.com`, fetches it with `Referer: https://movie.douban.com/`, returns base64-encoded data. Frontend utility wraps this with a cache-friendly async interface.

**Tech Stack:** Rust (reqwest, base64 crates), TypeScript, Vue 3, Tauri 2.x

---

## File Map

- **Create:** `src/utils/douban.ts` — frontend utility function
- **Modify:** `src-tauri/src/commands/douban.rs` — add `proxy_image` command + in-memory cache
- **Modify:** `src-tauri/src/main.rs:29-34` — register `proxy_image` in `generate_handler!`
- **Modify:** `src/components/VodCard.vue` — use `getDoubanImageUrl` for poster
- **Modify:** `src/views/HotDetail.vue` — use `getDoubanImageUrl` for `doubanHot.poster`

---

## Task 1: Backend — Add `proxy_image` command

**Files:**
- Modify: `src-tauri/src/commands/douban.rs`
- Modify: `src-tauri/src/main.rs:29-34`

- [ ] **Step 1: Add imports and in-memory cache to `commands/douban.rs`**

Add these imports at the top of the file (after existing imports):

```rust
use base64::Engine;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;
```

Add this cache before the first command function:

```rust
// In-memory cache for proxy_image results
// Key: URL, Value: base64-encoded image data
static IMAGE_CACHE: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));
```

- [ ] **Step 2: Add `proxy_image` command function**

Add this function after the last existing command (`get_douban_hot_by_type`):

```rust
#[tauri::command]
pub async fn proxy_image(url: String) -> Result<String, String> {
    // Validate URL is doubanio.com
    if !url.contains("doubanio.com") {
        return Err("Only doubanio.com URLs are allowed".to_string());
    }

    // Check cache first
    {
        let cache = IMAGE_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&url) {
            return Ok(cached.clone());
        }
    }

    // Fetch image with correct Referer header
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let resp = client
        .get(&url)
        .header("Referer", "https://movie.douban.com/")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let bytes = resp.bytes().await
        .map_err(|e| format!("Failed to read image bytes: {}", e))?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    // Cache the result
    {
        let mut cache = IMAGE_CACHE.lock().unwrap();
        cache.insert(url.clone(), b64.clone());
    }

    Ok(b64)
}
```

- [ ] **Step 3: Add `once_cell` dependency**

Check `src-tauri/Cargo.toml` — if `once_cell` is not listed, add it:

```toml
once_cell = "1.19"
```

- [ ] **Step 4: Register `proxy_image` in `main.rs`**

Modify `src-tauri/src/main.rs` line 29-34, add the new command:

```rust
            tvbox_lib::commands::douban::get_douban_hot,
            tvbox_lib::commands::douban::fetch_douban_hot,
            tvbox_lib::commands::douban::get_matched_hot_list,
            tvbox_lib::commands::douban::fetch_all_douban_hot,
            tvbox_lib::commands::douban::search_vod_sources,
            tvbox_lib::commands::douban::get_douban_hot_by_type,
            tvbox_lib::commands::douban::proxy_image,  // <-- ADD THIS
```

- [ ] **Step 5: Run cargo check to verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: No errors (warnings OK)

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/douban.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat(douban): add proxy_image command with in-memory cache"
```

---

## Task 2: Frontend — Create `douban.ts` utility

**Files:**
- Create: `src/utils/douban.ts`

- [ ] **Step 1: Create `src/utils/douban.ts`**

```typescript
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
```

- [ ] **Step 2: Commit**

```bash
git add src/utils/douban.ts
git commit -m "feat(frontend): add getDoubanImageUrl utility"
```

---

## Task 3: Update VodCard.vue

**Files:**
- Modify: `src/components/VodCard.vue`

- [ ] **Step 1: Update script setup to use the utility**

Add import:
```typescript
import { getDoubanImageUrl } from '@/utils/douban'
```

Change the `itemType`, `itemTitle`, `itemEpisodeMeta` functions, then add:

```typescript
const imageUrl = ref('')
watch(() => props.item.poster, async (newPoster) => {
  imageUrl.value = await getDoubanImageUrl(newPoster)
}, { immediate: true })
```

- [ ] **Step 2: Update template to use reactive imageUrl**

Change the `<img>` tag from:
```html
<img
  v-if="item.poster"
  :src="item.poster"
  ...
/>
```

To:
```html
<img
  v-if="imageUrl"
  :src="imageUrl"
  ...
/>
```

The `v-else` placeholder (showing 🎬 emoji) will automatically display when `imageUrl` is empty.

- [ ] **Step 3: Commit**

```bash
git add src/components/VodCard.vue
git commit -m "feat(VodCard): use getDoubanImageUrl for poster"
```

---

## Task 4: Update HotDetail.vue

**Files:**
- Modify: `src/views/HotDetail.vue`

- [ ] **Step 1: Add import and reactive poster URL**

Add to script setup imports:
```typescript
import { getDoubanImageUrl } from '@/utils/douban'
```

Add reactive ref after the existing refs:
```typescript
const posterUrl = ref('')
```

Add a watch after `loadHotDetail` or modify `loadHotDetail` to set poster:
```typescript
async function loadHotDetail() {
  loading.value = true
  error.value = null
  try {
    const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType: itemType.value })
    const hot = items.find((h: DoubanHot) => h.id === doubanId.value)
    if (hot) {
      doubanHot.value = hot
      posterUrl.value = await getDoubanImageUrl(hot.poster)
      await searchSources(hot.name)
    } else {
      error.value = '热播数据不存在'
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}
```

- [ ] **Step 2: Update template to use `posterUrl`**

Change line 80-84 from:
```html
<img
  v-if="doubanHot.poster"
  :src="doubanHot.poster"
  ...
/>
```

To:
```html
<img
  v-if="posterUrl"
  :src="posterUrl"
  ...
/>
```

- [ ] **Step 3: Commit**

```bash
git add src/views/HotDetail.vue
git commit -m "feat(HotDetail): use getDoubanImageUrl for poster"
```

---

## Task 5: Verify — Run cargo check and build

- [ ] **Step 1: Run cargo check in src-tauri**

Run: `cd src-tauri && cargo check 2>&1`
Expected: No errors

- [ ] **Step 2: Run TypeScript check**

Run: `cd /Users/dustin/Workspace/tvbox && npx tsc --noEmit 2>&1`
Expected: No errors (or only pre-existing errors)

- [ ] **Step 3: Build the app**

Run: `npm run tauri build 2>&1 | tail -20`
Expected: Build succeeds

---

## Self-Review Checklist

- [ ] All 4 files modified match the spec
- [ ] `proxy_image` validates URL contains `doubanio.com` before fetching
- [ ] Cache uses `once_cell::Lazy` + `Mutex<HashMap>` as specified
- [ ] `getDoubanImageUrl` returns `data:image/jpeg;base64,...` format for proxied images
- [ ] `getDoubanImageUrl` returns original URL unchanged for non-doubanio.com URLs
- [ ] `getDoubanImageUrl` returns empty string on error (triggers placeholder)
- [ ] `VodCard.vue` and `HotDetail.vue` both updated to use reactive poster URLs
- [ ] Commands registered in `main.rs` `generate_handler!` macro
