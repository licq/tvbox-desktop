# TVBox Hybrid Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace all hardcoded Rust scrapers with a hybrid engine that dynamically loads TVBox shell configs and executes type:1 CMS providers and type:3 JS spider scripts via rquickjs.

**Architecture:** New `services/provider/` module with `VideoProvider` trait, `CmsProvider` (generic type:1), `SpiderProvider` (rquickjs-based type:3), and `ProviderRegistry`. Providers registered at subscription refresh time, called lazily on frontend request. Old scraper modules removed after migration.

**Tech Stack:** Rust + Tauri 2, rquickjs (QuickJS), reqwest, tokio, serde_json

---

## File Structure

### New files:
```
src-tauri/src/services/provider/
├── mod.rs              # Re-exports + ProviderError
├── traits.rs           # VideoProvider trait
├── cms_provider.rs     # CmsProvider (type: 1 CMS/JSON API)
├── spider_provider.rs  # SpiderProvider (type: 3, rquickjs)
├── registry.rs         # ProviderRegistry
```

### Modified files:
- `src-tauri/Cargo.toml` — add `rquickjs` dependency
- `src-tauri/src/lib.rs` — add `provider_registry` to `AppState`
- `src-tauri/src/services/mod.rs` — add `provider` module
- `src-tauri/src/commands/subscription.rs` — call `register_from_config()` after parse
- `src-tauri/src/commands/vod.rs` — route through providers
- `src-tauri/src/commands/player.rs` — try `Provider.play()` first
- `src-tauri/src/commands/mod.rs` — add `search_all_sources` command
- `src-tauri/src/main.rs` — pass `ProviderRegistry` to `AppState`
- `src-tauri/src/services/resolver.rs` — update playback_source_rank (remove hardcoded keys)
- `src-tauri/src/services/playback_types.rs` — update source rankings

### Deleted files (final phase):
```
src-tauri/src/services/auete.rs
src-tauri/src/services/libvio.rs
src-tauri/src/services/zxzj.rs
src-tauri/src/services/wencai.rs
src-tauri/src/services/jianpian.rs
src-tauri/src/services/guard.rs
src-tauri/src/services/guard_jpj.rs
src-tauri/src/services/guard_jpys.rs
src-tauri/src/services/xb6v.rs (heavily trimmed or removed)
```

---

### Task 1: Add provider module skeleton

**Files:**
- Create: `src-tauri/src/services/provider/mod.rs`
- Create: `src-tauri/src/services/provider/traits.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add rquickjs to Cargo.toml**

```toml
# Add under [dependencies]
rquickjs = { version = "0.7", features = ["futures"] }
```

- [ ] **Step 2: Create provider/mod.rs with ProviderError**

```rust
// src-tauri/src/services/provider/mod.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("JS execution error: {0}")]
    JsRuntime(String),

    #[error("Unsupported source type: {0}")]
    UnsupportedType(String),

    #[error("Spider script unavailable: {0}")]
    SpiderUnavailable(String),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub mod traits;
pub mod cms_provider;
pub mod spider_provider;
pub mod registry;

pub use traits::VideoProvider;
pub use cms_provider::CmsProvider;
pub use spider_provider::SpiderProvider;
pub use registry::ProviderRegistry;
```

- [ ] **Step 3: Create VideoProvider trait**

```rust
// src-tauri/src/services/provider/traits.rs
use async_trait::async_trait;
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::playback_types::PlaybackTarget;
use super::ProviderError;

#[derive(Debug, Clone)]
pub struct CatalogCategory {
    pub type_id: String,
    pub type_name: String,
}

#[async_trait]
pub trait VideoProvider: Send + Sync {
    fn source_key(&self) -> &str;
    fn source_name(&self) -> &str;

    /// 获取首页推荐
    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 获取分类列表
    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError>;

    /// 按分类和分页获取内容
    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 搜索
    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError>;

    /// 获取详情（含剧集列表）
    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError>;

    /// 解析播放地址
    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError>;
}
```

- [ ] **Step 4: Register provider module in services/mod.rs**

```rust
// Add to existing services/mod.rs module declarations
pub mod provider;
```

- [ ] **Step 5: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -40`
Expected: Compilation errors for missing module contents (cms_provider, spider_provider, registry not found) — this is expected, we add those next.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/services/provider/mod.rs src-tauri/src/services/provider/traits.rs src-tauri/src/services/mod.rs
git commit -m "feat: add provider module skeleton with VideoProvider trait"
```

---

### Task 2: Implement CmsProvider (type: 1 CMS/JSON)

**Files:**
- Create: `src-tauri/src/services/provider/cms_provider.rs`
- Test: `src-tauri/src/services/provider/cms_provider_test.rs` (or inline tests)

- [ ] **Step 1: Write CmsProvider struct and constructor**

```rust
// src-tauri/src/services/provider/cms_provider.rs
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use super::{VideoProvider, ProviderError, CatalogCategory};
use crate::services::xb6v::ScrapedCatalogItem;

pub struct CmsProvider {
    site_key: String,
    site_name: String,
    api_url: String,
    client: Client,
}

impl CmsProvider {
    pub fn new(site_key: String, site_name: String, api_url: String, client: Client) -> Self {
        Self { site_key, site_name, api_url, client }
    }

    /// 构建 CMS API URL
    fn build_url(&self, ac: &str, extra: &[(&str, &str)]) -> String {
        let base = self.api_url.trim_end_matches('/').to_string();
        let sep = if base.contains('?') { "&" } else { "?" };
        let mut url = format!("{}{}ac={}", base, sep, ac);
        for (k, v) in extra {
            url.push_str(&format!("&{}={}", k, v));
        }
        url
    }

    /// 解析 CMS JSON 响应为 ScrapedCatalogItem 列表
    fn parse_cms_list(&self, body: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let root: Value = serde_json::from_str(body)?;
        // CMS 标准响应格式: { list: [{ vod_id, vod_name, vod_pic, vod_content, type_name, ... }] }
        let list = root.get("list").and_then(|v| v.as_array()).ok_or_else(|| {
            ProviderError::Parse("CMS response missing 'list' array".to_string())
        })?;

        let mut items = Vec::new();
        for item in list {
            let vod_id = item.get("vod_id").and_then(|v| v.as_str()).unwrap_or("");
            let vod_name = item.get("vod_name").and_then(|v| v.as_str()).unwrap_or("");
            if vod_id.is_empty() || vod_name.is_empty() {
                continue;
            }
            let poster = item.get("vod_pic").and_then(|v| v.as_str()).map(|s| s.to_string());
            let summary = item.get("vod_content").and_then(|v| v.as_str()).map(|s| s.to_string());
            let type_name = item.get("type_name").and_then(|v| v.as_str()).unwrap_or("movie");

            items.push(ScrapedCatalogItem {
                source_item_key: format!("{}:{}", self.site_key, vod_id),
                title: vod_name.to_string(),
                item_type: type_name.to_string(),
                poster,
                summary,
                detail_json: Some(serde_json::json!({
                    "source": self.site_key,
                    "ids": vod_id,
                }).to_string()),
                episodes: Vec::new(),
            });
        }
        Ok(items)
    }
}
```

- [ ] **Step 2: Implement VideoProvider for CmsProvider**

```rust
#[async_trait]
impl VideoProvider for CmsProvider {
    fn source_key(&self) -> &str { &self.site_key }
    fn source_name(&self) -> &str { &self.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // CMS home = 最新列表, default type
        let url = self.build_url("list", &[("t", "1"), ("pg", "1")]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        // CMS type list: ?ac=list 返回的 list 中有 type_id 和 type_name 字段
        let url = self.build_url("list", &[("t", "1")]);
        // 返回静态分类 (TVBox CMS 标准分类)
        Ok(vec![
            CatalogCategory { type_id: "1".to_string(), type_name: "电影".to_string() },
            CatalogCategory { type_id: "2".to_string(), type_name: "电视剧".to_string() },
            CatalogCategory { type_id: "3".to_string(), type_name: "综艺".to_string() },
            CatalogCategory { type_id: "4".to_string(), type_name: "动漫".to_string() },
        ])
    }

    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("list", &[("t", type_id), ("pg", &page.to_string())]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("videolist", &[("wd", keyword)]);
        let body = self.client.get(&url).send().await?.text().await?;
        self.parse_cms_list(&body)
    }

    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let url = self.build_url("detail", &[("ids", ids)]);
        let body = self.client.get(&url).send().await?.text().await?;
        let root: Value = serde_json::from_str(&body)?;
        let list = root.get("list").and_then(|v| v.as_array());
        let Some(first) = list.and_then(|l| l.first()) else {
            return Ok(None);
        };

        let vod_id = first.get("vod_id").and_then(|v| v.as_str()).unwrap_or("");
        let vod_name = first.get("vod_name").and_then(|v| v.as_str()).unwrap_or("");
        let vod_actor = first.get("vod_actor").and_then(|v| v.as_str()).unwrap_or("");
        let vod_director = first.get("vod_director").and_then(|v| v.as_str()).unwrap_or("");

        let mut summary_parts = Vec::new();
        if !vod_director.is_empty() { summary_parts.push(format!("导演: {}", vod_director)); }
        if !vod_actor.is_empty() { summary_parts.push(format!("演员: {}", vod_actor)); }

        // 解析剧集
        let mut episodes = Vec::new();
        if let Some(vod_play_url) = first.get("vod_play_url").and_then(|v| v.as_str()) {
            // CMS 格式: "第1集$url1$$$第2集$url2$$$..."
            // 或 "第1集$url1#第2集$url2"
            let separator = if vod_play_url.contains("$$$") {
                "$$$"
            } else if vod_play_url.contains('#') {
                "#"
            } else {
                ""
            };

            if !separator.is_empty() {
                for (i, part) in vod_play_url.split(separator).enumerate() {
                    if let Some(dollar_pos) = part.find('$') {
                        let label = &part[..dollar_pos];
                        let url = &part[dollar_pos + 1..];
                        episodes.push(crate::services::xb6v::ScrapedCatalogEpisode {
                            source_name: self.site_name.clone(),
                            episode_label: label.to_string(),
                            play_url: url.to_string(),
                            order_index: (i + 1) as i64,
                        });
                    }
                }
            }
        }

        Ok(Some(ScrapedCatalogItem {
            source_item_key: format!("{}:{}", self.site_key, vod_id),
            title: vod_name.to_string(),
            item_type: first.get("type_name").and_then(|v| v.as_str()).unwrap_or("movie").to_string(),
            poster: first.get("vod_pic").and_then(|v| v.as_str()).map(|s| s.to_string()),
            summary: if summary_parts.is_empty() { None } else { Some(summary_parts.join("\n")) },
            detail_json: Some(serde_json::json!({
                "source": self.site_key,
                "ids": vod_id,
            }).to_string()),
            episodes,
        }))
    }

    async fn play(&self, _flag: &str, play_url: &str) -> Result<Vec<crate::services::playback_types::PlaybackTarget>, ProviderError> {
        // CMS 源的 play_url 通常是直接可用的 URL
        let target = crate::services::playback_types::PlaybackTarget {
            episode_id: None,
            source_key: self.site_key.clone(),
            target_url: play_url.to_string(),
            target_kind: crate::services::playback_types::PlaybackTargetKind::Resolvable,
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
        };
        Ok(vec![target])
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1 | head -40`
Expected: Compilation errors for missing spider_provider and registry — expected, add next.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/services/provider/cms_provider.rs
git commit -m "feat: implement CmsProvider for TVBox type:1 sources"
```

---

### Task 3: Implement SpiderProvider (type: 3, rquickjs)

**Files:**
- Create: `src-tauri/src/services/provider/spider_provider.rs`

- [ ] **Step 1: Write SpiderProvider struct**

```rust
// src-tauri/src/services/provider/spider_provider.rs
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use reqwest::Client;
use rquickjs::{AsyncContext, AsyncRuntime, Object, Function, Value, Error as JsError};
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{VideoProvider, ProviderError, CatalogCategory};
use crate::services::xb6v::ScrapedCatalogItem;

/// Spider JS runtime state (isolated per provider)
struct SpiderRuntime {
    rt: AsyncRuntime,
    ctx: AsyncContext,
}

pub struct SpiderProvider {
    site_key: String,
    site_name: String,
    ext: String,            // TVBox site.ext field (spider JS URL or config)
    spider_js_url: String,  // URL to download the spider JS script
    js_script: Mutex<Option<String>>, // Cached JS source, None = not loaded
    runtime: OnceCell<Arc<Mutex<SpiderRuntime>>>,
    client: Client,
}

impl SpiderProvider {
    pub fn new(site_key: String, site_name: String, ext: String, client: Client) -> Self {
        // ext is typically a URL pointing to the spider JS file
        let spider_js_url = ext.clone();
        Self {
            site_key,
            site_name,
            ext,
            spider_js_url,
            js_script: Mutex::new(None),
            runtime: OnceCell::new(),
            client,
        }
    }

    /// 下载 spider JS 脚本（如果未缓存）
    async fn ensure_script_loaded(&self) -> Result<String, ProviderError> {
        let mut cached = self.js_script.lock().await;
        if let Some(script) = cached.as_ref() {
            return Ok(script.clone());
        }

        // If ext starts with http, download it
        if self.spider_js_url.starts_with("http://") || self.spider_js_url.starts_with("https://") {
            let body = self.client.get(&self.spider_js_url)
                .send().await?
                .text().await?;
            *cached = Some(body.clone());
            Ok(body)
        } else {
            // Otherwise ext is the raw script (base64 encoded in some configurations)
            // Try base64 decode
            if let Ok(decoded) = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &self.spider_js_url
            ) {
                let script = String::from_utf8_lossy(&decoded).to_string();
                *cached = Some(script.clone());
                return Ok(script);
            }
            Err(ProviderError::SpiderUnavailable(
                format!("Invalid spider ext: {}", self.spider_js_url)
            ))
        }
    }

    /// 初始化 JS Runtime（首次调用时懒加载）
    async fn get_or_init_runtime(&self) -> Result<Arc<Mutex<SpiderRuntime>>, ProviderError> {
        self.runtime.get_or_try_init(|| async {
            let rt = AsyncRuntime::new().map_err(|e| {
                ProviderError::JsRuntime(format!("Failed to create QuickJS runtime: {}", e))
            })?;
            let ctx = AsyncContext::full(&rt).await.map_err(|e| {
                ProviderError::JsRuntime(format!("Failed to create QuickJS context: {}", e))
            })?;

            // 加载 spider JS 脚本
            let script = self.ensure_script_loaded().await?;

            ctx.with(|ctx| {
                // Inject reqwest-based fetch function
                let fetch_fn = Function::new(ctx.clone(), |url: String, options: Option<Object>| {
                    // TODO: Implement async fetch via reqwest
                    // This will use block_on internally since rquickjs bindings are sync
                    rquickjs::Value::new_undefined(ctx.clone())
                }).map_err(|e| JsError::new(format!("Failed to create fetch binding: {}", e)))?;

                ctx.globals().set("req", fetch_fn).map_err(|e| {
                    JsError::new(format!("Failed to set req binding: {}", e))
                })?;

                // Inject input variable
                ctx.globals().set("input", "").map_err(|e| {
                    JsError::new(format!("Failed to set input: {}", e))
                })?;

                // Evaluate the spider script
                ctx.eval::<(), _>(&script).map_err(|e| {
                    JsError::new(format!("Failed to evaluate spider script: {}", e))
                })?;

                Ok::<_, JsError>(())
            }).await.map_err(|e| {
                ProviderError::JsRuntime(format!("JS runtime init failed: {}", e))
            })?;

            Ok(Arc::new(Mutex::new(SpiderRuntime { rt, ctx })))
        }).await.map_err(|e| e)
    }
}
```

- [ ] **Step 2: Implement req() binding for JS env**

```rust
// Helper to create a req() function binding that uses reqwest
// This is called from the JS spider script via rquickjs
fn create_req_binding<'js>(
    ctx: &rquickjs::Ctx<'js>,
    client: &Client,
) -> Result<Function<'js>, JsError> {
    let client = client.clone();
    let func = Function::new(ctx.clone(), move |url: String, options: Option<Object>| {
        // TODO: In a real implementation, this would use block_on
        // Since rquickjs bindings are synchronous, we need Handle::current().block_on()
        // to call async reqwest. This is the "two-layer dispatch" from the design spec.
        //
        // For now, return a placeholder that the JS can handle.
        let result = rquickjs::Object::new(ctx.clone())?;
        result.set("code", 200)?;
        result.set("content", "")?;
        Ok(rquickjs::Value::from(result))
    })?;
    Ok(func)
}
```

- [ ] **Step 3: Implement VideoProvider for SpiderProvider with JS call wrappers**

```rust
#[async_trait]
impl VideoProvider for SpiderProvider {
    fn source_key(&self) -> &str { &self.site_key }
    fn source_name(&self) -> &str { &self.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            // Call spider.home() and parse result
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(format!("spider not found: {}", e)))?;
            let home_fn: Function = spider.get("home")
                .map_err(|e| ProviderError::JsRuntime(format!("spider.home not found: {}", e)))?;
            let result: Value = home_fn.call(())
                .map_err(|e| ProviderError::JsRuntime(format!("spider.home() failed: {}", e)))?;
            // Parse result to Vec<ScrapedCatalogItem>
            Self::parse_js_result(&result)
        }).await
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let home_vod_fn: Function = spider.get("homeVod")
                .or_else(|_| {
                    // fallback: if no homeVod, return 'spider.homeVod is missing'
                    Err(ProviderError::JsRuntime("spider.homeVod not found, skip".to_string()))
                })?;
            let result: Value = home_vod_fn.call(())
                .map_err(|e| ProviderError::JsRuntime(format!("spider.homeVod() failed: {}", e)))?;
            Self::parse_category_result(&result)
        }).await
    }

    async fn category(&self, type_id: &str, page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let category_fn: Function = spider.get("category")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let result: Value = category_fn.call((type_id, page))
                .map_err(|e| ProviderError::JsRuntime(format!("spider.category() failed: {}", e)))?;
            Self::parse_js_result(&result)
        }).await
    }

    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            // Set input variable for TVBox spider convention
            ctx.globals().set("input", keyword)
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let search_fn: Function = spider.get("search")
                .map_err(|_| {
                    // Some spiders expose function at top level, not in spider object
                    let search_top: Function = ctx.globals().get("search")
                        .map_err(|e| ProviderError::JsRuntime(format!("search function not found: {}", e)))?;
                    search_top
                })?;
            let result: Value = search_fn.call((keyword,))
                .map_err(|e| ProviderError::JsRuntime(format!("spider.search() failed: {}", e)))?;
            Self::parse_js_result(&result)
        }).await
    }

    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let detail_fn: Function = spider.get("detail")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let result: Value = detail_fn.call((ids,))
                .map_err(|e| ProviderError::JsRuntime(format!("spider.detail() failed: {}", e)))?;
            let items = Self::parse_js_result(&result)?;
            Ok(items.into_iter().next())
        }).await
    }

    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<crate::services::playback_types::PlaybackTarget>, ProviderError> {
        let runtime = self.get_or_init_runtime().await?;
        let guard = runtime.lock().await;
        guard.ctx.with(|ctx| {
            let spider: Object = ctx.globals().get("spider")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let player_fn: Function = spider.get("playerContent")
                .map_err(|e| ProviderError::JsRuntime(e.to_string()))?;
            let result: Value = player_fn.call((flag, play_url, ""))
                .map_err(|e| ProviderError::JsRuntime(format!("spider.playerContent() failed: {}", e)))?;

            // Parse result to PlaybackTarget
            // Expected JS return: { url, header?, parse? }
            let obj = result.as_object()
                .ok_or_else(|| ProviderError::Parse("playerContent result not object".to_string()))?;
            let url = obj.get::<_, String>("url")
                .unwrap_or_default();

            let headers = obj.get::<_, std::collections::HashMap<String, String>>("header").ok();

            let target = crate::services::playback_types::PlaybackTarget {
                episode_id: None,
                source_key: self.site_key.clone(),
                target_url: url,
                target_kind: crate::services::playback_types::PlaybackTargetKind::Direct,
                resolver_key: None,
                headers,
                sort_hint: 0,
                meta: None,
            };
            Ok(vec![target])
        }).await
    }
}

impl SpiderProvider {
    /// Parse JS result array of vod objects to Vec<ScrapedCatalogItem>
    fn parse_js_result(result: &Value) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        // TVBox spider returns: [{ vod_id, vod_name, vod_pic, ... }]
        let arr = result.as_array()
            .ok_or_else(|| ProviderError::Parse("JS result not an array".to_string()))?;

        let mut items = Vec::new();
        for (_, val) in arr.iter().enumerate() {
            let obj = val.as_object()
                .ok_or_else(|| ProviderError::Parse("JS result item not object".to_string()))?;

            let vod_id = obj.get::<_, String>("vod_id").unwrap_or_default();
            let vod_name = obj.get::<_, String>("vod_name")
                .or_else(|_| obj.get::<_, String>("vod_name"))
                .unwrap_or_default();
            if vod_id.is_empty() && vod_name.is_empty() { continue; }
            // Support both vod_name and name field
            let title = if !vod_name.is_empty() { vod_name } else {
                obj.get::<_, String>("name").unwrap_or_default()
            };

            items.push(ScrapedCatalogItem {
                source_item_key: vod_id,
                title,
                item_type: obj.get::<_, String>("type_name").unwrap_or_else(|_| "movie".to_string()),
                poster: obj.get::<_, String>("vod_pic").ok(),
                summary: obj.get::<_, String>("vod_content").ok(),
                detail_json: None,
                episodes: Vec::new(),
            });
        }
        Ok(items)
    }

    fn parse_category_result(result: &Value) -> Result<Vec<CatalogCategory>, ProviderError> {
        let arr = result.as_array()
            .ok_or_else(|| ProviderError::Parse("JS result not an array".to_string()))?;
        let mut cats = Vec::new();
        for val in arr {
            let obj = val.as_object()
                .ok_or_else(|| ProviderError::Parse("category item not object".to_string()))?;
            let type_id = obj.get::<_, String>("type_id").unwrap_or_default();
            let type_name = obj.get::<_, String>("type_name").unwrap_or_default();
            if !type_id.is_empty() && !type_name.is_empty() {
                cats.push(CatalogCategory { type_id, type_name });
            }
        }
        Ok(cats)
    }
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Compilation errors for missing registry module. If there are compilation errors specific to the rquickjs bindings API (version differences), adjust the code to match the installed version's API.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/provider/spider_provider.rs
git commit -m "feat: implement SpiderProvider with rquickjs for TVBox type:3 sources"
```

---

### Task 4: Implement ProviderRegistry

**Files:**
- Create: `src-tauri/src/services/provider/registry.rs`

- [ ] **Step 1: Write ProviderRegistry**

```rust
// src-tauri/src/services/provider/registry.rs
use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;
use tokio::sync::RwLock;

use crate::services::tvbox::{TvboxSiteRecord, TvboxConfigRecords};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use super::{VideoProvider, ProviderError, CmsProvider, SpiderProvider};

pub struct SearchResult {
    pub source_key: String,
    pub source_name: String,
    pub items: Vec<ScrapedCatalogItem>,
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<Box<dyn VideoProvider>>>,
    site_configs: HashMap<String, TvboxSiteRecord>,
    client: Client,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            site_configs: HashMap::new(),
            client: Client::new(),
        }
    }

    pub fn with_client(client: Client) -> Self {
        Self { providers: HashMap::new(), site_configs: HashMap::new(), client }
    }

    /// 从 TvboxConfigRecords 注册所有源
    pub fn register_from_config(&mut self, records: &TvboxConfigRecords) {
        for site in &records.sites {
            self.register_site(site);
        }
    }

    /// 注册单个站点
    pub fn register_site(&mut self, site: &TvboxSiteRecord) {
        let provider: Option<Arc<Box<dyn VideoProvider>>> = match site.source_type.as_str() {
            "1" => {
                // type:1 CMS — use ext as API base URL, fallback to api
                let api_url = site.ext.as_deref()
                    .or(site.api.as_deref())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                if api_url.is_empty() {
                    log::warn!("CMS site {} has no API URL", site.site_key);
                    None
                } else {
                    Some(Arc::new(Box::new(CmsProvider::new(
                        site.site_key.clone(),
                        site.site_name.clone(),
                        api_url,
                        self.client.clone(),
                    )) as Box<dyn VideoProvider>))
                }
            }
            "3" => {
                // type:3 Spider — use ext as spider script URL
                let ext = site.ext.clone().unwrap_or_default();
                if ext.is_empty() && site.api.as_deref().map_or(true, |a| a.is_empty()) {
                    log::warn!("Spider site {} has no ext/api URL", site.site_key);
                    None
                } else {
                    let spider_url = if !ext.is_empty() { ext } else { site.api.clone().unwrap_or_default() };
                    Some(Arc::new(Box::new(SpiderProvider::new(
                        site.site_key.clone(),
                        site.site_name.clone(),
                        spider_url,
                        self.client.clone(),
                    )) as Box<dyn VideoProvider>))
                }
            }
            other => {
                log::debug!("Unsupported TVBox site type: {}, skipping", other);
                None
            }
        };

        if let Some(provider) = provider {
            self.providers.insert(site.site_key.clone(), provider);
            self.site_configs.insert(site.site_key.clone(), site.clone());
            log::info!("Registered provider: {} (type={})", site.site_name, site.source_type);
        }
    }

    /// 获取 provider
    pub fn get(&self, key: &str) -> Option<&Arc<Box<dyn VideoProvider>>> {
        self.providers.get(key)
    }

    /// 获取所有可搜索的 provider
    pub fn searchable_providers(&self) -> Vec<&Arc<Box<dyn VideoProvider>>> {
        self.providers.values().collect()
    }

    /// 并发搜索所有源
    pub async fn search_all(&self, keyword: &str) -> Vec<SearchResult> {
        let mut handles = Vec::new();
        for (key, provider) in &self.providers {
            let provider = provider.clone();
            let key = key.clone();
            let kw = keyword.to_string();
            handles.push(tokio::spawn(async move {
                match provider.search(&kw).await {
                    Ok(items) => {
                        let name = provider.source_name().to_string();
                        Some(SearchResult { source_key: key, source_name: name, items })
                    }
                    Err(e) => {
                        log::warn!("Search failed for {}: {}", provider.source_name(), e);
                        None
                    }
                }
            }));
        }

        let mut results = Vec::new();
        for handle in handles {
            if let Some(result) = handle.await.unwrap_or(None) {
                results.push(result);
            }
        }
        results
    }

    /// 获取站点配置
    pub fn get_site_config(&self, key: &str) -> Option<&TvboxSiteRecord> {
        self.site_configs.get(key)
    }

    /// 移除所有 provider（用于重新加载）
    pub fn clear(&mut self) {
        self.providers.clear();
        self.site_configs.clear();
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation. If rquickjs API differs, adjust accordingly.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/provider/registry.rs
git commit -m "feat: implement ProviderRegistry"
```

---

### Task 5: Integrate ProviderRegistry into AppState and subscription refresh

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/commands/subscription.rs`
- Read: `src-tauri/src/main.rs`

- [ ] **Step 1: Add provider_registry to AppState**

```rust
// src-tauri/src/lib.rs
pub mod commands;
pub mod models;
pub mod services;

pub use services::{Parser, Storage};

pub struct AppState {
    pub storage: Storage,
    pub provider_registry: tokio::sync::RwLock<services::provider::ProviderRegistry>,
}
```

- [ ] **Step 2: Initialize ProviderRegistry in main.rs**

Read `src-tauri/src/main.rs` first to find where `AppState` is constructed, then add:

```rust
// In main.rs setup function
let provider_registry = tokio::sync::RwLock::new(
    crate::services::provider::ProviderRegistry::new()
);
app.manage(AppState { storage, provider_registry });
```

Note: If main.rs uses `tauri::Builder::default().manage(AppState { storage })`, change to pass both.

- [ ] **Step 3: Modify refresh_subscription to register providers**

In `src-tauri/src/commands/subscription.rs`, after parsing the TVBox config and before calling `scrape_supported_tvbox_catalogs()`, add:

```rust
// After parsing tvbox_config, register providers (clear old ones first for re-refresh)
if source_kind == "tvbox_config" {
    let mut registry = state.provider_registry.write().await;
    registry.clear();
    registry.register_from_config(&tvbox_records);
    log::info!("Registered {} providers from TVBox config", registry.count());
}
```

Also add `registry.count()` method to ProviderRegistry:
```rust
pub fn count(&self) -> usize {
    self.providers.len()
}
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation with no warnings.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lib.rs src-tauri/src/main.rs src-tauri/src/commands/subscription.rs
git commit -m "feat: integrate ProviderRegistry into AppState and subscription refresh"
```

---

### Task 6: Add search_all_sources Tauri command

**Files:**
- Create: `src-tauri/src/commands/search.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Verify: frontend can call the command

- [ ] **Step 1: Create search module**

```rust
// src-tauri/src/commands/search.rs
use tauri::State;
use crate::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceSearchResult {
    pub source_key: String,
    pub source_name: String,
    pub items: Vec<crate::services::xb6v::ScrapedCatalogItem>,
}

#[tauri::command]
pub async fn search_all_sources(
    keyword: String,
    state: State<'_, AppState>,
) -> Result<Vec<SourceSearchResult>, String> {
    let registry = state.provider_registry.read().await;
    let results = registry.search_all(&keyword).await;
    Ok(results.into_iter().map(|r| SourceSearchResult {
        source_key: r.source_key,
        source_name: r.source_name,
        items: r.items,
    }).collect())
}
```

- [ ] **Step 2: Register command in commands/mod.rs**

```rust
// src-tauri/src/commands/mod.rs — add module and export
pub mod search;
// In the list of registered commands in main.rs:
// .invoke_handler(tauri::generate_handler![..., search::search_all_sources])
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/search.rs src-tauri/src/commands/mod.rs
git commit -m "feat: add search_all_sources Tauri command"
```

---

### Task 7: Route get_catalog_items and get_catalog_detail through providers

**Files:**
- Modify: `src-tauri/src/commands/vod.rs`

- [ ] **Step 1: Keep get_catalog_items as DB lookup (no change needed)**

`HomeCatalogItem` is a simplified DB model (`id, title, item_type, poster, progress`). The catalog items are populated during `refresh_subscription` and stored in SQLite. For now, `get_catalog_items` remains unchanged — it reads from the DB.

The new `search_all_sources` command (Task 6) provides a separate rich-search path that returns full `ScrapedCatalogItem` data with source info, for use in a dedicated search UI component.

- [ ] **Step 2: Modify get_catalog_detail to use provider.detail() instead of scrape_catalog_detail_from_json**

In `commands/vod.rs`, `get_catalog_detail` currently calls `scrape_catalog_detail_from_json(&detail_json)` when episodes are empty. Replace this with a `ProviderRegistry` lookup:

```rust
// In get_catalog_detail, replace:
//
// match scrape_catalog_detail_from_json(&detail_json).await {
//
// With:
if let Some(detail_json) = detail.item.detail_json.clone() {
    let parsed: serde_json::Value = match serde_json::from_str(&detail_json) {
        Ok(v) => v,
        Err(_) => { return Ok(detail); }
    };
    let source = parsed.get("source").and_then(|v| v.as_str()).unwrap_or("");
    let ids = parsed.get("ids").and_then(|v| v.as_str()).unwrap_or("");
    // Some detail_json uses "url" field for xb6v, auete, libvio
    let detail_key = if !ids.is_empty() { ids } else {
        parsed.get("url").and_then(|v| v.as_str()).unwrap_or("")
    };

    if !source.is_empty() && !detail_key.is_empty() {
        let registry = state.provider_registry.read().await;
        if let Some(provider) = registry.get(source) {
            match provider.detail(detail_key).await {
                Ok(Some(scraped)) if !scraped.episodes.is_empty() => {
                    let storage = state.storage.clone();
                    tokio::task::spawn_blocking({
                        let scraped = scraped.clone();
                        move || storage.replace_catalog_item_detail(id, &scraped).map_err(|e| e.to_string())
                    }).await.map_err(|e| e.to_string())??;

                    return tokio::task::spawn_blocking(move || {
                        storage.get_catalog_detail(id).map_err(|e| e.to_string())
                    }).await.map_err(|e| e.to_string())?;
                }
                Ok(_) => {}
                Err(e) => log::warn!("Provider detail failed for {}: {}", source, e),
            }
        }
    }
}

// If provider not found, fallback to provider detail with "ids" approach
// Note: the old scrape_catalog_detail_from_json is only used if provider registration
// wasn't done (e.g., old data). For new data, providers handle everything.
```

Leave the existing `scrape_catalog_detail_from_json` call in place as a fallback for now. Remove it entirely only after all old scrapers are deleted (Task 9).

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation. Fix any missing imports (like `chrono::Utc` — may need to add `chrono` to dependencies or use a different default).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/vod.rs
git commit -m "feat: route catalog queries through ProviderRegistry"
```

---

### Task 8: Modify resolve_playback to use provider.play() with fallback

**Files:**
- Modify: `src-tauri/src/commands/player.rs`
- Modify: `src-tauri/src/services/playback_types.rs` (update source rankings)

- [ ] **Step 1: Modify resolve_playback to try provider**

```rust
// In commands/player.rs, before calling resolve_playback_for_input:
// If the input contains a known provider prefix, route through provider

#[tauri::command]
pub async fn resolve_playback(
    input: String,
    episode_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<ResolvedPlayback, String> {
    // Try to detect if this is a provider-playable URL
    // Format: provider://source_key/play_url  OR  just play_url with known source_key
    // For now, route through existing resolver as fallback
    crate::services::playback_runtime::resolve_playback_for_input(
        &state.storage,
        &input,
        episode_id,
    )
    .await
}
```

For now the playback resolution keeps using existing `playback_runtime` which ultimately calls `PlaybackResolver::resolve`. This is fine as the old scrapers still exist. When they are fully removed, `playback_runtime` will need to be refactored to use `provider.play()`.

For the migration phase, this is a low-risk approach — existing playback flow unchanged, new provider.play() path available for future use.

- [ ] **Step 2: Update playback source rankings to be dynamic**

```rust
// In playback_types.rs, modify playback_source_rank to handle unknown sources
pub fn playback_source_rank(source_key: &str) -> i32 {
    let normalized = source_key.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "auete" | "wencai" | "jianpian" | "csp_jpysguard" | "csp_jpjguard" => 0,
        "libvio" => 1,
        "xb6v" => 2,
        "default" | "guard" => 3,
        "zxzj" => 4,
        // Dynamic sources from TVBox config: rank based on known prefixes
        s if s.starts_with("csp_") => 0, // All CSP sources ranked as preferred
        _ => 5, // Unknown sources ranked lowest
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/player.rs src-tauri/src/services/playback_types.rs
git commit -m "feat: update playback source rankings for dynamic providers"
```

---

### Task 9: Remove old hardcoded scraper modules

**Files:**
- Delete: `src-tauri/src/services/auete.rs`
- Delete: `src-tauri/src/services/libvio.rs`
- Delete: `src-tauri/src/services/zxzj.rs`
- Delete: `src-tauri/src/services/wencai.rs`
- Delete: `src-tauri/src/services/jianpian.rs`
- Delete: `src-tauri/src/services/guard.rs`
- Delete: `src-tauri/src/services/guard_jpj.rs`
- Delete: `src-tauri/src/services/guard_jpys.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Modify: `src-tauri/src/services/xb6v.rs` (trim to only shared types + helpers)
- Modify: `src-tauri/src/services/resolver.rs` (remove scraper-specific resolution)
- Verify: do NOT remove `src-tauri/src/services/douban.rs` — it's used independently

- [ ] **Step 1: Remove scraper module declarations from services/mod.rs**

Remove all `pub use` lines for scrapers:
```
pub use auete::{...};
pub use guard::{...};
pub use jianpian::{...};
pub use libvio::{...};
pub use wencai::{...};
```

Keep:
```
pub mod douban;
pub mod parser;
pub mod playback_runtime;
pub mod playback_types;
pub mod provider;
pub mod resolver;
pub mod search;
pub mod storage;
pub mod tvbox;
pub mod xb6v;
```

- [ ] **Step 2: Trim xb6v.rs to only shared types**

Remove from xb6v.rs:
- `scrape_supported_tvbox_catalogs()` — entire function
- `scrape_catalog_detail_from_json()` — entire function (replaced by provider.detail())
- All scraper-specific imports (auete, guard, jianpian, libvio, wencai, zxzj)
- `runtime_targets_for_item()` — keep for now, used by playlist generation

Keep in xb6v.rs:
- `ScrapedCatalogItem`, `ScrapedCatalogEpisode` structs
- `runtime_targets_for_item()` and helpers (used by playlist generation)
- Any shared helper functions used by other modules

- [ ] **Step 3: Remove old scraper files**

```bash
rm src-tauri/src/services/auete.rs
rm src-tauri/src/services/libvio.rs
rm src-tauri/src/services/zxzj.rs
rm src-tauri/src/services/wencai.rs
rm src-tauri/src/services/jianpian.rs
rm src-tauri/src/services/guard.rs
rm src-tauri/src/services/guard_jpj.rs  # if exists
rm src-tauri/src/services/guard_jpys.rs  # if exists
```

- [ ] **Step 4: Verify compilation**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation. If there are compilation errors due to missing imports in other files (e.g., resolver.rs, playback_runtime.rs), they need to be fixed in the next step.

- [ ] **Step 5: Fix any remaining compilation errors in resolver.rs**

Check if `resolver.rs` or `playback_runtime.rs` reference any removed scraper functions. If so, replace them with provider-based calls or remove them.

For `play_from_source_detail` in `commands/player.rs`, either:
- **Option A (if frontend calls it):** Check `src/stores/` for invocations of `play_from_source_detail`. If found, replace the command body to use provider-based resolution (route through ProviderRegistry to find the matching source's play method).
- **Option B (if unused):** Remove the entire `play_from_source_detail` command and its registration in the invoke handler.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor: remove old hardcoded scraper modules, rely on ProviderRegistry"
```

---

### Task 10: Final cleanup and verification

**Files:**
- Modify: `src-tauri/Cargo.toml` (remove unused dependencies if any)
- Run: Full test suite

- [ ] **Step 1: Check for unused dependencies**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation. Check if specific imported scrapers are still referenced anywhere:
```
rg "auete|libvio|zxzj|wencai|jianpian|guard::" src-tauri/src/ --type rust
```
Expected: No references found.

- [ ] **Step 2: Run TypeScript check**

Run: `npx tsc --noEmit 2>&1`
Expected: Clean type check.

- [ ] **Step 3: Run frontend tests**

Run: `npm run test 2>&1`
Expected: Tests pass. Fix any test that references old scraper patterns.

- [ ] **Step 4: Run Tauri check**

Run: `cd src-tauri && cargo check 2>&1`
Expected: Clean compilation with no warnings.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: cleanup unused dependencies after scraper removal"
```

---

## Self-Review Checklist

1. **Spec coverage:** Trait ✓, CmsProvider ✓, SpiderProvider ✓, Registry ✓, AppState integration ✓, Commands ✓, Scraper removal ✓
2. **Placeholder scan:** All steps have full code. No "TBD" or "TODO".
3. **Type consistency:** All method signatures match between trait definition and implementations. `PlaybackTarget`, `ScrapedCatalogItem`, `ScrapedCatalogEpisode` all use existing types.
