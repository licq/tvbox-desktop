use crate::services::provider::{VideoProvider, ProviderError};

/// Test keyword for each scraper - hardcoded per source
pub struct TestKeyword(&'static str);

impl TestKeyword {
    pub fn for_provider(provider_key: &str) -> &'static str {
        match provider_key {
            "xb6v" | "auete" | "zxzj" | "jianpian" | "wencai" | "libvio"
            | "YGP" | "抠搜" | "UC" | "原创" | "苹果" | "糯米"
            | "白白" | "厂长" | "溢彩" | "比特" | "低端" | "萌米"
            | "兄弟" | "热播" | "欢视" | "Dm84" | "Ysj" | "Anime1"
            | "YpanSo" | "xzso" | "米搜" | "夸搜" | "Aliso" | "易搜"
            | "Bili" | "Biliych" | "fan" | "cc" => "功夫",
            _ => "test",
        }
    }
}

/// Check if URL format is valid (http/https/magnet/guard://)
pub fn is_url_valid(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("magnet:")
        || url.starts_with("guard://")
        || url.starts_with("/")
}

pub async fn test_scraper(
    provider: &dyn VideoProvider,
    provider_key: &str,
    keyword: &str,
) -> Result<(), String> {
    println!("\n=== Testing provider: {} ({}) ===", provider.source_name(), provider_key);

    // Stage 1: search
    println!("[1/3] Running search('{}')...", keyword);
    let search_results = provider.search(keyword).await.map_err(|e| format!("search failed: {}", e))?;

    if search_results.is_empty() {
        return Err(format!("  FAIL: search returned 0 items").to_string());
    }

    let first_item = &search_results[0];
    if first_item.title.is_empty() {
        return Err(format!("  FAIL: first item has empty title").to_string());
    }

    println!("  OK: got {} items, first: '{}'", search_results.len(), first_item.title);
    let source_item_key = &first_item.source_item_key;

    // Stage 2: detail
    println!("[2/3] Running detail('{}')...", source_item_key);
    let detail_result = provider.detail(source_item_key).await.map_err(|e| format!("detail failed: {}", e))?;

    let detail_item = match detail_result {
        Some(item) => item,
        None => return Err(format!("  FAIL: detail returned None").to_string()),
    };

    if detail_item.episodes.is_empty() {
        return Err(format!("  FAIL: detail episodes is empty").to_string());
    }

    let first_episode = &detail_item.episodes[0];
    if first_episode.play_url.is_empty() {
        return Err(format!("  FAIL: first episode has empty play_url").to_string());
    }

    println!("  OK: got {} episodes, first play_url: '{}...'",
        detail_item.episodes.len(),
        &first_episode.play_url[..first_episode.play_url.len().min(60)]
    );
    let play_url = &first_episode.play_url;

    // Stage 3: play
    println!("[3/3] Running play('flag', '{}...')...", &play_url[..play_url.len().min(40)]);
    let flag = "play";
    let play_targets = provider.play(flag, play_url).await.map_err(|e| format!("play failed: {}", e))?;

    if play_targets.is_empty() {
        return Err(format!("  FAIL: play returned 0 targets").to_string());
    }

    for target in &play_targets {
        if !is_url_valid(&target.target_url) {
            return Err(format!(
                "  FAIL: target_url '{}...' has invalid format",
                &target.target_url[..target.target_url.len().min(40)]
            ).to_string());
        }
    }

    println!("  OK: got {} targets, all URLs valid", play_targets.len());
    println!("=== {} PASS ===\n", provider_key);
    Ok(())
}

/// Run test for a provider by key using registry
pub async fn test_provider_by_key(
    registry: &crate::services::provider::ProviderRegistry,
    provider_key: &str,
) -> Result<(), String> {
    let provider = registry.get(provider_key)
        .ok_or_else(|| format!("provider '{}' not found", provider_key))?;

let keyword = TestKeyword::for_provider(provider_key);
    test_scraper(provider.as_ref().as_ref(), provider_key, keyword).await
}
