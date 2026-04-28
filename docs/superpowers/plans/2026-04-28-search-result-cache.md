# Search Result Caching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cache source search results (per `source_key`+`keyword`) and Douban search results (per `keyword`) in SQLite with configurable TTL to eliminate redundant HTTP scraping.

**Architecture:** SQLite tables for cache storage with `created_at`/`expires_at` TTL pattern (matching existing `playback_health` table). Cache logic in storage layer + command handlers. Background stale refresh via `tokio::spawn`.

**Tech Stack:** Rust, rusqlite, serde_json, tokio

---

## File Structure

| File | Responsibility |
|---|---|
| `src-tauri/src/services/storage.rs` | Add 2 cache tables + 5 cache methods |
| `src-tauri/src/services/provider/registry.rs` | Add `all_provider_pairs()` getter |
| `src-tauri/src/commands/search.rs` | Integrate cache into `search_all_sources` |
| `src-tauri/src/commands/douban.rs` | Integrate cache into `search_douban_subject_by_keyword` |
| `src-tauri/src/main.rs` | Add startup cache pruning task |

### Task 1: Add cache tables and storage methods

**Files:**
- Modify: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: Add table creation in `init_tables()`**

After the `douban_subject_meta` table creation (around line 354, before `Ok(())`), add:

```rust
conn.execute(
    "CREATE TABLE IF NOT EXISTS source_search_cache (
        source_key TEXT NOT NULL,
        keyword TEXT NOT NULL,
        results_json TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL,
        PRIMARY KEY (source_key, keyword)
    )",
    [],
)?;

conn.execute(
    "CREATE TABLE IF NOT EXISTS douban_search_cache (
        keyword TEXT NOT NULL PRIMARY KEY,
        results_json TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        expires_at INTEGER NOT NULL
    )",
    [],
)?;
```

Add after `use` imports, a helper for current unix timestamp:

```rust
fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
```

(Check if `now_epoch()` already exists — if not, add it as a private helper function in the `Storage` impl.)

- [ ] **Step 2: Add `get_source_search_cache()` method**

Add after `init_tables()` (before `update_subscription_refresh_state`):

```rust
pub fn get_source_search_cache(&self, source_key: &str, keyword: &str) -> SqliteResult<Option<(String, bool)>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT results_json, expires_at FROM source_search_cache WHERE source_key = ?1 AND keyword = ?2"
    )?;
    let mut rows = stmt.query(rusqlite::params![source_key, keyword])?;
    match rows.next()? {
        Some(row) => {
            let results_json: String = row.get(0)?;
            let expires_at: i64 = row.get(1)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let expired = now > expires_at;
            Ok(Some((results_json, expired)))
        }
        None => Ok(None),
    }
}
```

- [ ] **Step 3: Add `set_source_search_cache()` method**

```rust
pub fn set_source_search_cache(&self, source_key: &str, keyword: &str, results_json: &str) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let expires_at = now + 7 * 86400; // 7 days
    conn.execute(
        "INSERT OR REPLACE INTO source_search_cache (source_key, keyword, results_json, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![source_key, keyword, results_json, now, expires_at],
    )?;
    Ok(())
}
```

- [ ] **Step 4: Add `get_douban_search_cache()` method**

```rust
pub fn get_douban_search_cache(&self, keyword: &str) -> SqliteResult<Option<(String, bool)>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT results_json, expires_at FROM douban_search_cache WHERE keyword = ?1"
    )?;
    let mut rows = stmt.query(rusqlite::params![keyword])?;
    match rows.next()? {
        Some(row) => {
            let results_json: String = row.get(0)?;
            let expires_at: i64 = row.get(1)?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let expired = now > expires_at;
            Ok(Some((results_json, expired)))
        }
        None => Ok(None),
    }
}
```

- [ ] **Step 5: Add `set_douban_search_cache()` method**

```rust
pub fn set_douban_search_cache(&self, keyword: &str, results_json: &str) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let expires_at = now + 30 * 86400; // 30 days
    conn.execute(
        "INSERT OR REPLACE INTO douban_search_cache (keyword, results_json, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![keyword, results_json, now, expires_at],
    )?;
    Ok(())
}
```

- [ ] **Step 6: Add `prune_expired_search_caches()` method**

```rust
pub fn prune_expired_search_caches(&self) -> SqliteResult<()> {
    let conn = self.conn.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.execute("DELETE FROM source_search_cache WHERE expires_at < ?1", rusqlite::params![now])?;
    conn.execute("DELETE FROM douban_search_cache WHERE expires_at < ?1", rusqlite::params![now])?;
    Ok(())
}
```

- [ ] **Step 7: Build check**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```
Expected: Compilation succeeds.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/services/storage.rs
git commit -m "feat(storage): add search result cache tables and methods"
```

### Task 2: Add provider pairs accessor to registry

**Files:**
- Modify: `src-tauri/src/services/provider/registry.rs`

- [ ] **Step 1: Add `all_provider_pairs()` method**

After the existing `searchable_providers()` method (around line 80):

```rust
pub fn all_provider_pairs(&self) -> Vec<(String, Arc<Box<dyn VideoProvider>>)> {
    self.providers.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}
```

- [ ] **Step 2: Build check**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/provider/registry.rs
git commit -m "feat(registry): add all_provider_pairs accessor"
```

### Task 3: Integrate cache into `search_all_sources`

**Files:**
- Modify: `src-tauri/src/commands/search.rs`

This is the main change. Rewrite `search_all_sources` to do per-provider cache checking.

- [ ] **Step 1: Rewrite `search_all_sources` command**

Replace the entire `search_all_sources` function (lines 14-29):

```rust
use crate::services::xb6v::ScrapedCatalogItem;

#[tauri::command]
pub async fn search_all_sources(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<SourceSearchResult>, String> {
    log::info!("[search_all_sources] Command called with keyword: {}", keyword);
    let registry = state.provider_registry.read().await;
    let pairs = registry.all_provider_pairs();
    let storage = state.storage.clone();
    log::info!("[search_all_sources] Registry acquired, providers count: {}", pairs.len());

    let mut handles = Vec::new();
    for (source_key, provider) in pairs {
        let storage = storage.clone();
        let kw = keyword.clone();
        let sk = source_key.clone();

        handles.push(tokio::spawn(async move {
            // Check cache first
            match storage.get_source_search_cache(&sk, &kw) {
                Ok(Some((cached_json, expired))) => {
                    // Deserialize cached results
                    match serde_json::from_str::<Vec<ScrapedCatalogItem>>(&cached_json) {
                        Ok(items) => {
                            if expired && !items.is_empty() {
                                // Background refresh: spawn fire-and-forget task
                                let storage = storage.clone();
                                let provider = provider.clone();
                                let kw = kw.clone();
                                let sk = sk.clone();
                                tokio::spawn(async move {
                                    log::info!("[search_all_sources] Background refresh for {}/{}", sk, kw);
                                    match provider.search(&kw).await {
                                        Ok(new_items) if !new_items.is_empty() => {
                                            if let Ok(json) = serde_json::to_string(&new_items) {
                                                let _ = storage.set_source_search_cache(&sk, &kw, &json);
                                            }
                                        }
                                        _ => {}
                                    }
                                });
                            }
                            let name = provider.source_name().to_string();
                            return Some(SourceSearchResult { source_key: sk, source_name: name, items });
                        }
                        Err(e) => {
                            log::warn!("[search_all_sources] Cache deserialize failed for {}: {}", sk, e);
                            // Fall through to real fetch
                        }
                    }
                }
                Ok(None) => {} // No cache, fall through to real fetch
                Err(e) => {
                    log::warn!("[search_all_sources] Cache check failed for {}: {}", sk, e);
                }
            }

            // No valid cache: fetch from provider in real time
            match provider.search(&kw).await {
                Ok(items) => {
                    let name = provider.source_name().to_string();
                    if !items.is_empty() {
                        // Cache result if non-empty
                        if let Ok(json) = serde_json::to_string(&items) {
                            let _ = storage.set_source_search_cache(&sk, &kw, &json);
                        }
                    }
                    Some(SourceSearchResult { source_key: sk, source_name: name, items })
                }
                Err(_) => None,
            }
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        if let Some(result) = handle.await.unwrap_or(None) {
            results.push(result);
        }
    }

    log::info!("[search_all_sources] Returning {} results", results.len());
    Ok(results)
}
```

- [ ] **Step 2: Build check**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/search.rs
git commit -m "feat(search): cache source search results per provider+keyword"
```

### Task 4: Integrate cache into `search_douban_subject_by_keyword`

**Files:**
- Modify: `src-tauri/src/commands/douban.rs`

The function `search_douban_subject_by_keyword` (line 358-434) currently returns `Result<Option<DoubanSubjectMeta>, String>`. The cache should store the serialized `DoubanSubjectMeta` directly.

- [ ] **Step 1: Add cache check at the start of `search_douban_subject_by_keyword`**

Replace the function body to add cache check at the very top, before the hot list scan. The existing hot list scan logic becomes the fallback when cache is missing or empty.

The modified function:

```rust
#[tauri::command(rename_all = "snake_case")]
pub async fn search_douban_subject_by_keyword(
    app: AppHandle,
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Option<DoubanSubjectMeta>, String> {
    // Cache check: if we have a non-expired cached result, return it immediately
    match state.storage.get_douban_search_cache(&keyword) {
        Ok(Some((cached_json, expired))) => {
            match serde_json::from_str::<DoubanSubjectMeta>(&cached_json) {
                Ok(meta) => {
                    if expired {
                        // Background refresh
                        let app = app.clone();
                        let kw = keyword.clone();
                        let storage = state.storage.clone();
                        tokio::spawn(async move {
                            log::info!("[search_douban] Background refresh for keyword: {}", kw);
                            refresh_douban_search_cache(&app, &kw, &storage).await;
                        });
                    }
                    return Ok(Some(meta));
                }
                Err(e) => {
                    log::warn!("[search_douban] Cache deserialize failed: {}", e);
                }
            }
        }
        Ok(None) => {}
        Err(e) => {
            log::warn!("[search_douban] Cache check failed: {}", e);
        }
    }

    // Step 1: Try DB hot list first (fast path, no WebView needed)
    // ... existing hot list scan code ...
    {
        let douban_items = state.storage.get_douban_hot().map_err(|e| e.to_string())?;
        let normalized = keyword.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase();

        for item in douban_items.iter().take(500) {
            let item_normalized = item.name
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .collect::<String>()
                .to_lowercase();

            let score = calculate_similarity(&normalized, &item_normalized);
            if score > 0.8 {
                // Found in hot list, check cache then scrape
                if let Ok(Some(cached)) = state.storage.get_douban_subject_meta(item.id) {
                    log::info!("[search_douban_subject_by_keyword] Cache hit for douban_id={}", item.id);
                    // Cache the result for keyword
                    if let Ok(json) = serde_json::to_string(&cached) {
                        let _ = state.storage.set_douban_search_cache(&keyword, &json);
                    }
                    return Ok(Some(cached));
                }
                log::info!("[search_douban_subject_by_keyword] Found in hot list, scraping douban_id={}", item.id);
                let meta = DoubanSubjectScraper::scrape(&app, item.id).await;
                match meta {
                    Ok(m) => {
                        let _ = state.storage.upsert_douban_subject_meta(&m);
                        if let Ok(json) = serde_json::to_string(&m) {
                            let _ = state.storage.set_douban_search_cache(&keyword, &json);
                        }
                        return Ok(Some(m));
                    }
                    Err(e) => log::warn!("Failed to scrape hot list item {}: {}", item.id, e),
                }
            }
        }
    }

    // Step 2: Search Douban via WebView (handles anti-scraping JS challenges)
    log::info!("[search_douban_subject_by_keyword] Using WebView to search Douban for keyword: {}", keyword);
    let found_ids = match DoubanSubjectScraper::search_subject_ids(&app, &keyword).await {
        Ok(ids) => ids,
        Err(e) => {
            log::warn!("[search_douban_subject_by_keyword] WebView search failed: {}", e);
            Vec::new()
        }
    };

    // Try each found id (check cache first, then scrape)
    for douban_id in found_ids {
        log::info!("[search_douban_subject_by_keyword] Found douban_id={} via WebView search", douban_id);

        // Check cache
        if let Ok(Some(cached)) = state.storage.get_douban_subject_meta(douban_id) {
            log::info!("[search_douban_subject_by_keyword] Cache hit for douban_id={}", douban_id);
            if let Ok(json) = serde_json::to_string(&cached) {
                let _ = state.storage.set_douban_search_cache(&keyword, &json);
            }
            return Ok(Some(cached));
        }

        // Scrape for rich metadata
        match DoubanSubjectScraper::scrape(&app, douban_id).await {
            Ok(m) => {
                let _ = state.storage.upsert_douban_subject_meta(&m);
                if let Ok(json) = serde_json::to_string(&m) {
                    let _ = state.storage.set_douban_search_cache(&keyword, &json);
                }
                return Ok(Some(m));
            }
            Err(e) => {
                log::warn!("Scrape failed for douban_id {}: {}", douban_id, e);
                continue;
            }
        }
    }

    log::info!("[search_douban_subject_by_keyword] No matching Douban subject found for keyword: {}", keyword);
    Ok(None)
}

/// Helper: refresh douban search cache by re-scraping via WebView
async fn refresh_douban_search_cache(app: &AppHandle, keyword: &str, storage: &crate::services::Storage) {
    let found_ids = match DoubanSubjectScraper::search_subject_ids(app, keyword).await {
        Ok(ids) => ids,
        Err(e) => {
            log::warn!("[refresh_douban_search_cache] WebView search failed: {}", e);
            return;
        }
    };

    for douban_id in found_ids {
        // Check subject meta cache first
        if let Ok(Some(cached)) = storage.get_douban_subject_meta(douban_id) {
            if let Ok(json) = serde_json::to_string(&cached) {
                let _ = storage.set_douban_search_cache(keyword, &json);
            }
            return;
        }

        match DoubanSubjectScraper::scrape(app, douban_id).await {
            Ok(m) => {
                let _ = storage.upsert_douban_subject_meta(&m);
                if let Ok(json) = serde_json::to_string(&m) {
                    let _ = storage.set_douban_search_cache(keyword, &json);
                }
                return;
            }
            Err(e) => {
                log::warn!("[refresh_douban_search_cache] Scrape failed for {}: {}", douban_id, e);
                continue;
            }
        }
    }
}
```

- [ ] **Step 2: Build check**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/commands/douban.rs
git commit -m "feat(douban): cache douban search results per keyword"
```

### Task 5: Add startup cache pruning

**Files:**
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add cache pruning in the `.setup()` closure**

After `app.manage(...)` (line 49), before the `log::info!`:

```rust
// Prune expired search caches on startup
let storage = storage.clone();
tokio::spawn(async move {
    tokio::task::spawn_blocking(move || {
        storage.prune_expired_search_caches().ok();
    }).await.ok();
});
```

- [ ] **Step 2: Build check**

```bash
cd src-tauri && cargo check 2>&1 | head -30
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: prune expired search caches on startup"
```

### Task 6: Integration test

- [ ] **Step 1: Full build and verify**

```bash
cd src-tauri && cargo check 2>&1
```
Expected: No errors.

- [ ] **Step 2: Commit any remaining changes**

```bash
git add -A && git commit -m "chore: final cleanup after search cache implementation"
```
