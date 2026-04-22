# TVBox WenCai / JianPian Playback Design

## Context

The current desktop app already supports several Fantaihard-derived VOD sources:

- `xb6v`: direct or share-page-resolvable playback
- `libvio`: detail-page hydration plus resolvable direct playback
- `auete`: multi-line playback with post-resolution stream probing
- `zxzj`: currently treated as embedded-only and therefore hidden from desktop-facing catalog surfaces

The next gap is support for `文采` and `荐片` lines referenced by Fantaihard-style TVBox configurations. The user has explicitly confirmed that these sources must follow the same filtering rules already used for other desktop playback sources:

- only show lines that resolve to true media streams
- hide broken lines after probing
- hide embedded pages, external tools, and shell-page-only playback targets

## Goal

Add `文采` and `荐片` as desktop-usable VOD sources while preserving the current playback contract:

1. catalog items should only surface if they can produce desktop-playable lines
2. detail pages should only show lines that resolve to real streams and pass probe checks
3. broken upstream lines must be filtered before the user clicks them

## Non-Goals

- No support for embedded-only playback pages
- No external helper / browser / download-tool playback for these sources
- No full TVBox runtime generalization in this step
- No subscription-refresh-time full pre-resolution of all episodes

## Approach Options

### Option 1: Resolver-only support

Keep existing catalog/detail scraping unchanged and only add playback-page resolution for `文采` and `荐片`.

Pros:

- smallest code change

Cons:

- detail views may still expose lines that later disappear or fail
- source quality remains opaque until playback time

### Option 2: Detail hydration plus resolver probing

Hydrate `文采` and `荐片` detail pages on demand, extract candidate play pages, resolve them to true streams, and only return probed playable candidates.

Pros:

- matches current desktop playback policy
- avoids expensive full-library pre-resolution
- gives the user cleaner detail pages with fewer dead lines

Cons:

- first detail open can be slower than fully cached sources

### Option 3: Refresh-time full pre-resolution

Resolve and probe all episodes during subscription refresh and persist only fully playable lines.

Pros:

- best runtime UX after refresh

Cons:

- refresh latency grows too much for Fantaihard-scale libraries
- unnecessary pressure on upstreams

## Decision

Use **Option 2**.

This preserves the existing desktop rule set and keeps the work aligned with the already-established `auete/libvio` model: catalog entry now, detail hydration on demand, resolver-backed playable-line filtering before presentation.

## Design

### 1. Source Identification

Add source detection for `文采` and `荐片` within the supported TVBox catalog pipeline.

Expected outcome:

- source records from Fantaihard can be recognized as `wencai` and `jianpian`
- these sources become eligible for catalog scraping and detail hydration

This should follow the same pattern already used for `xb6v`, `libvio`, `auete`, and `zxzj`: source-key matching first, then fallback matching via raw JSON / ext / api markers if needed.

### 2. Catalog Scraping

Implement source-specific catalog scrapers that extract:

- title
- item type
- poster when available
- source detail URL
- source marker in `detail_json`

Catalog scraping should stay shallow:

- no episode pre-resolution at refresh time
- enough metadata to populate library surfaces

Only sources that can eventually participate in the desktop playback pipeline should be written into `catalog_items`.

### 3. Detail Hydration

Implement source-specific detail parsers for `文采` and `荐片`.

Each parser should:

- fetch the detail page
- extract grouped episode/play entries
- normalize play-page URLs
- preserve human-readable source/line names
- skip obviously external entries such as cloud-disk, magnet, thunder, or download-only links

The output should still be raw candidate play pages rather than final streams. Final filtering happens in the resolver.

### 4. Playback Resolution

Add resolver branches for `文采` and `荐片`.

Each branch must:

1. fetch the source-specific play page
2. extract the final `m3u8` / `mp4` / equivalent direct media URL
3. if multiple same-episode lines exist, gather all candidates for that episode
4. probe each resolved candidate before returning it
5. discard any candidate that fails probing

Probe requirements stay consistent with current HLS policy:

- master playlist must load
- media playlist must load
- HLS key must load when present
- at least one real segment/resource must load

For non-HLS direct media URLs, a lightweight range probe remains sufficient.

### 5. Visibility Rules

The app must keep enforcing these presentation rules:

- `direct` and `resolvable` lines are visible
- `embedded` and `external` lines are hidden
- if a catalog item can only yield embedded/external/broken candidates, it should not appear in desktop catalog surfaces

This means the new sources should be treated like current desktop-native sources, not like fallback content.

### 6. Data Flow

The intended runtime path is:

1. subscription refresh identifies `文采` / `荐片` sources and writes shallow catalog items
2. user opens a catalog item
3. backend hydrates detail episodes on demand
4. resolver turns selected play pages into true stream candidates
5. failed candidates are filtered before the frontend sees them
6. frontend receives only desktop-usable lines

This keeps UI logic simple and concentrates source-specific complexity in backend services.

### 7. Error Handling

Source failures should degrade quietly and predictably:

- catalog scrape failure for one source should not fail the whole subscription refresh
- detail hydration failure should leave the item without visible lines rather than exposing bad fallback lines
- resolver failure should return no candidates plus a source-specific error message for logs and diagnostics

The user-facing rule remains simple: if a line is shown, it should already have passed basic desktop-compatibility checks.

## Files / Modules Affected

Likely modules:

- `src-tauri/src/services/xb6v.rs`
  - extend supported TVBox source dispatch
- `src-tauri/src/services/resolver.rs`
  - add `wencai` and `jianpian` resolver branches
- `src-tauri/src/services/mod.rs`
  - export new services where needed
- new service modules, likely:
  - `src-tauri/src/services/wencai.rs`
  - `src-tauri/src/services/jianpian.rs`
- `src-tauri/src/services/storage.rs`
  - no policy change expected beyond consuming the new source outputs

Frontend changes should be minimal because the current UI already consumes filtered candidates returned by backend commands.

## Testing Strategy

### Unit tests

For each source:

- listing-page parsing
- detail-page parsing
- play-page final-stream extraction
- candidate filtering behavior

For resolver:

- same-episode multi-line extraction
- broken-line filtering
- direct/resolvable classification remains correct

### Real-network tests

Add ignored live-network tests for both sources that verify:

- a real detail page produces episodes
- at least one resolved candidate survives probing

### Regression checks

Run:

- targeted resolver tests
- full Rust test suite
- frontend production build

## Success Criteria

This work is complete when:

1. `文采` catalog items appear in the library only when they can yield desktop-playable lines
2. `荐片` catalog items appear in the library only when they can yield desktop-playable lines
3. detail pages for these sources only show lines that resolved and passed probes
4. broken `文采/荐片` upstream lines are filtered before user interaction
5. no embedded-only or external-only lines from these sources are shown in the desktop UI
