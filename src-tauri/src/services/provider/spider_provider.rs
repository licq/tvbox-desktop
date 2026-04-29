// src-tauri/src/services/provider/spider_provider.rs
use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use rquickjs::{Context, Function, Object, Runtime, Value};
use std::thread;
use tokio::sync::{mpsc, oneshot};

use super::{VideoProvider, ProviderError};
use super::traits::CatalogCategory;
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};

enum JsThreadCommand {
    CallMethod {
        method: String,
        args: Vec<String>,
        response: oneshot::Sender<Result<String, ProviderError>>,
    },
}

/// A handle to communicate with the JS runtime thread
struct JsThreadHandle {
    sender: mpsc::Sender<JsThreadCommand>,
}

impl JsThreadHandle {
    fn new(site_key: String, _site_name: String, ext: String, client: Client) -> Result<Self, ProviderError> {
        let (sender, mut receiver) = mpsc::channel(32);

        // Spawn a dedicated thread for this provider's JS runtime
        thread::Builder::new()
            .name(format!("spider-js-{}", site_key))
            .spawn(move || {
                // Create synchronous runtime and context
                let rt = Runtime::new().expect("Failed to create JS runtime");
                let ctx = Context::full(&rt).expect("Failed to create JS context");

                // Script and client for the thread
                let mut js_script: Option<String> = None;
                let client = client;

                // If ext is a URL, fetch the script content synchronously on this thread
                if ext.starts_with("http://") || ext.starts_with("https://") {
                    // Use tokio runtime to fetch the script synchronously
                    let rt = tokio::runtime::Handle::current();
                    let ext_url = ext.clone();
                    let result = rt.block_on(async {
                        client.get(&ext_url).send().await
                    });
                    match result {
                        Ok(resp) => {
                            let text_result = rt.block_on(resp.text());
                            if let Ok(text) = text_result {
                                js_script = Some(text);
                            }
                        }
                        Err(e) => {
                            log::warn!("[spider-js-{}] Failed to fetch spider script from {}: {}", site_key, ext, e);
                        }
                    }
                } else {
                    // Base64 encoded
                    if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&ext) {
                        js_script = Some(String::from_utf8_lossy(&decoded).to_string());
                    } else {
                        js_script = Some(ext.clone());
                    }
                }

                // Process commands
                while let Some(cmd) = receiver.blocking_recv() {
                    match cmd {
                        JsThreadCommand::CallMethod { method, args, response } => {
                            let result = Self::execute_method(
                                &rt, &ctx,
                                js_script.as_deref(),
                                &client,
                                &method,
                                &args,
                            );
                            let _ = response.send(result);
                        }
                    }
                }
            })
            .expect("Failed to spawn JS thread");

        Ok(Self { sender })
    }

    fn execute_method(
        _rt: &Runtime,
        ctx: &Context,
        script: Option<&str>,
        client: &Client,
        method: &str,
        args: &[String],
    ) -> Result<String, ProviderError> {
        ctx.with(|ctx| {
            // If we have a script, load it
            if let Some(script) = script {
                // Create req() binding - need to clone ctx for use in closure
                let ctx_for_closure = ctx.clone();
                let client_for_closure = client.clone();
                let req_fn = Function::new(ctx.clone(), move |url: String, _options: Option<Object>| {
                    let rt = tokio::runtime::Handle::current();
                    let client = client_for_closure.clone();
                    let result = rt.block_on(async {
                        client.get(&url).send().await
                    });

                    let obj = Object::new(ctx_for_closure.clone()).unwrap_or_else(|_| Object::new(ctx_for_closure.clone()).expect("failed to create object"));
                    match result {
                        Ok(resp) => {
                            let status = resp.status().as_u16() as i64;
                            let content = rt.block_on(resp.text()).unwrap_or_default();
                            let _ = obj.set("code", status);
                            let _ = obj.set("content", content);
                        }
                        Err(_) => {
                            let _ = obj.set("code", 500i64);
                            let _ = obj.set("content", "");
                        }
                    }
                    Value::from(obj)
                }).map_err(|e| ProviderError::JsRuntime(format!("req binding: {}", e)))?;

                // Create input global - use String::from("")
                let input_val = rquickjs::String::from_str(ctx.clone(), "").unwrap_or_else(|_| {
                    rquickjs::String::from_str(ctx.clone(), "").expect("failed to create empty string")
                });

                // Set globals
                ctx.globals().set("req", req_fn).expect("failed to set req");
                ctx.globals().set("input", Value::from(input_val)).expect("failed to set input");

                // Evaluate script
                ctx.eval::<(), _>(script)?;
            }

            // Get the spider global object
            let spider: Object = ctx.globals().get("spider")?;

            // Get the method
            let method_fn: Function = spider.get(method)?;

            // Call the method with appropriate args
            let result: Value = match method {
                "home" | "homeVod" | "search" | "category" => {
                    method_fn.call::<_, Value>(())?
                }
                "detail" | "playerContent" => {
                    if let Some(arg) = args.first() {
                        method_fn.call::<_, Value>((arg,))?
                    } else {
                        method_fn.call::<(), Value>(())?
                    }
                }
                _ => {
                    return Err(ProviderError::JsRuntime(format!("Unknown method: {}", method)));
                }
            };

            // Convert result to JSON string
            // json_stringify returns Option<rquickjs::String>, which wraps the JS string Value
            let json_str = match ctx.json_stringify(result) {
                Ok(Some(s)) => {
                    // rquickjs::String derefs to Value, use the get method with FromJs to convert
                    let rust_str: String = (&s).get().map_err(|e| ProviderError::JsRuntime(format!("string conversion: {}", e)))?;
                    rust_str
                }
                Ok(None) => "null".to_string(),
                Err(e) => return Err(ProviderError::JsRuntime(format!("JSON stringify: {}", e))),
            };

            Ok(json_str)
        }).map_err(|e| ProviderError::JsRuntime(e.to_string()))
    }
}

pub struct SpiderProvider {
    site_key: String,
    site_name: String,
    js_thread: JsThreadHandle,
}

impl SpiderProvider {
    pub fn new(site_key: String, site_name: String, ext: String, client: Client) -> Self {
        // JsThreadHandle::new() now handles URL fetching and base64 decoding internally
        let js_thread = JsThreadHandle::new(site_key.clone(), site_name.clone(), ext, client.clone())
            .expect("Failed to create JS thread");

        Self {
            site_key,
            site_name,
            js_thread,
        }
    }

    async fn call_spider_method(&self, method: &str, args: &[String]) -> Result<String, ProviderError> {
        let (tx, rx) = oneshot::channel();
        self.js_thread.sender.send(JsThreadCommand::CallMethod {
            method: method.to_string(),
            args: args.to_vec(),
            response: tx,
        }).await.map_err(|e| ProviderError::JsRuntime(e.to_string()))?;

        rx.await.map_err(|e| ProviderError::JsRuntime(e.to_string()))?
    }

    fn parse_js_result(&self, json_str: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let array: Vec<serde_json::Value> = serde_json::from_str(json_str)?;

        let mut items = Vec::new();
        for item in array {
            let obj = item.as_object().ok_or_else(|| {
                ProviderError::Parse("Expected object in spider result array".to_string())
            })?;

            let title = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let vod_id = obj.get("vod_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let poster = obj.get("pic").and_then(|v| v.as_str()).map(|s| s.to_string());
            let summary = obj.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
            let raw_type = obj.get("type_name")
                .and_then(|v| v.as_str())
                .or_else(|| obj.get("type").and_then(|v| v.as_str()))
                .unwrap_or("movie");
            let item_type = crate::services::provider::normalize_item_type(raw_type);

            if title.is_empty() && vod_id.is_empty() {
                continue;
            }

            let source_item_key = if vod_id.is_empty() {
                format!("{}:{}", self.site_key, title)
            } else {
                format!("{}:{}", self.site_key, vod_id)
            };

            items.push(ScrapedCatalogItem {
                source_item_key,
                title,
                item_type,
                poster,
                summary,
                detail_json: None,
                episodes: Vec::new(),
            });
        }

        Ok(items)
    }

    fn parse_category_result(&self, json_str: &str) -> Result<Vec<CatalogCategory>, ProviderError> {
        let array: Vec<serde_json::Value> = serde_json::from_str(json_str)?;

        let mut categories = Vec::new();
        for item in array {
            let obj = item.as_object().ok_or_else(|| {
                ProviderError::Parse("Expected object in category array".to_string())
            })?;

            let type_id = obj.get("type_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let type_name = obj.get("type_name").and_then(|v| v.as_str()).unwrap_or_default().to_string();

            if !type_id.is_empty() || !type_name.is_empty() {
                categories.push(CatalogCategory {
                    type_id,
                    type_name,
                });
            }
        }

        Ok(categories)
    }

    fn parse_detail_result(&self, json_str: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        // Detail can return a single object or an array with one item
        let value: serde_json::Value = serde_json::from_str(json_str)?;

        let obj = if let Some(arr) = value.as_array() {
            arr.first().and_then(|v| v.as_object())
        } else {
            value.as_object()
        };

        let obj = obj.ok_or_else(|| {
            ProviderError::Parse("Expected object from detail result".to_string())
        })?;

        let title = obj.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let vod_id = obj.get("vod_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let poster = obj.get("pic").and_then(|v| v.as_str()).map(|s| s.to_string());
        let summary = obj.get("desc").and_then(|v| v.as_str()).map(|s| s.to_string());
        let raw_type = obj.get("type_name")
            .and_then(|v| v.as_str())
            .or_else(|| obj.get("type").and_then(|v| v.as_str()))
            .unwrap_or("movie");
        let item_type = crate::services::provider::normalize_item_type(raw_type);

        if title.is_empty() && vod_id.is_empty() {
            return Ok(None);
        }

        // Parse episodes
        let mut episodes = Vec::new();
        if let Some(play_url) = obj.get("play_url").and_then(|v| v.as_str()) {
            if !play_url.is_empty() {
                let separator = if play_url.contains("$$$") {
                    "$$$"
                } else if play_url.contains('#') {
                    "#"
                } else {
                    ""
                };

                if !separator.is_empty() {
                    for (i, part) in play_url.split(separator).enumerate() {
                        if let Some(dollar_pos) = part.find('$') {
                            let label = &part[..dollar_pos];
                            let url = &part[dollar_pos + 1..];
                            episodes.push(ScrapedCatalogEpisode {
                                source_name: self.site_name.clone(),
                                episode_label: label.to_string(),
                                play_url: url.to_string(),
                                order_index: (i + 1) as i64,
                            });
                        }
                    }
                }
            }
        }

        let source_item_key = if vod_id.is_empty() {
            format!("{}:{}", self.site_key, title)
        } else {
            format!("{}:{}", self.site_key, vod_id)
        };

        Ok(Some(ScrapedCatalogItem {
            source_item_key,
            title,
            item_type,
            poster,
            summary,
            detail_json: Some(serde_json::json!({
                "source": self.site_key,
                "ids": vod_id,
            }).to_string()),
            episodes,
        }))
    }

    fn parse_play_result(&self, flag: &str, json_str: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        let value: serde_json::Value = serde_json::from_str(json_str)?;

        let target_url = if let Some(url) = value.as_str() {
            url.to_string()
        } else if let Some(obj) = value.as_object() {
            obj.get("url").and_then(|v| v.as_str()).unwrap_or_else(|| {
                obj.get("content").and_then(|v| v.as_str()).unwrap_or_default()
            }).to_string()
        } else {
            return Err(ProviderError::Parse("Invalid play result".to_string()));
        };

        if target_url.is_empty() {
            return Ok(Vec::new());
        }

        Ok(vec![PlaybackTarget {
            episode_id: None,
            source_key: self.site_key.clone(),
            target_url,
            target_kind: if flag.contains("m3u8") || flag.contains("mp4") {
                PlaybackTargetKind::Direct
            } else {
                PlaybackTargetKind::Resolvable
            },
            resolver_key: None,
            headers: None,
            sort_hint: 0,
            meta: None,
            referer: None,
        }])
    }
}

#[async_trait]
impl VideoProvider for SpiderProvider {
    fn source_key(&self) -> &str { &self.site_key }
    fn source_name(&self) -> &str { &self.site_name }

    async fn home(&self) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let json_str = self.call_spider_method("home", &[]).await?;
        self.parse_js_result(&json_str)
    }

    async fn home_vod(&self) -> Result<Vec<CatalogCategory>, ProviderError> {
        let json_str = self.call_spider_method("homeVod", &[]).await?;
        self.parse_category_result(&json_str)
    }

    async fn category(&self, type_id: &str, _page: u32) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let json_str = self.call_spider_method("category", &[type_id.to_string()]).await?;
        self.parse_js_result(&json_str)
    }

    async fn search(&self, keyword: &str) -> Result<Vec<ScrapedCatalogItem>, ProviderError> {
        let json_str = self.call_spider_method("search", &[keyword.to_string()]).await?;
        self.parse_js_result(&json_str)
    }

    async fn detail(&self, ids: &str) -> Result<Option<ScrapedCatalogItem>, ProviderError> {
        let json_str = self.call_spider_method("detail", &[ids.to_string()]).await?;
        self.parse_detail_result(&json_str)
    }

    async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
        let json_str = self.call_spider_method("playerContent", &[flag.to_string(), play_url.to_string()]).await?;
        self.parse_play_result(flag, &json_str)
    }
}
