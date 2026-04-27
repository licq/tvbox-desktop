use std::collections::HashMap;
use std::sync::Arc;

use crate::services::tvbox::TvboxConfigRecords;
use crate::services::xb6v::ScrapedCatalogItem;
use super::{VideoProvider, Xb6vScraper, AueteScraper, ZxzjScraper, JianpianScraper, WencaiScraper, LibvioScraper, YgpScraper, KkssScraper, UussScraper, YcyzScraper, LiteAppleScraper, NuomiScraper, BaibaiScraper, ChangzhangScraper, YicaiScraper, BiteScraper, DdrkScraper, MengmiScraper, XiongdiScraper, ReboScraper, HuanshiScraper, Dm84Scraper, YsjScraper, Anime1Scraper, YpansoScraper, XzsoScraper, MisoScraper, KuasouScraper, AlisoScraper, YisoScraper, BiliScraper, BiliychScraper, FanScraper, CcScraper};

pub struct SearchResult {
    pub source_key: String,
    pub source_name: String,
    pub items: Vec<ScrapedCatalogItem>,
}

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<Box<dyn VideoProvider>>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

        /// 从 TvboxConfigRecords 注册所有源（使用原生 Rust scrapers）
    pub fn register_from_config(&mut self, _records: &TvboxConfigRecords) {
        // No longer uses TVBox config-based registration - just use native scrapers
        self.register_all_native_sources();
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

    pub fn clear(&mut self) {
        self.providers.clear();
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

#[cfg(test)]
mod native_scraper_tests {
    use super::*;

    /// Integration test: verifies all native scrapers are registered and search infrastructure works.
    #[tokio::test]
    async fn native_scrapers_register_and_search() {
        let mut registry = ProviderRegistry::new();
        registry.register_all_native_sources();

        let provider_count = registry.count();
        println!("Registered {} native scraper providers", provider_count);
        assert!(provider_count > 0, "should have registered providers");

        // Test that search_all runs without panicking and returns results structure
        let keyword = "功夫";
        let results = registry.search_all(keyword).await;
        let result_groups = results.len();
        let total_items: usize = results.iter().map(|r| r.items.len()).sum();

        println!("Search '{}': {} providers returned results, {} total items",
            keyword, result_groups, total_items);

        for result in &results {
            if !result.items.is_empty() {
                println!("  {} ({}): {} items", result.source_name, result.source_key, result.items.len());
                for item in result.items.iter().take(2) {
                    println!("    - {}", item.title);
                }
            }
        }

        // The test passes if we get here without panic
        // Note: actual results depend on whether the sites are reachable and return data
        // We just verify the infrastructure works (providers registered, search runs)
        assert!(provider_count >= 28, "should have at least 28 native scrapers registered");
    }

    /// Test that individual scraper can be retrieved by key
    #[tokio::test]
    async fn native_scrapers_get_by_key() {
        let mut registry = ProviderRegistry::new();
        registry.register_all_native_sources();

        // Test a few known keys
        let keys = vec!["xb6v", "auete", "zxzj", "jianpian", "wencai", "libvio"];
        for key in keys {
            let provider = registry.get(key);
            assert!(provider.is_some(), "provider {} should be registered", key);
            println!("Provider '{}' found: {} ({})", key, provider.unwrap().source_name(), provider.unwrap().source_key());
        }

        // Test some of the batch-2 keys
        let batch2_keys = vec!["白白", "厂长", "溢彩", "比特", "低端", "萌米"];
        for key in batch2_keys {
            let provider = registry.get(key);
            assert!(provider.is_some(), "provider {} should be registered", key);
        }
    }
}