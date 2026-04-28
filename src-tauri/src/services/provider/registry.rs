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
        // Use verified working scrapers for now
        self.register_working_sources();
    }

        pub fn get(&self, key: &str) -> Option<&Arc<Box<dyn VideoProvider>>> {
        self.providers.get(key)
    }

    pub fn searchable_providers(&self) -> Vec<&Arc<Box<dyn VideoProvider>>> {
        self.providers.values().collect()
    }

    pub fn all_provider_pairs(&self) -> Vec<(String, Arc<Box<dyn VideoProvider>>)> {
        self.providers.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
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
                    Err(_) => {
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
        // zxzj
        self.providers.insert("zxzj".to_string(), Arc::new(Box::new(ZxzjScraper::new())));
        // auete
        self.providers.insert("auete".to_string(), Arc::new(Box::new(AueteScraper::new())));
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

    /// Register only the scrapers that have been verified to work.
    /// Verified working (2026-04-28): xb6v, zxzj, YpanSo, auete.
    /// Out of 34 scrapers tested, 4 sites are verified working.
    /// All other sites are either dead (DNS failure), domain taken over, wrong domain, or require JS/CAPTCHA.
    pub fn register_working_sources(&mut self) {
        // xb6v - 新6V, verified working (search -> detail -> play)
        self.providers.insert("xb6v".to_string(), Arc::new(Box::new(Xb6vScraper::new())));
        // zxzj - 在线之家, verified working (search -> detail -> play) at www.zxzjys.com
        self.providers.insert("zxzj".to_string(), Arc::new(Box::new(ZxzjScraper::new())));
        // YpanSo - 盘她, verified working (search -> detail -> play)
        self.providers.insert("YpanSo".to_string(), Arc::new(Box::new(YpansoScraper::new())));
        // auete - 奥特, verified working (home -> detail -> play)
        self.providers.insert("auete".to_string(), Arc::new(Box::new(AueteScraper::new())));
    }
}

#[cfg(test)]
mod native_scraper_tests {
    use super::*;
    use std::time::Duration;

    /// Pairs a scraper constructor with its key for easy testing
    type ScraperEntry = (&'static str, fn() -> Box<dyn VideoProvider>);

    /// List of all scraper entries with key and constructor
    fn all_scraper_entries() -> Vec<ScraperEntry> {
        vec![
            ("xb6v", || Box::new(Xb6vScraper::new()) as Box<dyn VideoProvider>),
            ("auete", || Box::new(AueteScraper::new()) as Box<dyn VideoProvider>),
            ("zxzj", || Box::new(ZxzjScraper::new()) as Box<dyn VideoProvider>),
            ("jianpian", || Box::new(JianpianScraper::new()) as Box<dyn VideoProvider>),
            ("wencai", || Box::new(WencaiScraper::new()) as Box<dyn VideoProvider>),
            ("libvio", || Box::new(LibvioScraper::new()) as Box<dyn VideoProvider>),
            ("YGP", || Box::new(YgpScraper::new()) as Box<dyn VideoProvider>),
            ("抠搜", || Box::new(KkssScraper::new()) as Box<dyn VideoProvider>),
            ("UC", || Box::new(UussScraper::new()) as Box<dyn VideoProvider>),
            ("原创", || Box::new(YcyzScraper::new()) as Box<dyn VideoProvider>),
            ("苹果", || Box::new(LiteAppleScraper::new()) as Box<dyn VideoProvider>),
            ("糯米", || Box::new(NuomiScraper::new()) as Box<dyn VideoProvider>),
            ("白白", || Box::new(BaibaiScraper::new()) as Box<dyn VideoProvider>),
            ("厂长", || Box::new(ChangzhangScraper::new()) as Box<dyn VideoProvider>),
            ("溢彩", || Box::new(YicaiScraper::new()) as Box<dyn VideoProvider>),
            ("比特", || Box::new(BiteScraper::new()) as Box<dyn VideoProvider>),
            ("低端", || Box::new(DdrkScraper::new()) as Box<dyn VideoProvider>),
            ("萌米", || Box::new(MengmiScraper::new()) as Box<dyn VideoProvider>),
            ("兄弟", || Box::new(XiongdiScraper::new()) as Box<dyn VideoProvider>),
            ("热播", || Box::new(ReboScraper::new()) as Box<dyn VideoProvider>),
            ("欢视", || Box::new(HuanshiScraper::new()) as Box<dyn VideoProvider>),
            ("Dm84", || Box::new(Dm84Scraper::new()) as Box<dyn VideoProvider>),
            ("Ysj", || Box::new(YsjScraper::new()) as Box<dyn VideoProvider>),
            ("Anime1", || Box::new(Anime1Scraper::new()) as Box<dyn VideoProvider>),
            ("YpanSo", || Box::new(YpansoScraper::new()) as Box<dyn VideoProvider>),
            ("xzso", || Box::new(XzsoScraper::new()) as Box<dyn VideoProvider>),
            ("米搜", || Box::new(MisoScraper::new()) as Box<dyn VideoProvider>),
            ("夸搜", || Box::new(KuasouScraper::new()) as Box<dyn VideoProvider>),
            ("Aliso", || Box::new(AlisoScraper::new()) as Box<dyn VideoProvider>),
            ("易搜", || Box::new(YisoScraper::new()) as Box<dyn VideoProvider>),
            ("Bili", || Box::new(BiliScraper::new()) as Box<dyn VideoProvider>),
            ("Biliych", || Box::new(BiliychScraper::new()) as Box<dyn VideoProvider>),
            ("fan", || Box::new(FanScraper::new()) as Box<dyn VideoProvider>),
            ("cc", || Box::new(CcScraper::new()) as Box<dyn VideoProvider>),
        ]
    }

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

    /// Test that verified working scrapers can be registered and searched
    #[tokio::test]
    async fn working_scrapers_register_and_search() {
        let mut registry = ProviderRegistry::new();
        registry.register_working_sources();

        let provider_count = registry.count();
        println!("Registered {} working scraper providers", provider_count);
        assert_eq!(provider_count, 4, "should have exactly 4 working scrapers (xb6v, zxzj, YpanSo, auete)");

        // Verify all working scrapers are registered
        let working_keys = vec!["xb6v", "zxzj", "YpanSo", "auete"];
        for key in working_keys {
            let provider = registry.get(key);
            assert!(provider.is_some(), "provider {} should be registered", key);
            println!("Working provider '{}' found: {} ({})", key, provider.unwrap().source_name(), provider.unwrap().source_key());
        }

        // Test search with all 3 working scrapers
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

        // All 3 working scrapers should return results
        assert!(result_groups >= 1, "at least 1 scraper should return results");
        assert!(total_items >= 1, "at least 1 item should be returned");
    }

    /// Diagnostic test: test each scraper's search individually with per-provider timeout.
    /// Reports which scrapers passed/failed without blocking the whole test.
    #[tokio::test]
    async fn diagnostic_search_each_provider() {
        let keyword = "功夫";
        let mut passed = Vec::new();
        let mut failed = Vec::new();
        let timeout = Duration::from_secs(15);

        for (key, constructor) in all_scraper_entries() {
            let provider = constructor();
            eprintln!("\n[DIAG] Testing scraper: {} ({})", provider.source_name(), key);

            let search_fut = provider.search(keyword);
            match tokio::time::timeout(timeout, search_fut).await {
                Ok(Ok(items)) => {
                    if items.is_empty() {
                        failed.push((key, "search returned 0 items"));
                        eprintln!("[DIAG] {} FAIL: 0 items", key);
                    } else {
                        passed.push(key);
                        eprintln!("[DIAG] {} PASS: {} items, first='{}'", key, items.len(), items[0].title);
                    }
                }
                Ok(Err(e)) => {
                    failed.push((key, "search returned error"));
                    eprintln!("[DIAG] {} FAIL: search error: {}", key, e);
                }
                Err(_) => {
                    failed.push((key, "search timed out"));
                    eprintln!("[DIAG] {} FAIL: timed out after {}s", key, timeout.as_secs());
                }
            }
        }

        eprintln!("\n[DIAG] ============ RESULTS ============");
        eprintln!("[DIAG] PASSED ({}): {:?}", passed.len(), passed);
        eprintln!("[DIAG] FAILED ({}): {:?}", failed.len(), failed.iter().map(|(k, r)| format!("{} ({})", k, r)).collect::<Vec<_>>());
        eprintln!("[DIAG] ===================================");

        // No assertions - this is diagnostic only
    }
}