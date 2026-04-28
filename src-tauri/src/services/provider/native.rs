// src-tauri/src/services/provider/native.rs
use reqwest::Client;
use crate::services::provider::ProviderError;

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

    /// POST form data and follow redirect (for search endpoints that redirect to results)
    pub async fn post_form_follow_redirect(&self, url: &str, data: &[(&str, &str)]) -> Result<String, ProviderError> {
        let resp = self.client.post(url)
            .form(data)
            .send()
            .await?;

        // Check if we got redirected (xb6v returns 302 to result page)
        if let Some(location) = resp.headers().get("location") {
            let loc = location.to_str().unwrap_or("");
            if !loc.is_empty() {
                // Build full redirect URL - location is relative to current path
                let redirect_url = if loc.starts_with("http") {
                    loc.to_string()
                } else {
                    // e.g. "result/?searchid=606" -> "https://www.xb6v.com/e/search/result/?searchid=606"
                    // Extract base path from original URL
                    let base_path = &url[self.base_url.len()..]; // e.g. "/e/search/1index.php"
                    let search_base = &base_path[..base_path.rfind('/').unwrap_or(0)]; // e.g. "/e/search"
                    // Ensure proper joining with "/"
                    let loc_prefix = if loc.starts_with('/') { "" } else { "/" };
                    format!("{}{}{}{}", self.base_url, search_base, loc_prefix, loc)
                };
                return self.client.get(&redirect_url)
                    .send()
                    .await?
                    .text()
                    .await
                    .map_err(|e| ProviderError::Http(e));
            }
        }

        resp.text().await.map_err(|e| ProviderError::Http(e))
    }
}
