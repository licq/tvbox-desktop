use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;

use crate::services::tvbox::{TvboxSiteRecord, TvboxConfigRecords};
use crate::services::xb6v::ScrapedCatalogItem;
use super::{VideoProvider, CmsProvider, SpiderProvider, NativeScraper, Xb6vScraper, AueteScraper, ZxzjScraper, JianpianScraper, WencaiScraper, LibvioScraper, YgpScraper, KkssScraper, UussScraper, YcyzScraper, LiteAppleScraper, NuomiScraper, BaibaiScraper, ChangzhangScraper, YicaiScraper, BiteScraper, DdrkScraper, MengmiScraper, XiongdiScraper, ReboScraper, HuanshiScraper, Dm84Scraper, YsjScraper, Anime1Scraper, YpansoScraper, XzsoScraper, MisoScraper, KuasouScraper, AlisoScraper, YisoScraper, BiliScraper, BiliychScraper, FanScraper, CcScraper};

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

    /// Register all known native Rust scrapers.
    /// Each scraper is created and registered with its source key.
    pub fn register_all_native_sources(&mut self) {
        // xb6v
        self.providers.insert("xb6v".to_string(), Arc::new(Box::new(Xb6vScraper::new())));
        // auete
        self.providers.insert("auete".to_string(), Arc::new(Box::new(AueteScraper::new())));
        // zxzj
        self.providers.insert("zxzj".to_string(), Arc::new(Box::new(ZxzjScraper::new())));
        // jianpian
        self.providers.insert("jianpian".to_string(), Arc::new(Box::new(JianpianScraper::new())));
        // wencai
        self.providers.insert("wencai".to_string(), Arc::new(Box::new(WencaiScraper::new())));
        // libvio
        self.providers.insert("libvio".to_string(), Arc::new(Box::new(LibvioScraper::new())));
        // YGP
        self.providers.insert("YGP".to_string(), Arc::new(Box::new(YgpScraper::new())));
        // 抠搜
        self.providers.insert("抠搜".to_string(), Arc::new(Box::new(KkssScraper::new())));
        // UC
        self.providers.insert("UC".to_string(), Arc::new(Box::new(UussScraper::new())));
        // 原创
        self.providers.insert("原创".to_string(), Arc::new(Box::new(YcyzScraper::new())));
        // 苹果
        self.providers.insert("苹果".to_string(), Arc::new(Box::new(LiteAppleScraper::new())));
        // 糯米
        self.providers.insert("糯米".to_string(), Arc::new(Box::new(NuomiScraper::new())));
        // 白白
        self.providers.insert("白白".to_string(), Arc::new(Box::new(BaibaiScraper::new())));
        // 厂长
        self.providers.insert("厂长".to_string(), Arc::new(Box::new(ChangzhangScraper::new())));
        // 溢彩
        self.providers.insert("溢彩".to_string(), Arc::new(Box::new(YicaiScraper::new())));
        // 比特
        self.providers.insert("比特".to_string(), Arc::new(Box::new(BiteScraper::new())));
        // 低端
        self.providers.insert("低端".to_string(), Arc::new(Box::new(DdrkScraper::new())));
        // 萌米
        self.providers.insert("萌米".to_string(), Arc::new(Box::new(MengmiScraper::new())));
        // 兄弟
        self.providers.insert("兄弟".to_string(), Arc::new(Box::new(XiongdiScraper::new())));
        // 热播
        self.providers.insert("热播".to_string(), Arc::new(Box::new(ReboScraper::new())));
        // 欢视
        self.providers.insert("欢视".to_string(), Arc::new(Box::new(HuanshiScraper::new())));
        // Dm84
        self.providers.insert("Dm84".to_string(), Arc::new(Box::new(Dm84Scraper::new())));
        // Ysj
        self.providers.insert("Ysj".to_string(), Arc::new(Box::new(YsjScraper::new())));
        // Anime1
        self.providers.insert("Anime1".to_string(), Arc::new(Box::new(Anime1Scraper::new())));
        // YpanSo
        self.providers.insert("YpanSo".to_string(), Arc::new(Box::new(YpansoScraper::new())));
        // xzso
        self.providers.insert("xzso".to_string(), Arc::new(Box::new(XzsoScraper::new())));
        // 米搜
        self.providers.insert("米搜".to_string(), Arc::new(Box::new(MisoScraper::new())));
        // 夸搜
        self.providers.insert("夸搜".to_string(), Arc::new(Box::new(KuasouScraper::new())));
        // Aliso
        self.providers.insert("Aliso".to_string(), Arc::new(Box::new(AlisoScraper::new())));
        // 易搜
        self.providers.insert("易搜".to_string(), Arc::new(Box::new(YisoScraper::new())));
        // Bili
        self.providers.insert("Bili".to_string(), Arc::new(Box::new(BiliScraper::new())));
        // Biliych
        self.providers.insert("Biliych".to_string(), Arc::new(Box::new(BiliychScraper::new())));
        // fan
        self.providers.insert("fan".to_string(), Arc::new(Box::new(FanScraper::new())));
        // cc
        self.providers.insert("cc".to_string(), Arc::new(Box::new(CcScraper::new())));
    }
}