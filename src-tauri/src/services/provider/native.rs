// src-tauri/src/services/provider/native.rs
use async_trait::async_trait;
use reqwest::Client;
use crate::services::playback_types::{PlaybackTarget, PlaybackTargetKind};
use crate::services::xb6v::{ScrapedCatalogItem, ScrapedCatalogEpisode};
use crate::services::provider::traits::CatalogCategory;
use crate::services::provider::{VideoProvider, ProviderError};

/// Base struct for all native scrapers. Each source embeds its own HTTP client.
pub struct NativeScraper {
    pub site_key: String,
    pub site_name: String,
    pub base_url: String,
    pub client: Client,
}

impl NativeScraper {
    pub fn new(site_key: &str, site_name: &str, base_url: &str) -> Self {
        Self {
            site_key: site_key.to_string(),
            site_name: site_name.to_string(),
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn fetch_text(&self, url: &str) -> Result<String, ProviderError> {
        self.client.get(url).send().await?
            .text().await.map_err(|e| ProviderError::Http(e))
    }

    pub async fn fetch_json(&self, url: &str) -> Result<serde_json::Value, ProviderError> {
        let text = self.fetch_text(url).await?;
        serde_json::from_str(&text).map_err(|e| ProviderError::Parse(e.to_string()))
    }
}