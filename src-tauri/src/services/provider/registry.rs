use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;
use tokio::sync::RwLock;

use crate::services::tvbox::{TvboxSiteRecord, TvboxConfigRecords};
use crate::services::xb6v::ScrapedCatalogItem;
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

    pub fn get(&self, key: &str) -> Option<&Arc<Box<dyn VideoProvider>>> {
        self.providers.get(key)
    }

    pub fn searchable_providers(&self) -> Vec<&Arc<Box<dyn VideoProvider>>> {
        self.providers.values().collect()
    }

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

    pub fn get_site_config(&self, key: &str) -> Option<&TvboxSiteRecord> {
        self.site_configs.get(key)
    }

    pub fn clear(&mut self) {
        self.providers.clear();
        self.site_configs.clear();
    }

    pub fn count(&self) -> usize {
        self.providers.len()
    }
}