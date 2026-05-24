# Source Line Name Differentiation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract YPanSo `player_aaaa.name` field (e.g. "高清", "标清") and display it in the playback source panel and control bar instead of generic source name.

**Architecture:** Modify `extract_ypanso_player_url` to return both `url` and `name`. Pass `name` via `PlaybackTarget.meta`. Add `lineName` to TypeScript types. Display in PlaybackDrawer and control bar.

**Tech Stack:** Rust (Tauri backend), TypeScript, Vue 3

---

## Task 1: Modify extract_ypanso_player_url to return (url, name)

**Files:**
- Modify: `src-tauri/src/services/provider/ypanso_scraper.rs` — change function signature and body

- [ ] **Step 1: Read the current function (lines 235-248)**

Read the current `extract_ypanso_player_url` function to understand its structure.

- [ ] **Step 2: Modify function to return Option<(String, Option<String>)>**

Replace the function body:

```rust
/// Ypanso uses maccms (Apple CMS) which embeds a `player_aaaa` JSON object
/// with the real video source URL in a `url` field and an optional `name` field
/// for the line name (e.g. "高清线路", "标清").
fn extract_ypanso_player_url(body: &str) -> Option<(String, Option<String>)> {
    // (?s) enables dotall mode so `.` matches newlines – the player_aaaa JSON may span multiple lines.
    // Use the same relaxed pattern as resolver::extract_maccms_player_url to handle:
    // - spaces around the equals sign (player_aaaa = {...})
    // - other player variable names like player_bbbb used by some maccms themes
    let player_regex = Regex::new(r"(?s)player_[a-z]{4}\s*=\s*(\{.*?\})</script>").ok()?;
    player_regex.captures(body).and_then(|captures| {
        let json_str = captures.get(1).map(|m| m.as_str())?;
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let url = parsed.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())?;
        let name = parsed.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        Some((url, name))
    })
}
```

- [ ] **Step 3: Update the play() method to use the new return type**

Read the `play()` method (around line 50) and update it:

```rust
pub async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
    let body = self.base.fetch_text(play_url).await?;
    let (video_url, line_name) = extract_ypanso_player_url(&body)
        .unwrap_or_else(|| (play_url.to_string(), None));

    Ok(vec![PlaybackTarget {
        episode_id: None,
        source_key: "YpanSo".to_string(),
        target_url: video_url,
        target_kind: PlaybackTargetKind::Direct,
        resolver_key: None,
        headers: None,
        sort_hint: 0,
        meta: line_name,
        referer: Some(play_url.to_string()),
    }])
}
```

- [ ] **Step 4: Update tests to expect new return type**

Read the tests (lines 262-330) and update them:

The test at line 292 currently expects `Option<String>`:
```rust
assert_eq!(
    extract_ypanso_player_url(html).as_deref(),
    Some("https://cdn.example.com/video.m3u8")
);
```

Change to:
```rust
assert_eq!(
    extract_ypanso_player_url(html).as_deref(),
    Some(("https://cdn.example.com/video.m3u8", Some("标清".to_string())))
);
```

Similarly update all other tests:
- Line 285: `(url, Some("高清线路"))`
- Line 295: `(url, Some("标清".to_string()))`
- Line 302: `is_none()`
- Line 308: `is_none()`
- Line 316: `(url, Some("高清".to_string()))`
- Line 326: `(url, Some("标清".to_string()))`

- [ ] **Step 5: Run cargo test to verify**

Run: `cd src-tauri && cargo test ypanso`
Expected: All YpanSo tests pass

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/provider/ypanso_scraper.rs
git commit -m "feat(ypanso): extract line name from player_aaaa.name field"
```

---

## Task 2: Add lineName field to TypeScript UnifiedEpisodeSource

**Files:**
- Modify: `src/types/index.ts` — add `lineName?: string` to `UnifiedEpisodeSource`

- [ ] **Step 1: Read UnifiedEpisodeSource definition (lines 202-206)**

```ts
export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  episode: CatalogEpisode
}
```

- [ ] **Step 2: Add lineName field**

```ts
export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  lineName?: string  // 线路名，如"高清"、"标清"（从 player_aaaa.name 提取）
  episode: CatalogEpisode
}
```

- [ ] **Step 3: Commit**

```bash
git add src/types/index.ts
git commit -m "feat(types): add lineName optional field to UnifiedEpisodeSource"
```

---

## Task 3: Update episode merge logic to populate lineName

**Files:**
- Modify: `src/utils/episode.ts` (or wherever `mergeEpisodes` is defined)

- [ ] **Step 1: Find where UnifiedEpisodeSource objects are created**

Search for `UnifiedEpisodeSource` creation in the codebase:
```bash
grep -rn "UnifiedEpisodeSource" src/
```

- [ ] **Step 2: Add lineName from meta field**

Where the `sources` array is built from `PlaybackTarget`, extract `meta` into `lineName`:

```ts
sources: targets.map(t => ({
  sourceKey: t.source_key,
  sourceName: getSourceDisplayName(t.source_key), // existing function
  lineName: t.meta ?? undefined,  // extract meta as lineName
  episode: { id, episode_label, play_url, order_index }
}))
```

- [ ] **Step 3: Commit**

```bash
git add src/
git commit -m "feat(episode): populate lineName from PlaybackTarget.meta"
```

---

## Task 4: Update PlaybackDrawer to display lineName

**Files:**
- Modify: `src/components/player/PlaybackDrawer.vue` — use `lineName || sourceName`

- [ ] **Step 1: Read PlaybackDrawer line 160**

```vue
<span class="source-row-label">{{ attempt.source.sourceName }}</span>
```

- [ ] **Step 2: Change to lineName with fallback**

```vue
<span class="source-row-label">{{ attempt.source.lineName || attempt.source.sourceName }}</span>
```

- [ ] **Step 3: Commit**

```bash
git add src/components/player/PlaybackDrawer.vue
git commit -m "feat(playback): show lineName with fallback to sourceName in source panel"
```

---

## Task 5: Add current line name display to PlayerPage control bar

**Files:**
- Modify: `src/views/PlayerPage.vue` — add currentLineName computed and display in control bar

- [ ] **Step 1: Read PlayerPage control bar area (lines 2140-2175)**

Find where the control bar buttons are rendered.

- [ ] **Step 2: Add currentLineName computed property**

After existing computed properties, add:

```ts
const currentLineName = computed(() => {
  const attempt = episodeSourceAttempts.value?.find(a => a.status === 'playing')
  return attempt?.source.lineName ?? null
})
```

- [ ] **Step 3: Add line name badge to control bar**

Find where the fullscreen button ends (around line 2144) and add a line name badge:

```vue
<span v-if="currentLineName" class="line-name-badge">{{ currentLineName }}</span>
```

Add CSS for `.line-name-badge` in the style section:

```css
.line-name-badge {
  font-size: 0.75rem;
  padding: 0.2rem 0.5rem;
  background: rgba(255,255,255,0.1);
  border-radius: 0.25rem;
  color: var(--text-secondary);
}
```

- [ ] **Step 4: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): show current line name in control bar"
```

---

## Verification

1. Run `npm run build` — should compile without errors
2. Run `cargo test -p tvbox` in src-tauri — all tests pass
3. Navigate to a YPanSo video with multiple lines
4. Open playback drawer — lines should show "高清", "标清" instead of "YPanSo"
5. Control bar should show current line name when watching