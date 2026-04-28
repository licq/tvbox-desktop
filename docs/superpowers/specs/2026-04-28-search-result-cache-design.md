# Search Result Caching Design

## Overview

Cache source search and Douban search results in SQLite with configurable TTL to reduce redundant HTTP scraping and improve perceived performance.

**Source search TTL**: 7 days (per `(source_key, keyword)` pair)
**Douban search TTL**: 30 days (per `keyword`)

## Cache Table Schema

Two new tables follow the existing `playback_health` TTL pattern (`checked_at` / `expires_at`):

```sql
CREATE TABLE IF NOT EXISTS source_search_cache (
    source_key TEXT NOT NULL,
    keyword TEXT NOT NULL,
    results_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    PRIMARY KEY (source_key, keyword)
);

CREATE TABLE IF NOT EXISTS douban_search_cache (
    keyword TEXT NOT NULL PRIMARY KEY,
    results_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);
```

- `created_at` and `expires_at` are Unix epoch seconds
- `source_search_cache.expires_at = created_at + 7 * 86400`
- `douban_search_cache.expires_at = created_at + 30 * 86400`

## Data Flow

### Source Search (`search_all_sources`)

Each provider is evaluated independently for cache hit/miss:

```
for each provider:
  1. check source_search_cache(source_key, keyword)
  2. if hit AND not expired:
       deserialize and return cached JSON
  3. if hit BUT expired:
       deserialize and return cached JSON immediately
       tokio::spawn background task to re-fetch and upsert cache
  4. if miss:
       fetch from provider in real time
       if non-empty, INSERT OR REPLACE into cache
```

This means:
- Adding a new source = natural cache miss for that source only
- Existing sources continue serving from cache
- Per-provider independence: one provider's refresh doesn't block others

### Douban Search (`search_douban_subject_by_keyword`)

```
1. check douban_search_cache(keyword)
2. if hit AND not expired:
     deserialize and return cached results
3. if hit BUT expired:
     return cached results immediately
     tokio::spawn background task to re-scrape and upsert cache
4. if miss:
     real-time WebView search for subject IDs
     scrape metadata for found subjects (existing behavior)
     if non-empty, INSERT OR REPLACE into cache
```

### Background Cache Pruning

On app startup and after each cache write, a lightweight cleanup runs:

```sql
DELETE FROM source_search_cache WHERE expires_at < unixepoch();
DELETE FROM douban_search_cache WHERE expires_at < unixepoch();
```

## Code Changes

### `src-tauri/src/services/storage.rs`

Add four new methods:

- `get_source_search_cache(&self, source_key: &str, keyword: &str) -> Result<Option<(String, bool)>>`
  Returns `(results_json, is_expired)` tuple. `None` if no cache entry exists.

- `set_source_search_cache(&self, source_key: &str, keyword: &str, results_json: &str) -> Result<()>`
  `INSERT OR REPLACE` with `created_at = now`, `expires_at = now + 7 days`.

- `get_douban_search_cache(&self, keyword: &str) -> Result<Option<(String, bool)>>`
  Same pattern but for douban.

- `set_douban_search_cache(&self, keyword: &str, results_json: &str) -> Result<()>`
  `INSERT OR REPLACE` with `expires_at = now + 30 days`.

- `prune_expired_search_caches(&self) -> Result<()>`
  Deletes expired rows from both tables.

All use `spawn_blocking` wrappers from callers (existing pattern).

### `src-tauri/src/commands/search.rs`

Modify `search_all_sources`:
- After getting the provider list, iterate with cache check logic
- Wrap each provider fetch in an async block that checks cache first
- Serialize provider results (`Vec<ScrapedCatalogItem>`) to JSON for storage
- Spawn background refresh on stale cache hit

### `src-tauri/src/commands/douban.rs`

Modify `search_douban_subject_by_keyword`:
- At the function entry point, check `douban_search_cache`
- On hit: deserialize and return
- On stale hit: deserialize and return + background spawn
- On miss: existing scrape logic
- On successful scrape: serialize results and store in cache

### `src-tauri/src/lib.rs`

In the app `setup` hook, add:

```rust
let storage = app_state.storage.clone();
tokio::spawn(async move {
    tokio::task::spawn_blocking(move || {
        storage.prune_expired_search_caches().ok();
    }).await.ok();
});
```

## Serialization

Cache stores JSON-serialized results:

- For source search: `HashMap<String, Vec<ScrapedCatalogItem>>` keyed by `source_key` — but since each cache entry is per `(source_key, keyword)`, it only stores one source's items
- For douban search: the result type used by `search_douban_subject_by_keyword`

`serde_json` is already a dependency — no new crate needed.

## Frontend Impact

**None.** The API signatures (`search_all_sources`, `search_douban_subject_by_keyword`) remain identical. Frontend continues calling them exactly as before. The only user-visible difference is faster response on cache hits.

## Edge Cases

- **Empty results**: Not cached. A zero-item result from a provider should not be cached, so the next search retries.
- **Provider error/timeout**: Not cached. Errors should not overwrite valid existing cache entries.
- **Keyword normalization**: Keywords are stored as-is from the frontend. No stemming or normalization. A search for "蜘蛛侠" and "蜘蛛侠 " (trailing space) are different cache keys. This matches existing behavior.
- **Concurrent searches**: If two requests for the same keyword arrive simultaneously, both may fetch. This is acceptable — the second write will `REPLACE` the first. No mutex needed.
- **Database locked**: SQLite busy errors are already handled by the existing storage layer.

## Estimated Scope

- ~60 lines: storage.rs (table creation + 5 new methods)
- ~80 lines: commands/search.rs (cache integration in search_all_sources)
- ~40 lines: commands/douban.rs (cache integration in search_douban_subject_by_keyword)
- ~10 lines: lib.rs (startup prune)
- **Total: ~190 lines of Rust**
