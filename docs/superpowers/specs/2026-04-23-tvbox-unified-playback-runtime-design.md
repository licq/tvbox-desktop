# TVBox Unified Playback Runtime Design

## Context

The current desktop app can ingest Fantaihard and other TVBox-style sources, but playback reliability is still driven by source-specific fixes. We have repeatedly patched individual providers such as `zxzj`, `auete`, `wencai`, `jianpian`, `libvio`, and direct HLS domains like `bfllvip`, `dytt-kan`, and `fengbao9`.

That approach has two structural problems:

1. Playback logic is fragmented across source parsers and the shared resolver.
2. The user still sees half-resolved or dead lines because there is no single runtime that decides whether a line is actually playable in the desktop app.

The goal of this project is to raise real playback success rate, not to keep accumulating domain-specific fixes.

## Goals

- Maximize real playback success rate for TVBox/Fantaihard VOD sources.
- Stop exposing half-resolved, embedded-only, or obviously dead lines to the frontend.
- Add short-lived health caching so the app does not re-probe the same failing line on every click.
- Preserve currently working direct/resolvable sources while moving them into a unified runtime.

## Non-Goals

- Full TVBox spider or `drpy` runtime compatibility in this phase.
- Solving every possible TVBox source in one pass.
- Reworking the frontend information architecture beyond what is needed for playback behavior.

## Current Problems

### Fragmented resolution

`PlaybackResolver` currently mixes:

- protocol classification
- source-specific page parsing
- media probing
- filtering rules

Each new source adds more branching, and the same playback policy is reimplemented in several places.

### Unreliable visibility decisions

Some lines are filtered too late:

- dead direct HLS links still leak through until a player error occurs
- manifest-level failures and browser CORS failures are not handled uniformly
- some sources return page shells or embedded players that the desktop app cannot really use

### No runtime memory

The app forgets whether a line was just verified or just failed. That creates:

- repeated upstream requests for the same target
- repeated user exposure to the same bad lines
- unnecessary latency when reopening a detail page

## Proposed Approach

Introduce a unified playback runtime that turns all episode lines into standardized playback targets, resolves them through a shared pipeline, probes them with browser-aware checks, ranks them, caches the result, and only then returns playable candidates to the frontend.

This is a runtime-level refactor, not a UI-only change and not a full TVBox reimplementation.

## Architecture

### 1. Playback Target Model

Add a normalized internal model:

- `PlaybackTarget`
  - `episode_id`
  - `source_key`
  - `target_url`
  - `target_kind`
  - `resolver_key`
  - `headers`
  - `sort_hint`
  - `meta`

`target_kind` is the raw target class before filtering:

- `direct`
- `resolvable`
- `embedded`
- `external_required`

This model is internal to the runtime. The frontend continues to consume `ResolvedPlayback`.

### 2. Unified Runtime Pipeline

The runtime executes six stages:

1. `discover`
   - collect raw episode targets from `catalog_episodes` or a detail refresh
   - standardize them into `PlaybackTarget`

2. `resolve`
   - turn `guard://`, source play pages, and intermediate parser targets into direct or near-direct candidates

3. `probe`
   - validate candidate playability with browser-aware checks

4. `rank`
   - score candidates so the best likely line is first

5. `cache`
   - store health outcomes with TTL

6. `present`
   - return only candidates that pass the configured playback threshold

### 3. Source Adapter Contract

Source adapters stop owning the full playback lifecycle.

Each source implementation should only provide:

- `discover_targets`
- `resolve_target`

Everything else belongs to the runtime:

- health probing
- candidate filtering
- ranking
- caching
- final presentation decisions

### 4. Health Cache

Add a short-lived runtime cache for candidate health.

#### `playback_health`

- `target_hash`
- `status`
- `manifest_ok`
- `segment_ok`
- `cors_ok`
- `http_status`
- `failure_reason`
- `checked_at`
- `expires_at`

Recommended TTL policy:

- successful candidate: `30-90` minutes
- explicit failure: `5-15` minutes
- unknown or partial result: `5` minutes

This cache is keyed by normalized target URL plus effective headers and resolver identity.

### 5. Persisted Target Table

Phase 1 will add a persisted target table for reuse, diagnostics, and deduplication.

#### `playback_targets`

- `episode_id`
- `source_key`
- `target_url`
- `target_kind`
- `resolver_key`
- `headers_json`
- `sort_hint`
- `created_at`
- `updated_at`

This table is part of phase 1 so the runtime has a stable place to reuse normalized targets across repeated detail opens and playback attempts.

## Probe Policy

The probe stage must reflect whether a desktop browser player can actually use the line, not merely whether the backend can fetch bytes.

### Direct HLS

Checks:

- manifest request succeeds
- manifest has `#EXTM3U`
- if master playlist, a variant playlist is reachable
- media playlist has at least one segment
- encryption key, if present, is reachable
- first segment is reachable
- manifest and segment responses satisfy browser CORS expectations

### Direct HTTP video

Checks:

- range request succeeds
- response status is acceptable
- content is not obviously an HTML error shell
- browser-use constraints are satisfied when relevant

### Resolvable targets

Checks:

- source-specific resolver returns candidate media targets
- returned targets re-enter the same direct probe pipeline

### Embedded and external targets

- `embedded` does not count as desktop-playable in this phase
- `external_required` does not count as desktop-playable in this phase
- neither should appear in the playable candidate list

## Ranking Policy

Ranking is explicit and shared across all sources:

1. direct HLS or direct media with successful probe
2. resolvable targets that produce direct playable media
3. anything else is hidden

Additional ranking inputs:

- previous success in health cache
- lower redirect depth
- fewer required headers
- lower resolver complexity

This keeps stable direct lines ahead of fragile multi-hop lines.

## Frontend Behavior

The frontend will continue to call `resolve_playback`, but the command semantics change.

### `resolve_playback`

The command becomes the orchestrator for:

- target discovery
- cache lookup
- resolution
- probing
- ranking
- filtering

Returned candidates must already be safe for desktop playback.

### Detail Page

- only show lines that passed the runtime threshold
- do not show `embedded`
- do not show `external_required`
- if no line passes, show a single clear state:
  - `当前集未找到通过探测的可播线路`

### Player Page

- attempt the top-ranked candidate first
- on fatal failure, only switch within the already-approved result set
- do not attempt hidden or raw source lines directly

## Error Handling and Diagnostics

The runtime must preserve enough diagnostics for engineering without exposing noisy internals to end users.

Internal diagnostics should track:

- failing stage: `discover`, `resolve`, `probe`
- failing URL
- effective headers
- HTTP status
- CORS failure
- manifest failure
- segment failure

User-facing output should remain compact:

- `当前集未找到通过探测的可播线路`
- `当前直链不可播放`
- `播放页未提取到有效媒体地址`

## Migration Plan

### Phase 1: Runtime Extraction

- introduce `PlaybackTarget`
- extract shared `resolve/probe/rank/cache` pipeline
- route existing sources through the unified runtime
- keep current sources working:
  - `guard`
  - `wencai`
  - `jianpian`
  - `libvio`
  - `auete`

Primary success criterion:

- dead direct HLS and dead manifest lines stop appearing in the playable list

### Phase 2: Adapter Normalization

- move source-specific playback behavior behind adapter interfaces
- remove duplicate probe logic from source branches
- centralize ranking and headers policy

Primary success criterion:

- adding a new source only requires candidate discovery and page-to-media resolution logic

### Phase 3: Future TVBox Expansion

- add broader Guard coverage
- evaluate whether heavier spider/drpy compatibility is worth adding later

This phase is intentionally outside the immediate implementation scope.

## Testing Strategy

### Unit Tests

- target classification
- ranking rules
- health cache TTL behavior
- probe failure classification
- source adapter contract compliance

### Integration Tests

- `resolve_playback` returns only playable candidates
- `embedded`-only titles stay hidden
- direct dead HLS links are filtered
- valid resolvable targets survive and rank ahead of weaker candidates

### Live Network Tests

Keep a small ignored suite for representative real sources:

- one working Guard-based Wencai title
- one working Guard-based Jianpian title
- one working direct HLS title
- one dead direct HLS title

These tests are validation aids, not the main correctness mechanism.

## Risks

### Probe strictness may hide salvageable lines

Mitigation:

- keep probe stages separately visible in diagnostics
- tune thresholds based on observed working lines
- prefer false negative over repeatedly surfacing obviously broken lines in phase 1

### Runtime latency may increase

Mitigation:

- add short TTL health caching
- reuse cached success aggressively
- cap synchronous probe fanout per request

### Source-specific regressions during migration

Mitigation:

- migrate current working sources first
- keep regression tests for known working sample titles
- only remove old branch logic after the runtime path has equivalent coverage

## Success Criteria

The redesign is successful when:

- dead direct HLS and dead manifest lines no longer appear in the UI after refresh
- frontend only receives desktop-playable candidates
- repeated opens of the same episode avoid redundant upstream probing
- currently working `guard/wencai/jianpian/libvio/auete` lines do not regress materially
- new source onboarding shifts from ad hoc player fixes to adapter implementation
