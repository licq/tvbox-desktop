# TVBox Guard Runtime Design

## Goal

Build a pure-Rust Guard runtime inside the existing Tauri backend so `csp_*Guard` TVBox sources can produce:

- catalog items
- detail episodes
- playable line candidates
- resolved media streams

The first implementation target is Fantaihard's `csp_JpysGuard` and `csp_JPJGuard`, but the architecture must be reusable for additional `csp_*Guard` sources without introducing Node.js, JVM, or an external spider host.

## Context

The current app can:

- parse `tvbox_config`
- cache `source_sites`, `source_parses`, and `source_lives`
- ingest direct web catalogs for specific sites such as `xb6v`, `libvio`, `auete`, and `zxzj`
- resolve several page-based play targets into real stream candidates

The current app cannot:

- execute `csp_*Guard` sources
- derive catalogs from `type=3` Guard-only TVBox site entries
- resolve Guard play targets into final media streams

This is why Fantaihard's `文采` and `贱贱/荐片` exist in `source_sites` but never produce `catalog_items`.

## Non-Goals

- Do not introduce Node.js, QuickJS, JVM, jar spiders, or any external runtime host.
- Do not attempt complete compatibility with arbitrary third-party TVBox spider jars.
- Do not implement generic JS rule execution in the first version.
- Do not redesign the frontend information architecture as part of this project.

## Options Considered

### Option 1: Source-Specific Hardcoding

Implement `JpysGuard` and `JPJGuard` as isolated one-off scrapers with no shared Guard abstraction.

Pros:

- Fastest route to visible catalog results

Cons:

- Conflicts with the goal of a reusable Guard framework
- Guarantees duplicated parsing, transport, and resolver logic when the next Guard source is added

### Option 2: Pure Rust Guard Protocol Bridge

Create a shared Guard runtime contract in Rust, then implement `JpysGuard` and `JPJGuard` adapters on top of it.

Pros:

- Fits the no-external-runtime constraint
- Reuses the existing storage and resolver pipeline
- Leaves room to add more `csp_*Guard` adapters later without rewriting the core flow

Cons:

- Requires adapter design work up front
- Does not magically support every Guard source in version one

### Option 3: General JS-Driven Runtime

Embed a JS engine such as `boa_engine` or `rquickjs` and build Guard execution on top of that.

Pros:

- Higher theoretical ceiling if future Guard behavior becomes heavily script-driven

Cons:

- Still requires a custom host API and protocol bridge
- Adds runtime complexity before there is a proven need
- Violates the project's preference for the simplest viable pure-Rust architecture

## Decision

Use Option 2: a pure-Rust Guard protocol bridge.

Version one will establish a reusable Guard runtime contract and implement adapters for:

- `csp_JpysGuard`
- `csp_JPJGuard`

Later Guard sources can plug into the same runtime if they can be modeled with the same operations.

## High-Level Architecture

The Guard runtime adds a new backend path parallel to the current page-scraper path.

### Existing path

`source_sites -> site-specific catalog scraper -> catalog_items -> catalog_detail -> resolver`

### New Guard path

`source_sites(type=3/csp_*Guard) -> guard adapter registry -> catalog/detail/play operations -> catalog_items/catalog_episodes -> resolver probe/filter`

## Components

### 1. Guard Registry

A registry maps `source_sites.api` values to Rust adapter implementations.

Examples:

- `csp_JpysGuard` -> `JpysGuardAdapter`
- `csp_JPJGuard` -> `JpjGuardAdapter`

Responsibilities:

- decide whether a `TvboxSiteRecord` is supported by the Guard runtime
- construct the correct adapter
- expose a uniform interface to catalog, detail, search, and playback resolution operations

### 2. Guard Adapter Trait

Each adapter implements the same contract. The exact Rust names may differ, but the capability surface must be:

- `list(category, page)`
- `search(keyword, page)`
- `detail(item_id)`
- `resolve(play_target)`

Return types must be converted into existing app models, not custom frontend-only shapes.

### 3. Guard Catalog Bridge

The current `scrape_supported_tvbox_catalogs` flow must gain a Guard branch.

Behavior:

- inspect `source_sites`
- select sites supported by the Guard registry
- request category lists from each adapter
- persist returned items into `catalog_items`

Important rule:

Guard sites must no longer depend on guessed web roots. If a site is Guard-backed, catalog ingestion must go through the adapter, not through `collect_site_roots`.

### 4. Guard Detail Bridge

When a `catalog_item.detail_json` indicates a Guard-backed item, detail loading must call the owning Guard adapter instead of page-specific HTML detail scraping.

Responsibilities:

- fetch Guard item detail
- normalize episodes into `catalog_episodes`
- preserve source labels and logical play targets
- keep the existing visibility filtering contract intact

### 5. Guard Resolver Bridge

Guard episodes will not necessarily store a direct media URL. They may store a logical Guard play target that requires one more adapter step.

Resolver behavior:

- detect Guard play targets
- delegate to the correct Guard adapter
- receive one or more candidate media URLs
- reuse the existing probe flow to keep only playable candidates

This preserves the current policy:

- direct/resolvable lines are shown
- embedded/external/broken lines are hidden

## Data Model

The current tables remain, but `detail_json` and `play_url` semantics expand.

### catalog_items.detail_json

Guard-backed items must include enough metadata to re-enter the correct adapter.

Minimum fields:

- `source = "guard"`
- `guard_key`
- `site_key`
- `site_name`
- `item_id`
- `item_type`

Example shape:

```json
{
  "source": "guard",
  "guard_key": "csp_JpysGuard",
  "site_key": "文采",
  "site_name": "💮文采┃秒播",
  "item_id": "1419",
  "item_type": "movie"
}
```

### catalog_episodes.play_url

For Guard-backed episodes, `play_url` may be a logical target rather than a direct media URL.

Example shape:

- `guard://csp_JpysGuard/1419/1/1`
- `guard://csp_JPJGuard/71483/1/1`

These URLs are internal transport identifiers consumed by the resolver, not literal frontend-playable URLs.

## Playback Visibility Policy

The existing storage and UI policy stays unchanged:

- visible catalog items must have at least one desktop-usable line after detail resolution
- visible episode groups must only include direct or resolvable lines
- external and embedded-only lines remain hidden

Guard adapters do not bypass this policy.

## Error Handling

The runtime must degrade gracefully at three levels.

### Site-level failure

If one Guard site fails category retrieval:

- log the failure
- continue other Guard sites
- do not fail the entire subscription refresh unless no usable site remains

### Item-level failure

If one Guard item fails detail retrieval:

- skip or retain stale cached data if present
- do not invalidate other items from the same site

### Candidate-level failure

If one candidate stream fails probing:

- discard the candidate
- continue testing other candidates
- only fail the whole episode if all candidates are unusable

## Testing Strategy

### Unit tests

For each adapter:

- category list parsing
- detail parsing
- logical play target parsing
- final candidate extraction

### Integration tests

Across the Guard runtime:

- `source_sites -> catalog_items`
- `detail_json -> catalog_episodes`
- `guard://... -> resolved playback candidates`

### Real-network tests

At minimum:

- one live detail test for `JpysGuard`
- one live detail test for `JPJGuard`
- one live playback resolution test for each

Ignored live tests are acceptable, but they must verify the real runtime entry points, not just parser helpers.

## Rollout Plan

### Phase 1

Create the Guard runtime abstractions and adapter registry.

### Phase 2

Implement `JpysGuardAdapter` and `JpjGuardAdapter` for:

- category listing
- detail loading
- logical play target generation

### Phase 3

Integrate Guard catalog ingestion into subscription refresh and deferred detail loading.

### Phase 4

Integrate Guard play target resolution into `PlaybackResolver`.

### Phase 5

Add storage/query regression coverage and real-network tests.

## Success Criteria

The project is successful when all of the following are true:

- Fantaihard's `文采` and `贱贱/荐片` produce non-zero `catalog_items`
- opening one of those items produces visible playable episode groups
- play resolution yields probed direct media candidates for at least one real item from each source
- the app remains pure Rust and does not require Node.js, JVM, jar spiders, or any external Guard host
- the runtime contract can accept a third `csp_*Guard` adapter without changing frontend APIs or database schema
