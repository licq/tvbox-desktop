# Source-Aware UI and Playback Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace generic UI state with real source-aware state across home, category, subscription management, detail, and player flows.

**Architecture:** Add missing backend read models first, then consume them from Pinia stores and views. Player routing should prefer `episodeId` and load playback context from storage/runtime instead of relying on generic URL labels. UI changes should hide noisy empty/diagnostic panels from consumer pages and move diagnostics into source management and player diagnostics.

**Tech Stack:** Rust/Tauri commands, rusqlite storage, Vue 3, Pinia, vue-router, Vitest, `cargo test`, `npm test`, `npm run build`.

---

## File Structure

- Modify `src-tauri/src/models/mod.rs`: add source-aware DTOs for source health and playback context.
- Modify `src-tauri/src/services/storage.rs`: add source badges to catalog/home queries, return continue watching from history, add source health summary queries, add episode playback context query.
- Modify `src-tauri/src/commands/subscription.rs`: expose source health summaries or extend subscription payloads.
- Modify `src-tauri/src/commands/player.rs`: expose playback context by `episode_id`.
- Modify `src-tauri/src/main.rs`: register new Tauri commands.
- Modify `src/types/index.ts`: mirror backend DTOs.
- Modify `src/stores/library.ts`: consume source badges and hide empty continue rail at view level.
- Modify `src/stores/subscription.ts`: fetch source summaries for management UI.
- Modify `src/stores/playback.ts`: resolve playback by episode context and preserve source labels.
- Modify `src/views/Home.vue`: split home landing and category catalog behavior.
- Modify `src/views/Subscriptions.vue`: turn source management into a health/control page.
- Modify `src/views/VodDetail.vue`: route to player with `episodeId` and no raw URL dependency for normal flow.
- Modify `src/views/PlayerPage.vue`: load source-aware playback context and use robust fullscreen.
- Modify `src/components/home/ContinueRail.vue`: render nothing when empty, instead of a large empty state.
- Modify `src/components/player/PlaybackDrawer.vue`: show source/episode labels and diagnostics.
- Add `src/utils/fullscreen.ts`: browser fullscreen plus Tauri window fullscreen fallback.
- Add or extend frontend tests under `src/stores/__tests__` and `src/utils/__tests__`.
- Add backend tests inside existing `#[cfg(test)]` modules in `src-tauri/src/services/storage.rs`, `src-tauri/src/services/resolver.rs`, and `src-tauri/src/services/jianpian.rs`.

---

### Task 1: Backend Home Payload Uses Real History and Source Badges

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Replace the existing empty-history test with a failing history test**

In `src-tauri/src/services/storage.rs`, replace `library_home_returns_empty_continue_watching_for_now` with:

```rust
#[test]
fn library_home_returns_continue_watching_from_vod_history() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("荐片", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");

    seed_catalog_item_with_source(&storage, subscription.id, 101, "示例影片", "movie", "jianpian");
    storage
        .save_play_history("vod", 101, 42.0)
        .expect("play history should insert");

    let home = storage
        .get_library_home()
        .expect("library home should query");

    assert_eq!(home.continue_watching.len(), 1);
    assert_eq!(home.continue_watching[0].id, 101);
    assert_eq!(home.continue_watching[0].title, "示例影片");
    assert_eq!(home.continue_watching[0].item_type, "movie");
    assert_eq!(home.continue_watching[0].progress, Some(42.0));
    assert_eq!(home.continue_watching[0].source_badge.as_deref(), Some("荐片"));
}
```

- [ ] **Step 2: Extend the Rust model**

In `src-tauri/src/models/mod.rs`, change `HomeCatalogItem` to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeCatalogItem {
    pub id: i64,
    pub title: String,
    pub item_type: String,
    pub poster: Option<String>,
    pub progress: Option<f64>,
    pub source_badge: Option<String>,
    pub update_badge: Option<String>,
}
```

- [ ] **Step 3: Update `query_home_catalog_items` mapping**

In `src-tauri/src/services/storage.rs`, update `query_home_catalog_items` so every query selects:

```sql
ci.id,
ci.title,
ci.item_type,
ci.poster,
<progress expression> AS progress,
s.name AS source_badge,
NULL AS update_badge
```

And maps:

```rust
Ok(HomeCatalogItem {
    id: row.get(0)?,
    title: row.get(1)?,
    item_type: row.get(2)?,
    poster: row.get(3)?,
    progress: row.get(4)?,
    source_badge: row.get(5)?,
    update_badge: row.get(6)?,
})
```

- [ ] **Step 4: Implement history query**

In `get_library_home()`, replace `let continue_watching = Vec::new();` with a query that joins history to enabled catalog items:

```sql
SELECT ci.id, ci.title, ci.item_type, ci.poster, ph.progress, s.name AS source_badge, '继续观看' AS update_badge
FROM play_history ph
INNER JOIN catalog_items ci ON ph.item_type = 'vod' AND ph.item_id = ci.id
INNER JOIN subscriptions s ON ci.subscription_id = s.id
WHERE s.enabled = 1
  AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
ORDER BY ph.last_played DESC
LIMIT 12
```

- [ ] **Step 5: Update all `get_catalog_items()` and home latest/featured selects**

Ensure each `query_home_catalog_items` caller selects the seven fields above. For non-history rows use `NULL AS progress` and a useful update badge such as `NULL AS update_badge`.

- [ ] **Step 6: Run backend test**

Run:

```bash
cd src-tauri && cargo test -q library_home_returns_continue_watching_from_vod_history
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/services/storage.rs
git commit -m "feat: return source-aware home history"
```

---

### Task 2: Source Health Summary Backend

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/services/storage.rs`
- Modify: `src-tauri/src/commands/subscription.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/types/index.ts`

- [ ] **Step 1: Add failing storage test**

Add this test to `src-tauri/src/services/storage.rs`:

```rust
#[test]
fn source_health_summaries_count_live_catalog_and_episode_rows() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("饭太硬", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");

    seed_live_source(&storage, subscription.id, Some("央视频道"), "CCTV-1", "https://live.example/cctv1.m3u8");
    seed_catalog_item_with_source(&storage, subscription.id, 201, "示例电影", "movie", "jianpian");
    seed_catalog_episode(&storage, 201, "荐片线路", "第01集", "https://media.example/index.m3u8", 0);

    let summaries = storage
        .get_source_health_summaries()
        .expect("source summaries should query");

    let summary = summaries
        .iter()
        .find(|summary| summary.id == subscription.id)
        .expect("subscription summary should exist");
    assert_eq!(summary.name, "饭太硬");
    assert_eq!(summary.live_channel_count, 1);
    assert_eq!(summary.catalog_item_count, 1);
    assert_eq!(summary.catalog_episode_count, 1);
    assert!(summary.enabled);
}
```

- [ ] **Step 2: Add model**

In `src-tauri/src/models/mod.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceHealthSummary {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub kind: String,
    pub enabled: bool,
    pub last_refreshed_at: Option<String>,
    pub last_error: Option<String>,
    pub live_channel_count: i64,
    pub catalog_item_count: i64,
    pub catalog_episode_count: i64,
}
```

- [ ] **Step 3: Implement storage method**

Add `Storage::get_source_health_summaries()` with a single grouped query:

```sql
SELECT s.id, s.name, s.url, s.kind, s.enabled, s.last_refreshed_at, s.last_error,
       COUNT(DISTINCT sl.id) AS live_channel_count,
       COUNT(DISTINCT ci.id) AS catalog_item_count,
       COUNT(DISTINCT ce.id) AS catalog_episode_count
FROM subscriptions s
LEFT JOIN source_lives sl ON sl.subscription_id = s.id
LEFT JOIN catalog_items ci ON ci.subscription_id = s.id
LEFT JOIN catalog_episodes ce ON ce.catalog_item_id = ci.id
GROUP BY s.id
ORDER BY s.id DESC
```

- [ ] **Step 4: Add command**

In `src-tauri/src/commands/subscription.rs`:

```rust
#[tauri::command]
pub async fn get_source_health_summaries(
    state: State<'_, AppState>,
) -> Result<Vec<SourceHealthSummary>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage
            .get_source_health_summaries()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

Import `SourceHealthSummary`.

- [ ] **Step 5: Register command**

In `src-tauri/src/main.rs`, add:

```rust
tvbox_lib::commands::subscription::get_source_health_summaries,
```

- [ ] **Step 6: Mirror frontend type**

In `src/types/index.ts` add:

```ts
export interface SourceHealthSummary extends SourceSubscription {
  live_channel_count: number
  catalog_item_count: number
  catalog_episode_count: number
}
```

- [ ] **Step 7: Verify**

Run:

```bash
cd src-tauri && cargo test -q source_health_summaries_count_live_catalog_and_episode_rows
npm run build
```

Expected: both pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/services/storage.rs src-tauri/src/commands/subscription.rs src-tauri/src/main.rs src/types/index.ts
git commit -m "feat: expose source health summaries"
```

---

### Task 3: Home and Category Page Separation

**Files:**
- Modify: `src/views/Home.vue`
- Modify: `src/components/home/ContinueRail.vue`
- Modify: `src/stores/__tests__/library.spec.ts`

- [ ] **Step 1: Add store test for source badge fields**

Extend `src/stores/__tests__/library.spec.ts` existing normalization test with:

```ts
expect(store.continueWatching[0].source_badge).toBe('荐片')
expect(store.continueWatching[0].update_badge).toBe('继续观看')
```

Expected currently passes if Task 1 frontend type mapping is complete.

- [ ] **Step 2: Make `ContinueRail` render nothing when empty**

In `src/components/home/ContinueRail.vue`, change template root to:

```vue
<template>
  <section v-if="items.length" class="media-rail continue-rail">
    <div class="media-rail-header">
      <div>
        <div class="section-title">继续观看</div>
        <p>从播放历史接回，不需要先回到目录里找。</p>
      </div>
    </div>

    <div class="media-rail-track">
      <button
        v-for="item in items"
        :key="item.id"
        class="media-rail-card continue-rail-card"
        type="button"
        @click="$emit('select', item)"
      >
        <MediaCard
          :title="item.title"
          :poster="item.poster"
          :subtitle="item.update_badge || `${formatProgress(item.progress)}% watched`"
          :source-badge="item.source_badge"
        />
        <span v-if="item.progress !== undefined" class="continue-rail-progress">
          <span :style="{ width: `${formatProgress(item.progress)}%` }"></span>
        </span>
      </button>
    </div>
  </section>
</template>
```

- [ ] **Step 3: Split route rendering in `Home.vue`**

Add computed flags:

```ts
const isLandingTab = computed(() => activeTab.value === 'live')
const isCatalogTab = computed(() => activeTab.value !== 'live' && activeTab.value !== 'hot')
```

Render the landing sections only for `isLandingTab`:

```vue
<template v-if="isLandingTab">
  <HomeHero ... />
  <ContinueRail ... />
  <MediaRail ... />
  <LiveNowPanel ... />
</template>
```

Remove `<SourceHealthPanel :subscriptions="subStore.subscriptions" />` from the home landing.

- [ ] **Step 4: Replace category header copy**

For catalog tabs, replace the current secondary browser intro with direct catalog copy:

```vue
<h2>{{ formatTypeLabel(activeTab) }}</h2>
<p>{{ libraryStore.catalogItems.length }} 个条目。搜索会在当前分类内筛选。</p>
```

For live tab, keep live search. For hot tab, keep hot rendering.

- [ ] **Step 5: Add compact source warning**

If `failedSubscriptions.length > 0`, render a small banner near the top:

```vue
<RouterLink v-if="failedSubscriptions.length" to="/subscriptions" class="source-warning-banner">
  {{ failedSubscriptions.length }} 个订阅源异常，去订阅管理查看
</RouterLink>
```

Add minimal CSS in `src/style.css` for `.source-warning-banner`.

- [ ] **Step 6: Verify**

Run:

```bash
npm test
npm run build
```

Expected: frontend tests/build pass.

- [ ] **Step 7: Commit**

```bash
git add src/views/Home.vue src/components/home/ContinueRail.vue src/style.css src/stores/__tests__/library.spec.ts
git commit -m "feat: separate home landing from catalog pages"
```

---

### Task 4: Subscription Management Source Console

**Files:**
- Modify: `src/stores/subscription.ts`
- Modify: `src/views/Subscriptions.vue`
- Modify: `src/style.css`

- [ ] **Step 1: Extend store**

In `src/stores/subscription.ts`, add:

```ts
import type { SourceHealthSummary, SourceSubscription } from '@/types'
```

Add state and action:

```ts
const sourceHealth = ref<SourceHealthSummary[]>([])

async function fetchSourceHealth() {
  loading.value = true
  error.value = null
  try {
    sourceHealth.value = await invoke<SourceHealthSummary[]>('get_source_health_summaries')
  } catch (e) {
    error.value = String(e)
    throw e
  } finally {
    loading.value = false
  }
}
```

Return `sourceHealth` and `fetchSourceHealth`.

- [ ] **Step 2: Update mounted flow**

In `src/views/Subscriptions.vue`, replace mounted fetch with:

```ts
onMounted(async () => {
  try {
    await Promise.all([
      subStore.fetchSubscriptions(),
      subStore.fetchSourceHealth()
    ])
  } catch (e) {
    alert('加载订阅失败: ' + e)
  }
})
```

After refresh/toggle/delete/add, call `await subStore.fetchSourceHealth()`.

- [ ] **Step 3: Render health summaries**

Replace subscription list item content with rows driven by `subStore.sourceHealth`:

```vue
<div v-else class="source-console-list">
  <div v-for="source in subStore.sourceHealth" :key="source.id" class="source-console-row">
    <div class="source-console-main">
      <strong>{{ source.name }}</strong>
      <small>{{ source.url }}</small>
      <p v-if="source.last_error">{{ source.last_error }}</p>
    </div>
    <div class="source-console-metrics">
      <span>{{ source.live_channel_count }} 直播</span>
      <span>{{ source.catalog_item_count }} 片库</span>
      <span>{{ source.catalog_episode_count }} 选集</span>
    </div>
    <div class="source-console-actions">
      <!-- reuse toggle, refresh, delete buttons with source.id -->
    </div>
  </div>
</div>
```

When an action needs `SourceSubscription`, find it from `subStore.subscriptions.find(sub => sub.id === source.id)`.

- [ ] **Step 4: Replace alert-only refresh result**

Keep refresh failures visible inline by relying on `source.last_error`. Remove success `alert('刷新成功')`; refresh button text and updated counts are the feedback.

- [ ] **Step 5: Verify**

Run:

```bash
npm run build
```

Expected: build passes.

- [ ] **Step 6: Commit**

```bash
git add src/stores/subscription.ts src/views/Subscriptions.vue src/style.css
git commit -m "feat: move source health into subscription console"
```

---

### Task 5: Playback Context by Episode ID

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/services/storage.rs`
- Modify: `src-tauri/src/commands/player.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src/types/index.ts`
- Modify: `src/stores/playback.ts`
- Modify: `src/views/VodDetail.vue`
- Modify: `src/views/PlayerPage.vue`
- Modify: `src/components/player/PlaybackDrawer.vue`
- Modify: `src/stores/__tests__/playback.spec.ts`

- [ ] **Step 1: Add failing storage test for episode context**

In `src-tauri/src/services/storage.rs`:

```rust
#[test]
fn playback_context_includes_catalog_episode_and_source_name() {
    let storage = Storage::new(unique_test_dir()).expect("storage should initialize");
    let subscription = storage
        .add_subscription("饭太硬", "https://example.com/tvbox.json")
        .expect("subscription should be inserted");
    seed_catalog_item(&storage, subscription.id, 301, "示例剧集", "series");
    seed_catalog_episode(&storage, 301, "金牌资源", "第02集", "https://example.com/play/2", 2);

    let context = storage
        .get_playback_context_for_episode(1)
        .expect("context query should run")
        .expect("context should exist");

    assert_eq!(context.catalog_item_id, 301);
    assert_eq!(context.catalog_title, "示例剧集");
    assert_eq!(context.source_name, "金牌资源");
    assert_eq!(context.episode_label, "第02集");
    assert_eq!(context.original_url, "https://example.com/play/2");
}
```

If the seeded episode id is not `1` due previous test data assumptions, query inserted id inside the helper or add a helper returning `last_insert_rowid()`.

- [ ] **Step 2: Add models**

In `src-tauri/src/models/mod.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackContext {
    pub catalog_item_id: i64,
    pub catalog_title: String,
    pub episode_id: i64,
    pub episode_label: String,
    pub source_name: String,
    pub original_url: String,
    pub resolved: ResolvedPlayback,
}

#[derive(Debug, Clone)]
pub struct StoredEpisodePlaybackContext {
    pub catalog_item_id: i64,
    pub catalog_title: String,
    pub episode_id: i64,
    pub episode_label: String,
    pub source_name: String,
    pub original_url: String,
}
```

- [ ] **Step 3: Add storage query**

Implement `Storage::get_playback_context_for_episode(episode_id: i64) -> SqliteResult<Option<StoredEpisodePlaybackContext>>`:

```sql
SELECT ci.id, ci.title, ce.id, ce.episode_label,
       COALESCE(NULLIF(TRIM(ce.source_name), ''), '默认来源') AS source_name,
       ce.play_url
FROM catalog_episodes ce
INNER JOIN catalog_items ci ON ce.catalog_item_id = ci.id
INNER JOIN subscriptions s ON ci.subscription_id = s.id
WHERE s.enabled = 1 AND ce.id = ?1
```

- [ ] **Step 4: Add Tauri command**

In `src-tauri/src/commands/player.rs`, add:

```rust
#[tauri::command]
pub async fn resolve_playback_context(
    episode_id: i64,
    state: State<'_, AppState>,
) -> Result<PlaybackContext, String> {
    let storage = state.storage.clone();
    let stored = tokio::task::spawn_blocking({
        let storage = storage.clone();
        move || storage.get_playback_context_for_episode(episode_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??
    .ok_or_else(|| "播放集数不存在或所属订阅未启用".to_string())?;

    let resolved = crate::services::playback_runtime::resolve_playback_for_input(
        &storage,
        &stored.original_url,
        Some(stored.episode_id),
    )
    .await?;

    Ok(PlaybackContext {
        catalog_item_id: stored.catalog_item_id,
        catalog_title: stored.catalog_title,
        episode_id: stored.episode_id,
        episode_label: stored.episode_label,
        source_name: stored.source_name,
        original_url: stored.original_url,
        resolved,
    })
}
```

Register it in `src-tauri/src/main.rs`.

- [ ] **Step 5: Mirror frontend type and store action**

In `src/types/index.ts`:

```ts
export interface PlaybackContext {
  catalogItemId: number
  catalogTitle: string
  episodeId: number
  episodeLabel: string
  sourceName: string
  originalUrl: string
  resolved: ResolvedPlayback
}
```

In `src/stores/playback.ts`, add:

```ts
const context = ref<PlaybackContext | null>(null)

async function resolveContext(episodeId: number) {
  status.value = 'resolving'
  errorMessage.value = null
  try {
    const payload = await invoke<PlaybackContext>('resolve_playback_context', { episodeId })
    context.value = payload
    applyResolved(payload.resolved)
    candidates.value = payload.resolved.candidates.map(candidate => ({
      ...candidate,
      label: candidate.label || `${payload.sourceName} ${payload.episodeLabel}`
    }))
    return payload
  } catch (e) {
    status.value = 'failed'
    errorMessage.value = String(e)
    candidates.value = []
    currentIndex.value = 0
    throw e
  }
}
```

Return `context` and `resolveContext`.

- [ ] **Step 6: Route from detail by episode id**

In `src/views/VodDetail.vue`, change `handlePlay` to:

```ts
function handlePlay(episode: CatalogEpisode) {
  router.push(`/player/vod/${itemId.value}?episodeId=${episode.id}`)
}
```

Keep old URL fallback only in player.

- [ ] **Step 7: Load context in player**

In `src/views/PlayerPage.vue`, when `mode === 'vod'` and `episodeId.value` exists, call:

```ts
const context = await playbackStore.resolveContext(episodeId.value)
sources.value = context.resolved.candidates.map(candidate => ({
  url: candidate.url,
  label: candidate.label || `${context.sourceName} ${context.episodeLabel}`,
  kind: candidate.kind
}))
```

Use old `episodeUrl` branch only when no `episodeId` exists.

- [ ] **Step 8: Verify**

Run:

```bash
cd src-tauri && cargo test -q playback_context_includes_catalog_episode_and_source_name
npm test
npm run build
```

Expected: all pass.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/services/storage.rs src-tauri/src/commands/player.rs src-tauri/src/main.rs src/types/index.ts src/stores/playback.ts src/views/VodDetail.vue src/views/PlayerPage.vue src/components/player/PlaybackDrawer.vue src/stores/__tests__/playback.spec.ts
git commit -m "feat: resolve playback with episode source context"
```

---

### Task 6: Fullscreen Abstraction

**Files:**
- Create: `src/utils/fullscreen.ts`
- Create: `src/utils/__tests__/fullscreen.spec.ts`
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: Add fullscreen utility test**

Create `src/utils/__tests__/fullscreen.spec.ts`:

```ts
import { describe, expect, it, vi } from 'vitest'
import { enterFullscreen, exitFullscreen } from '@/utils/fullscreen'

describe('fullscreen utility', () => {
  it('uses element fullscreen before native fallback', async () => {
    const requestFullscreen = vi.fn().mockResolvedValue(undefined)
    const nativeEnter = vi.fn()

    await enterFullscreen({ requestFullscreen } as unknown as HTMLElement, nativeEnter)

    expect(requestFullscreen).toHaveBeenCalled()
    expect(nativeEnter).not.toHaveBeenCalled()
  })

  it('falls back to native fullscreen when element fullscreen rejects', async () => {
    const requestFullscreen = vi.fn().mockRejectedValue(new Error('unsupported'))
    const nativeEnter = vi.fn().mockResolvedValue(undefined)

    await enterFullscreen({ requestFullscreen } as unknown as HTMLElement, nativeEnter)

    expect(nativeEnter).toHaveBeenCalled()
  })

  it('exits document fullscreen before native fallback', async () => {
    const exitFullscreen = vi.fn().mockResolvedValue(undefined)
    const nativeExit = vi.fn()

    await exitFullscreen({ fullscreenElement: {} as Element, exitFullscreen } as unknown as Document, nativeExit)

    expect(exitFullscreen).toHaveBeenCalled()
    expect(nativeExit).not.toHaveBeenCalled()
  })
})
```

- [ ] **Step 2: Implement utility**

Create `src/utils/fullscreen.ts`:

```ts
export async function enterFullscreen(
  element: HTMLElement | null,
  nativeEnter: () => Promise<void>
) {
  if (element?.requestFullscreen) {
    try {
      await element.requestFullscreen()
      return
    } catch {
      // Tauri WebView can reject browser fullscreen; use native fallback.
    }
  }

  await nativeEnter()
}

export async function exitFullscreen(
  doc: Document,
  nativeExit: () => Promise<void>
) {
  if (doc.fullscreenElement && doc.exitFullscreen) {
    try {
      await doc.exitFullscreen()
      return
    } catch {
      // Keep fallback symmetrical with enterFullscreen.
    }
  }

  await nativeExit()
}
```

- [ ] **Step 3: Wire player**

In `src/views/PlayerPage.vue`, import:

```ts
import { getCurrentWindow } from '@tauri-apps/api/window'
import { enterFullscreen, exitFullscreen } from '@/utils/fullscreen'
```

Add:

```ts
const playerStageRef = ref<HTMLElement | null>(null)
const appWindow = getCurrentWindow()
```

Update `toggleFullscreen()`:

```ts
async function toggleFullscreen() {
  if (!fullscreen.value) {
    await enterFullscreen(playerStageRef.value, async () => {
      await appWindow.setFullscreen(true)
    })
    fullscreen.value = true
    return
  }

  await exitFullscreen(document, async () => {
    await appWindow.setFullscreen(false)
  })
  fullscreen.value = false
}
```

Add `ref="playerStageRef"` to `<section class="player-stage">`.

- [ ] **Step 4: Verify**

Run:

```bash
npx vitest run src/utils/__tests__/fullscreen.spec.ts src/utils/__tests__/player.spec.ts
npm run build
```

Expected: tests and build pass.

- [ ] **Step 5: Commit**

```bash
git add src/utils/fullscreen.ts src/utils/__tests__/fullscreen.spec.ts src/views/PlayerPage.vue
git commit -m "fix: use player surface fullscreen fallback"
```

---

### Task 7: 金牌资源 Diagnostics and Gating

**Files:**
- Modify: `src-tauri/src/services/jianpian.rs`
- Modify: `src-tauri/src/services/resolver.rs`
- Modify: `src-tauri/src/services/playback_runtime.rs`
- Modify: `src-tauri/src/services/playback_types.rs`

- [ ] **Step 1: Add parser regression for 金牌资源 source name**

In `src-tauri/src/services/jianpian.rs`, add or keep a test asserting:

```rust
assert_eq!(item.episodes[0].source_name, "金牌资源");
```

Use the existing parser test near the 金牌资源 fixture and extend its fixture with a realistic play URL such as `/vodplay/123-1-1.html` when the current fixture lacks a play URL.

- [ ] **Step 2: Add resolver/probe regression**

In `src-tauri/src/services/resolver.rs`, extract a pure helper named `classify_resolved_candidates_for_source(source_name: &str, candidates: Vec<PlaybackTarget>) -> Vec<PlaybackTarget>` if the candidate filtering logic is currently embedded in async runtime code. Add a unit test for that helper.

Target assertion:

```rust
assert!(
    candidates.iter().all(|candidate| candidate.kind != PlaybackTargetKind::Embed),
    "金牌资源 should not be surfaced as embedded-only playable candidates"
);
```

Also assert labels retain `"金牌资源"` when candidates exist.

- [ ] **Step 3: Ensure failed candidates do not become ready**

In playback runtime, make the final status rule explicit:

```rust
if playable_candidates.is_empty() && external_candidates.is_empty() {
    return ResolvedPlayback {
        status: "failed".to_string(),
        candidates: vec![],
        error_message: Some("当前集未找到通过探测的可播线路".to_string()),
    };
}
```

Do not include candidates that fail manifest/resource probing in the returned ready list.

- [ ] **Step 4: Preserve diagnostic reason in logs**

When a 金牌资源 candidate fails probe, log:

```rust
log::warn!(
    "playback probe failed source={} url={} reason={}",
    target.source_key,
    target.target_url,
    reason
);
```

- [ ] **Step 5: Verify targeted Rust tests**

Run:

```bash
cd src-tauri && cargo test -q jianpian
cd src-tauri && cargo test -q resolver
```

Expected: relevant parser and resolver tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/services/jianpian.rs src-tauri/src/services/resolver.rs src-tauri/src/services/playback_runtime.rs src-tauri/src/services/playback_types.rs
git commit -m "fix: gate unplayable jinpai playback routes"
```

---

### Task 8: Final Integration Verification

**Files:**
- No intended source changes unless verification reveals a defect.

- [ ] **Step 1: Run full frontend verification**

```bash
npm test
npm run build
```

Expected: tests pass; build passes with only the known hls chunk-size warning.

- [ ] **Step 2: Run backend verification**

```bash
cd src-tauri && cargo test -q
```

Expected: all Rust tests pass.

- [ ] **Step 3: Run desktop app**

```bash
npm run tauri dev
```

Manual checks:

- Home does not show the full source health panel.
- Empty continue watching does not occupy a large block.
- `/library/movie`, `/library/series`, and `/library/variety` show direct catalog pages without the home hero.
- Subscription page shows source counts and inline errors.
- Detail page episode click routes with `episodeId`.
- Player drawer shows real source and episode names.
- Fullscreen enters and exits in the Tauri window.
- 金牌资源 either plays or is hidden/failed with a clear reason.

- [ ] **Step 4: Restore generated frontend build output**

If `dist/index.html` changed after build:

```bash
git restore -- dist/index.html
```

- [ ] **Step 5: Commit any final fixes**

Only if Step 1-3 required source fixes:

```bash
git add <changed-source-files>
git commit -m "fix: polish source-aware playback integration"
```

Do not commit `src-tauri/target` build artifacts.

---

## Plan Self-Review

Spec coverage:

- Source health moved from home to subscription management: Tasks 2, 3, and 4.
- Continue watching backed by real history: Task 1 and Task 3.
- Category pages stop showing home landing content: Task 3.
- Player labels use source and episode context: Task 5.
- Fullscreen reliable fallback: Task 6.
- 金牌资源 diagnostics and gating: Task 7.
- Final verification: Task 8.

Placeholder scan:

- The plan contains no incomplete implementation markers.
- Steps that modify code include the concrete shape of the change or the target command and expected result.

Type consistency:

- Rust `PlaybackContext` uses `serde(rename_all = "camelCase")`; frontend type uses camelCase fields.
- `SourceHealthSummary` extends the existing frontend `SourceSubscription` fields.
- Normal player flow uses `episodeId`; old raw `episode` URL remains fallback only.
