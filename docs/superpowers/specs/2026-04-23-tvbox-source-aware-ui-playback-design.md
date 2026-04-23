# TVBox Source-Aware UI and Playback Repair Design

## Problem

The current app has several related experience failures:

- The home page shows source health as a primary content block, even though source health belongs in source management.
- The "continue watching" rail is always empty because the backend currently returns an empty list.
- Category pages for movies, series, variety, and anime still show home-page hero content and explanatory panels before the requested catalog.
- Playback line names such as "直播线路 1" or generic playback labels do not tell the user which source or route is being used.
- Player fullscreen is unreliable because it targets `document.documentElement` rather than the player surface or native window.
- Some source families, especially "金牌资源", appear in the UI but fail at playback time.

These are not isolated copy or styling bugs. They come from the same root issue: the UI presents layout placeholders instead of source-aware product state. The app needs to carry source, episode, route, health, and playback capability through the data flow and show those facts where they are useful.

## Goals

1. Move source health from the home page into the subscription management page.
2. Make the home page content-focused: featured content, latest updates, category entry points, and optional continue watching only when real history exists.
3. Make category pages direct catalog pages, not a copy of the home landing page.
4. Make playback source-aware by preserving source names and episode context from detail page to player.
5. Fix fullscreen using a reliable player-surface-first strategy with a native fallback.
6. Diagnose and gate bad playback routes such as 金牌资源 so unusable lines are not promoted as normal playable lines.

## Non-Goals

- This change will not redesign the entire visual language again.
- This change will not add proxy playback.
- This change will not guarantee every third-party source can play. It will make unplayable lines diagnosable and stop presenting them as normal successful lines.
- This change will not remove existing source scrapers unless a specific route is proven unusable.

## Information Architecture

### Home

The home page should answer "what can I watch now?"

It should show:

- A focused hero from featured or latest content.
- Latest updates grouped by content type.
- Continue watching only when there is at least one real play history item.
- A compact source-warning banner only when enabled sources have errors, linking to subscription management.

It should not show:

- A full source health table.
- Large empty states for continue watching.
- Design-explanation copy such as "保留原有路由分类...".

### Source Management

The subscription page becomes the source control center.

Each subscription row should show:

- Enabled or disabled status.
- Last refreshed time.
- Last error, if any.
- Live channel count.
- Catalog item count.
- Episode count, if available cheaply.
- A refresh action with clear progress and result.

This is the only page that should show detailed source health. The home page links here when attention is needed.

### Category Pages

Routes such as `/library/movie`, `/library/series`, `/library/variety`, and `/library/anime` should behave as direct catalog pages.

They should show:

- Category title and item count.
- Search.
- Optional source/type filters if the data is available.
- Catalog grid.
- Empty state that points to subscription refresh only when the catalog is actually empty.

They should not show:

- Home hero.
- Continue watching rail.
- Cross-category rails.
- Source health panel.

### Detail Page

The detail page should stay source-oriented, but every episode click must preserve enough context for player labels.

The minimum source-aware episode context is:

- `episode_id`
- `source_name`
- `episode_label`
- `catalog_item_id`
- `catalog_title`

The player should prefer resolving by `episode_id`, not by a naked query-string URL. URL-based playback can remain as a fallback for old routes.

### Player

The player page should answer "what is currently being played, through which source, and why did it fail if it fails?"

The source drawer should show:

- Source name, for example "金牌资源", "文采线路A", or "荐片线路B".
- Episode label.
- Candidate kind: hls, http, external, embed.
- Probe result if known.
- Failed status for lines that failed during this session.

Generic labels such as "播放地址三" or "直播线路 3" should only be fallback labels when no source name exists. Live playback should derive source labels from subscription names where possible.

## Data Model Changes

### Catalog Cards

`HomeCatalogItem` should include source display metadata:

- `source_badge`
- `updated_at` or a display update badge when available

The existing frontend `CatalogCard` already supports `source_badge` and `update_badge`; backend queries should populate them.

### Play History

`get_library_home()` should populate `continue_watching` from play history by joining `play_history` to `catalog_items`.

The returned item should include:

- Catalog item id.
- Title.
- Item type.
- Poster.
- Progress.
- Source badge if available.

If the history table cannot link an item, the row should be ignored rather than producing a broken card.

### Episode Playback Context

Add a backend command or extend an existing command so the player can load playback context by `episode_id`.

The result should include:

- Catalog item id and title.
- Episode id and label.
- Source name.
- Original play URL.
- Resolved candidates.

This avoids stuffing the player route with both raw URL and display metadata.

### Source Health Summary

Subscription records or a derived summary command should provide counts:

- Live channels.
- Catalog items.
- Catalog episodes.
- Last error.
- Last refreshed time.

Counts can be computed on demand for the subscription page. They do not need to be persisted unless performance requires it.

## Playback Diagnostics

Playback resolution should separate three outcomes:

- `ready`: at least one candidate is directly playable.
- `external_required`: only external or embedded lines were found.
- `failed`: no directly playable candidate passed extraction/probe.

For 金牌资源 and similar routes, add targeted tests that cover:

- Detail parsing produces the expected source names.
- Resolver extracts candidate URLs from the selected route.
- Probe can read the playlist manifest.
- Probe can read at least one playlist resource or segment reference when available.
- Failed candidates include a reason that can be shown in diagnostics or logs.

The UI should not promote failed or external-only candidates as normal playable lines. If every candidate fails, the detail/player page should show a clear "当前集没有内置可播线路" state.

## Fullscreen Strategy

The fullscreen button should:

1. Try `playerStageElement.requestFullscreen()`.
2. If that fails in Tauri/WebView, call the native Tauri window fullscreen API.
3. Listen for fullscreen changes and keep UI state in sync.
4. Exit fullscreen through the same abstraction.

The player should not call fullscreen on `document.documentElement` as the primary path.

## Error Handling

- Home should not block rendering when source refresh fails.
- Category pages should show catalog data already present even if one source fails.
- Subscription management should show refresh failures inline, not only via alert.
- Player should mark failed lines and auto-advance only to candidates that are plausible for direct playback.
- External or embed-only lines should not appear as normal playable candidates unless the user explicitly enables an external-open workflow later.

## Testing Plan

Frontend tests:

- Home hides continue watching when the list is empty.
- Home does not render the source health panel.
- Category routes render category catalog content without the home landing sections.
- Player source drawer displays real source names from playback context.
- Fullscreen abstraction falls back when browser fullscreen rejects.

Backend tests:

- `get_library_home()` returns play history rows when history exists.
- Source health summary counts channels, catalog items, and episodes per subscription.
- Playback context by `episode_id` returns source name, episode label, and resolved candidates.
- 金牌资源 sample route records a useful failure or a directly playable candidate; it must not silently produce generic unusable candidates.

Manual verification:

- Refresh 饭太硬 source.
- Open home and verify source status is not a main content block.
- Open movie, series, and variety routes and verify no home hero appears.
- Play a known working item and verify the drawer shows source names.
- Try a known 金牌资源 item and verify it either plays or is clearly hidden/failed with a reason.
- Test fullscreen enter and exit in the Tauri desktop window.

## Implementation Boundaries

The implementation should be split into small commits:

1. Backend source-aware models and queries.
2. Home and category page separation.
3. Subscription management source health UI.
4. Player playback context and source labels.
5. Fullscreen abstraction.
6. 金牌资源 diagnostics and gating.

Each step must be independently testable. Playback diagnostics should be implemented before UI claims that 金牌资源 is fixed.
