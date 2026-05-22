//! Segment prefetch cache for hiding CDN rate-limiting latency.
//!
//! Uses a bounded channel to batch cache operations, avoiding lock contention
//! from spawning many concurrent tasks that all need to access the same HashMap.

use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex};

/// Maximum cache size in bytes (50MB)
const MAX_CACHE_SIZE: usize = 50 * 1024 * 1024;

/// Maximum age for cached segments (5 minutes)
const MAX_SEGMENT_AGE: Duration = Duration::from_secs(300);

/// Cached segment data with metadata
#[derive(Clone)]
struct CachedSegment {
    /// Base64-encoded segment data
    data: String,
    /// Content-Range header if available
    content_range: Option<String>,
    /// HTTP status code
    status: u16,
    /// When this entry was cached
    cached_at: Instant,
    /// Size in bytes of the original (decoded) data
    decoded_size: usize,
}

/// Message to the background cache worker
enum CacheOp {
    Put(String, String, Option<String>, u16, usize),
    Clear,
}

/// Internal cache state
struct CacheData {
    entries: HashMap<String, CachedSegment>,
    total_size: usize,
}

/// Segment prefetch cache with bounded size and TTL
pub struct SegmentCache {
    /// Internal cache state wrapped in Arc<Mutex<...>> for shared access
    cache: Arc<CacheOps>,
}

struct CacheOps {
    data: Arc<Mutex<CacheData>>,
    tx: mpsc::Sender<CacheOp>,
}

impl SegmentCache {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel(100); // Bounded channel for backpressure
        
        let data = Arc::new(Mutex::new(CacheData {
            entries: HashMap::new(),
            total_size: 0,
        }));
        
        let data_clone = data.clone();
        
        // Spawn background worker to process cache operations sequentially
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                match op {
                    CacheOp::Put(url, data, content_range, status, decoded_size) => {
                        Self::process_put(&data_clone, url, data, content_range, status, decoded_size).await;
                    }
                    CacheOp::Clear => {
                        let mut inner = data_clone.lock().await;
                        inner.entries.clear();
                        inner.total_size = 0;
                    }
                }
            }
        });
        
        Self { cache: Arc::new(CacheOps { data, tx }) }
    }

    /// Process a single cache put operation
    async fn process_put(
        data: &Arc<Mutex<CacheData>>,
        url: String,
        data_str: String,
        content_range: Option<String>,
        status: u16,
        decoded_size: usize,
    ) {
        let entry_size = data_str.len() + decoded_size;
        
        let mut inner = data.lock().await;
        
        // Evict old entries if needed
        while inner.total_size + entry_size > MAX_CACHE_SIZE && !inner.entries.is_empty() {
            let oldest_key = inner.entries.iter()
                .min_by_key(|(_, entry)| entry.cached_at)
                .map(|(k, _)| k.clone());
            
            if let Some(key) = oldest_key {
                if let Some(entry) = inner.entries.remove(&key) {
                    inner.total_size -= entry.data.len() + entry.decoded_size;
                }
            } else {
                break;
            }
        }
        
        let entry = CachedSegment {
            data: data_str,
            content_range,
            status,
            cached_at: Instant::now(),
            decoded_size,
        };
        
        inner.total_size += entry_size;
        inner.entries.insert(url, entry);
    }

    /// Check if a segment is in cache and not expired
    pub async fn get(&self, url: &str) -> Option<crate::commands::player::SegmentProxyResponse> {
        let inner = self.cache.data.lock().await;
        let entry = inner.entries.get(url)?;
        
        // Check if expired
        if entry.cached_at.elapsed() > MAX_SEGMENT_AGE {
            return None;
        }
        
        Some(crate::commands::player::SegmentProxyResponse {
            data: entry.data.clone(),
            content_range: entry.content_range.clone(),
            status: entry.status,
        })
    }

    /// Store a segment in the cache (non-blocking)
    pub fn put_bg(&self, url: String, data: String, content_range: Option<String>, status: u16, decoded_size: usize) {
        let _ = self.cache.tx.try_send(CacheOp::Put(url, data, content_range, status, decoded_size));
    }

    /// Clear all cached segments
    pub async fn clear(&self) {
        let _ = self.cache.tx.try_send(CacheOp::Clear);
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let inner = self.cache.data.lock().await;
        CacheStats {
            entries: inner.entries.len(),
            total_size_bytes: inner.total_size,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub entries: usize,
    pub total_size_bytes: usize,
}

/// Background prefetch worker that scans HLS playlists and pre-fetches segments
pub struct PrefetchWorker {
    /// HTTP client for fetching segments
    client: reqwest::Client,
    /// Segment cache
    cache: Arc<SegmentCache>,
    /// Currently prefetching URLs to avoid duplicate fetches
    in_flight: Arc<Mutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl PrefetchWorker {
    pub fn new(cache: Arc<SegmentCache>) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .http1_only()
            .pool_max_idle_per_host(16)
            .build()
            .map_err(|e| e.to_string())?;
        
        Ok(Self {
            client,
            cache,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Parse an HLS playlist and return all segment URLs
    fn extract_segment_urls(playlist_body: &str) -> Vec<String> {
        let mut urls = Vec::new();
        for line in playlist_body.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            urls.push(trimmed.to_string());
        }
        urls
    }

    /// Prefetch a single segment in the background
    pub async fn prefetch_segment(&self, url: &str) {
        if self.cache.get(url).await.is_some() {
            return;
        }
        
        {
            let in_flight = self.in_flight.lock().await;
            if in_flight.contains_key(url) {
                return;
            }
        }
        
        let url_owned = url.to_string();
        {
            let mut in_flight = self.in_flight.lock().await;
            if in_flight.contains_key(&url_owned) {
                return;
            }
            let _ = in_flight.insert(url_owned.clone(), tokio::spawn(async {}));
        }
        
        let client = self.client.clone();
        let cache = self.cache.clone();
        let in_flight = self.in_flight.clone();
        let url_for_task = url_owned.clone();
        
        let url_for_spawn = url_for_task.clone();
        tokio::spawn(async move {
            let result = client.get(&url_for_task)
                .header(reqwest::header::USER_AGENT, "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
                .header(reqwest::header::ACCEPT, "*/*")
                .send()
                .await;
            
            match result {
                Ok(resp) if resp.status().as_u16() == 200 => {
                    let content_range = resp.headers()
                        .get("content-range")
                        .and_then(|v| v.to_str().ok())
                        .map(String::from);
                    
                    match resp.bytes().await {
                        Ok(bytes) => {
                            let decoded_size = bytes.len();
                            let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
                            cache.put_bg(url_for_task, data, content_range, 200, decoded_size);
                        }
                        Err(e) => {
                            eprintln!("[prefetch] failed to read segment bytes: {}", e);
                        }
                    }
                }
                Ok(resp) => {
                    eprintln!("[prefetch] segment returned status: {}", resp.status());
                }
                Err(e) => {
                    eprintln!("[prefetch] failed to fetch segment: {}", e);
                }
            }
            
            in_flight.lock().await.remove(&url_for_spawn);
        });
    }

    /// Prefetch segments from an HLS playlist
    pub async fn prefetch_playlist_segments(&self, playlist_url: &str, start_after_index: Option<usize>, count: usize) {
        let body = match self.client.get(playlist_url)
            .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    eprintln!("[prefetch] failed to read playlist: {}", e);
                    return;
                }
            },
            Err(e) => {
                eprintln!("[prefetch] failed to fetch playlist: {}", e);
                return;
            }
        };
        
        let urls = Self::extract_segment_urls(&body);
        let start_idx = start_after_index.map(|i| i + 1).unwrap_or(0);
        let end_idx = (start_idx + count).min(urls.len());
        
        for url in urls[start_idx..end_idx].iter() {
            self.prefetch_segment(url).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_segment_urls() {
        let playlist = r#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:2
#EXT-X-MEDIA-SEQUENCE:0
#EXTINF:2.000000,
https://example.com/seg1.ts
#EXTINF:2.000000,
https://example.com/seg2.ts
#EXTINF:2.000000,
https://example.com/seg3.ts
"#;
        
        let urls = PrefetchWorker::extract_segment_urls(playlist);
        assert_eq!(urls.len(), 3);
    }
}