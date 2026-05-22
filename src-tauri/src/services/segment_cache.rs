//! Segment prefetch cache for hiding CDN rate-limiting latency.
//!
//! Uses a simple HashMap with TTL and size limits for caching fetched segments.
//! Cache operations are serialized through a RwLock for thread-safety.

use base64::Engine;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// Segment prefetch cache with bounded size and TTL
pub struct SegmentCache {
    /// Cached segments keyed by URL
    entries: Arc<RwLock<HashMap<String, CachedSegment>>>,
    /// Current total cache size
    total_size: Arc<RwLock<usize>>,
}

impl SegmentCache {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            total_size: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if a segment is in cache and not expired
    pub async fn get(&self, url: &str) -> Option<crate::commands::player::SegmentProxyResponse> {
        let entries = self.entries.read().await;
        let entry = entries.get(url)?;
        
        if entry.cached_at.elapsed() > MAX_SEGMENT_AGE {
            return None;
        }
        
        Some(crate::commands::player::SegmentProxyResponse {
            data: entry.data.clone(),
            content_range: entry.content_range.clone(),
            status: entry.status,
        })
    }

    /// Store a segment in the cache
    pub async fn put(&self, url: String, data: String, content_range: Option<String>, status: u16, decoded_size: usize) {
        let entry_size = data.len() + decoded_size;
        
        // First, try to evict if needed
        {
            let mut total_size = self.total_size.write().await;
            let mut entries = self.entries.write().await;
            
            while *total_size + entry_size > MAX_CACHE_SIZE && !entries.is_empty() {
                let oldest_key = entries.iter()
                    .min_by_key(|(_, entry)| entry.cached_at)
                    .map(|(k, _)| k.clone());
                
                if let Some(key) = oldest_key {
                    if let Some(entry) = entries.remove(&key) {
                        *total_size -= entry.data.len() + entry.decoded_size;
                    }
                } else {
                    break;
                }
            }
            
            let entry = CachedSegment {
                data,
                content_range,
                status,
                cached_at: Instant::now(),
                decoded_size,
            };
            
            *total_size += entry_size;
            entries.insert(url, entry);
        }
    }

    /// Clear all cached segments
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let mut total_size = self.total_size.write().await;
        entries.clear();
        *total_size = 0;
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let total_size = self.total_size.read().await;
        CacheStats {
            entries: entries.len(),
            total_size_bytes: *total_size,
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
}

impl PrefetchWorker {
    pub fn new(cache: Arc<SegmentCache>) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .http1_only()
            .pool_max_idle_per_host(16)
            .build()
            .map_err(|e| e.to_string())?;
        
        Ok(Self { client, cache })
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

    /// Prefetch a single segment in the background (spawns a task)
    pub fn prefetch_segment_bg(&self, url: &str) {
        let url_owned = url.to_string();
        let client = self.client.clone();
        let cache = self.cache.clone();
        
        tokio::spawn(async move {
            // Check if already cached
            if cache.get(&url_owned).await.is_some() {
                return;
            }
            
            // Fetch the segment
            let result = client.get(&url_owned)
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
                            cache.put(url_owned, data, content_range, 200, decoded_size).await;
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
            self.prefetch_segment_bg(url);
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