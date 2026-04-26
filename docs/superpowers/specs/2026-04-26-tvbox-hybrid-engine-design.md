# TVBox Hybrid Engine Design

## Overview

Replace the current hardcoded Rust scraper architecture with a hybrid Rust + JS engine that dynamically loads TVBox/CatVod shell configs. The system runs TVBox spider JS scripts via an embedded QuickJS runtime, enabling the app to support the entire TVBox source ecosystem without recompilation.

## Architecture

```
Frontend (Vue 3 / Pinia)
    │ invoke()
    ▼
Tauri Commands
    │
    ▼
ProviderRegistry
    ├── CmsProvider (type: 1)  ──→ reqwest (CMS JSON API)
    ├── SpiderProvider (type: 3) ──→ rquickjs + reqwest
    └── (future: ExternalProvider for drpy/jar)
            │
            ▼
    SQLite Storage (catalog cache + playback history)
```

## Key Components

### 1. VideoProvider Trait

```rust
#[async_trait]
pub trait VideoProvider: Send + Sync {
    fn source_key(&self) -> &str;
    fn source_name(&self) -> &str;

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;
    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError>;
    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;
    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;
    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError>;
    async fn play(&self, flag: &str, play_url: &str) -> Result<PlayerPlayResult, ProviderError>;
}
```

### 2. CmsProvider (type: 1)

Generic implementation for CMS/JSON API sources. Uses the ext field as the API base URL and follows TVBox CMS protocol:

- `{ext}?ac=list&t={type_id}&pg={page}` — category listing
- `{ext}?ac=detail&ids={vod_id}` — detail + episodes
- `{ext}?ac=videolist&wd={keyword}` — search

No site-specific Rust code needed. Handles all type:1 sources from any TVBox config.

### 3. SpiderProvider (type: 3)

Runs TVBox spider JS scripts via rquickjs. Each spider gets its own isolated JS Runtime.

**Runtime lifecycle:**
- Provider created during `refresh_subscription`
- JS Runtime created lazily on first method call
- JS script downloaded from spider URL (or read from ext if base64-encoded)
- Script cached in memory for app lifetime

**JS environment bindings:**
| TVBox API | Rust Implementation |
|-----------|-------------------|
| `req(url, options)` | Rust fn → tokio::runtime::Handle::current().block_on(reqwest) |
| `fetch(url, options)` | Same as req |
| `jars` | Rust CookieJar (per-spider isolation optional) |
| `base64.encode/decode` | Rust base64 crate |
| `input` | Injected search keyword from Rust |
| `MY_ARGS` | Parsed from SiteConfig.ext |
| `JSON.parse/stringify` | JS built-in |

**Async bridging:**
SpiderProvider uses a two-layer dispatch:
1. Outer async method (e.g., `search(wd)`) — Rust async
2. Inner `ctx.with()` block — synchronous JS execution
3. `req()` bindings use `Handle::current().block_on()` for HTTP calls

### 4. ProviderRegistry

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<Box<dyn VideoProvider>>>,
    site_configs: HashMap<String, TvboxSiteRecord>,
}
```

**Key methods:**
- `register_from_config(records: TvboxConfigRecords)` — batch register from parsed config
- `search_all(keyword)` — concurrent search across all searchable providers
- `get(key)` — get specific provider by site_key

### 5. Configuration Loading Flow

```
Subscription URL (e.g., 饭太硬)
    ↓ HTTP GET
TvboxConfigParser::parse()  (existing, enhanced)
    ↓
TvboxConfigRecords { sites, parses, lives }
    ↓
ProviderRegistry::register_from_config()
    ├── type: 1  → CmsProvider::new(api, ext)
    ├── type: 3  → SpiderProvider::new(key, name, ext)
    └── other    → skip
    ↓
Store to SQLite (sites, parses, lives tables)
```

## Command Changes

| Command | Change |
|---------|--------|
| `search_all_sources(keyword)` | NEW: Concurrent search across all providers |
| `get_catalog_items` | Route through ProviderRegistry instead of SQLite |
| `get_catalog_detail` | Route through Provider.detail() |
| `resolve_playback` | Try Provider.play() first, fallback to existing resolver |
| `refresh_subscription` | Replace `scrape_supported_tvbox_catalogs()` with `register_from_config()` |

## Frontend Impact

Minimal. Existing Pinia stores unchanged. Add a global search UI component that calls `search_all_sources`. All existing commands keep their signatures.

## JS Script Caching

- **Memory**: JS source kept in SpiderProvider struct for app lifetime
- **Disk**: Optional file/SQLite cache with URL-based invalidation
- **Refresh**: Re-downloaded on each `refresh_subscription` call if URL changed

## Error Handling

| Scenario | Action |
|----------|--------|
| JS parse error | Skip source, log warning |
| JS runtime error | Retry once, skip on failure |
| req() timeout | Per-request timeout (5s), continue others |
| Invalid return format | Return empty result |
| Spider URL unreachable | Mark source as unavailable, retry on next refresh |

## Migration Path

1. Add new `services/provider/` module with Trait, CmsProvider, SpiderProvider, Registry
2. Add `provider_registry` to `AppState`
3. Modify `refresh_subscription` to register providers instead of calling old scraper dispatch
4. Add `search_all_sources` command
5. Modify `get_catalog_items` and `get_catalog_detail` to use providers
6. Modify `resolve_playback` to use provider.play() with fallback
7. Remove old scraper modules (auete, libvio, zxzj, wencai, jianpian, xb6v)
8. Remove guard modules (guard, guard_jpj, guard_jpys) — spider JS handles this
9. Clean up unused dependencies and dead code

## Dependencies Added

- `rquickjs` with `futures` feature flag for async bridging support
- No new Java dependencies — JAR spiders remain unsupported

## Scope Boundaries

- **Only JS spider scripts are supported.** JAR files (Java spider bytecode) are explicitly out of scope — they require a JVM runtime which is not practical for Tauri desktop distribution.
- **Only type: 1 (CMS/JSON) and type: 3 (Spider JS) are supported.** Other TVBox source types (type: 0 = JSON configuration proxy, custom types) are skipped.
- **No JS sandbox escape protection.** rquickjs provides memory isolation but no security sandbox. The JS scripts are trusted (loaded from user-configured subscription URLs).
- **rquickjs version**: Use the `rquickjs` crate (wraps QuickJS C library via FFI), not `boa_engine` (pure Rust JS engine, slower startup).
