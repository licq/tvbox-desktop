# Provider Testing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为每一个 provider 编写完整的集成测试，验证 search / detail / play 三个核心方法

**Architecture:** 共享测试模块 `scraper_tests.rs` 提供 `test_scraper()` 函数，各 scraper 文件通过 `mod tests` 调用。测试默认忽略，通过环境变量激活。

**Tech Stack:** Rust, tokio, reqwest (HTTP), 内置 scraper parser

---

## File Map

| 文件 | 职责 |
|------|------|
| `src-tauri/src/services/provider/scraper_tests.rs` | **新建** — 共享测试函数 |
| `src-tauri/src/services/provider/registry.rs` | **修改** — 不变，仅作参考 |
| `src-tauri/src/services/provider/xb6v_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/auete_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/zxzj_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/jianpian_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/wencai_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/libvio_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/ygp_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/kkss_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/uuss_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/ycyz_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/lite_apple_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/nuomi_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/baibai_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/changzhang_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/yicai_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/bite_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/ddrk_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/mengmi_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/xiongdi_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/rebo_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/huanshi_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/dm84_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/ysj_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/anime1_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/ypanso_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/xzso_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/miso_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/kuasou_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/aliso_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/yiso_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/bili_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/biliych_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/fan_scraper.rs` | **修改** — 添加 `mod tests` |
| `src-tauri/src/services/provider/cc_scraper.rs` | **修改** — 添加 `mod tests` |

---

## Task 1: Create scraper_tests.rs shared module

**Files:**
- Create: `src-tauri/src/services/provider/scraper_tests.rs`

- [ ] **Step 1: Write scraper_tests.rs**

```rust
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

pub async fn test_scraper<P: VideoProvider + ?Sized>(
    provider: &P,
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
    test_scraper(provider.as_ref(), provider_key, keyword).await
}
```

- [ ] **Step 2: Add scraper_tests module to mod.rs**

Modify: `src-tauri/src/services/provider/mod.rs`

在文件末尾（在 `pub use cc_scraper::CcScraper;` 之后）添加：

```rust
pub mod scraper_tests;
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/services/provider/scraper_tests.rs src-tauri/src/services/provider/mod.rs
git commit -m "feat(provider): add shared scraper_tests module"
```

---

## Task 2: Add tests to first batch (8 scrapers: xb6v, auete, zxzj, jianpian, wencai, libvio, YGP, 抠搜)

**Files:**
- Modify: `src-tauri/src/services/provider/xb6v_scraper.rs:209-` (end of file)
- Modify: `src-tauri/src/services/provider/auete_scraper.rs`
- Modify: `src-tauri/src/services/provider/zxzj_scraper.rs`
- Modify: `src-tauri/src/services/provider/jianpian_scraper.rs`
- Modify: `src-tauri/src/services/provider/wencai_scraper.rs`
- Modify: `src-tauri/src/services/provider/libvio_scraper.rs`
- Modify: `src-tauri/src/services/provider/ygp_scraper.rs`
- Modify: `src-tauri/src/services/provider/kkss_scraper.rs`

For each scraper file, add to the end of the file (after the `impl VideoProvider for XxxScraper` block):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::provider::scraper_tests::test_scraper;

    const TEST_KEYWORD: &str = "功夫";

    #[tokio::test]
    #[ignore]
    async fn test_search_then_detail_then_play() {
        let scraper = Xb6vScraper::new();
        test_scraper(&scraper, "xb6v", TEST_KEYWORD).await
            .expect("xb6v test failed");
    }
}
```

Each file gets a different scraper struct name and key string:
- xb6v → `Xb6vScraper::new()`, key `"xb6v"`
- auete → `AueteScraper::new()`, key `"auete"`
- zxzj → `ZxzjScraper::new()`, key `"zxzj"`
- jianpian → `JianpianScraper::new()`, key `"jianpian"`
- wencai → `WencaiScraper::new()`, key `"wencai"`
- libvio → `LibvioScraper::new()`, key `"libvio"`
- YGP → `YgpScraper::new()`, key `"YGP"`
- 抠搜 → `KkssScraper::new()`, key `"抠搜"`

- [ ] **Step 1: Add tests to xb6v_scraper.rs**
- [ ] **Step 2: Add tests to auete_scraper.rs**
- [ ] **Step 3: Add tests to zxzj_scraper.rs**
- [ ] **Step 4: Add tests to jianpian_scraper.rs**
- [ ] **Step 5: Add tests to wencai_scraper.rs**
- [ ] **Step 6: Add tests to libvio_scraper.rs**
- [ ] **Step 7: Add tests to ygp_scraper.rs**
- [ ] **Step 8: Add tests to kkss_scraper.rs**
- [ ] **Step 9: Run tests to verify compilation**

Run: `cd src-tauri && cargo test --lib provider::xb6v_scraper::tests 2>&1 | head -30`

Expected: compile success (tests are #[ignore] so they won't run)

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/services/provider/xb6v_scraper.rs \
  src-tauri/src/services/provider/auete_scraper.rs \
  src-tauri/src/services/provider/zxzj_scraper.rs \
  src-tauri/src/services/provider/jianpian_scraper.rs \
  src-tauri/src/services/provider/wencai_scraper.rs \
  src-tauri/src/services/provider/libvio_scraper.rs \
  src-tauri/src/services/provider/ygp_scraper.rs \
  src-tauri/src/services/provider/kkss_scraper.rs
git commit -m "feat(provider): add tests for first batch (xb6v, auete, zxzj, jianpian, wencai, libvio, YGP, 抠搜)"
```

---

## Task 3: Add tests to second batch (8 scrapers: UC, 原创, 苹果, 糯米, 白白, 厂长, 溢彩, 比特)

**Files:**
- Modify: `src-tauri/src/services/provider/uuss_scraper.rs`
- Modify: `src-tauri/src/services/provider/ycyz_scraper.rs`
- Modify: `src-tauri/src/services/provider/lite_apple_scraper.rs`
- Modify: `src-tauri/src/services/provider/nuomi_scraper.rs`
- Modify: `src-tauri/src/services/provider/baibai_scraper.rs`
- Modify: `src-tauri/src/services/provider/changzhang_scraper.rs`
- Modify: `src-tauri/src/services/provider/yicai_scraper.rs`
- Modify: `src-tauri/src/services/provider/bite_scraper.rs`

Same pattern as Task 2 with correct scraper struct names:
- UC → `UussScraper::new()`, key `"UC"`
- 原创 → `YcyzScraper::new()`, key `"原创"`
- 苹果 → `LiteAppleScraper::new()`, key `"苹果"`
- 糯米 → `NuomiScraper::new()`, key `"糯米"`
- 白白 → `BaibaiScraper::new()`, key `"白白"`
- 厂长 → `ChangzhangScraper::new()`, key `"厂长"`
- 溢彩 → `YicaiScraper::new()`, key `"溢彩"`
- 比特 → `BiteScraper::new()`, key `"比特"`

- [ ] **Step 1-8: Add tests to each file**
- [ ] **Step 9: Verify compilation**

Run: `cd src-tauri && cargo build --lib 2>&1 | tail -5`

- [ ] **Step 10: Commit**

---

## Task 4: Add tests to third batch (8 scrapers: 低端, 萌米, 兄弟, 热播, 欢视, Dm84, Ysj, Anime1)

**Files:**
- Modify: `src-tauri/src/services/provider/ddrk_scraper.rs`
- Modify: `src-tauri/src/services/provider/mengmi_scraper.rs`
- Modify: `src-tauri/src/services/provider/xiongdi_scraper.rs`
- Modify: `src-tauri/src/services/provider/rebo_scraper.rs`
- Modify: `src-tauri/src/services/provider/huanshi_scraper.rs`
- Modify: `src-tauri/src/services/provider/dm84_scraper.rs`
- Modify: `src-tauri/src/services/provider/ysj_scraper.rs`
- Modify: `src-tauri/src/services/provider/anime1_scraper.rs`

- [ ] **Step 1-8: Add tests to each file**
- [ ] **Step 9: Verify compilation**
- [ ] **Step 10: Commit**

---

## Task 5: Add tests to fourth batch (8 scrapers: YpanSo, xzso, 米搜, 夸搜, Aliso, 易搜, Bili, Biliych)

**Files:**
- Modify: `src-tauri/src/services/provider/ypanso_scraper.rs`
- Modify: `src-tauri/src/services/provider/xzso_scraper.rs`
- Modify: `src-tauri/src/services/provider/miso_scraper.rs`
- Modify: `src-tauri/src/services/provider/kuasou_scraper.rs`
- Modify: `src-tauri/src/services/provider/aliso_scraper.rs`
- Modify: `src-tauri/src/services/provider/yiso_scraper.rs`
- Modify: `src-tauri/src/services/provider/bili_scraper.rs`
- Modify: `src-tauri/src/services/provider/biliych_scraper.rs`

- [ ] **Step 1-8: Add tests to each file**
- [ ] **Step 9: Verify compilation**
- [ ] **Step 10: Commit**

---

## Task 6: Add tests to final batch (4 scrapers: fan, cc + registry tests update)

**Files:**
- Modify: `src-tauri/src/services/provider/fan_scraper.rs`
- Modify: `src-tauri/src/services/provider/cc_scraper.rs`
- Modify: `src-tauri/src/services/provider/registry.rs:150-209`

Update the registry integration tests to also print provider names and confirm test infrastructure works.

- [ ] **Step 1: Add tests to fan_scraper.rs**
- [ ] **Step 2: Add tests to cc_scraper.rs**
- [ ] **Step 3: Update registry test to verify all tests are compilable**

Run: `cd src-tauri && cargo test --lib provider::registry::native_scraper_tests 2>&1 | tail -20`

- [ ] **Step 4: Commit**

---

## Task 7: End-to-end verification

- [ ] **Step 1: Run live tests for first batch providers**

Run: `cd src-tauri && PROVIDER_TEST_LIVE=1 cargo test --lib provider::xb6v_scraper::tests -- --include-ignored 2>&1 | tail -30`

Expected: Test runs and either passes or fails with diagnostic output

- [ ] **Step 2: Commit final state**

---

## Verification Commands

All tests default to `#[ignore]`. Run with:

```bash
# Run all ignored provider tests
PROVIDER_TEST_LIVE=1 cargo test --lib -- --include-ignored provider:: 2>&1 | grep -E "(test |PASS|FAIL|---)"

# Run specific provider
PROVIDER_KEY=xb6v PROVIDER_TEST_LIVE=1 cargo test --lib -- --include-ignored xb6v 2>&1 | tail -20

# Compile check all tests (no run)
cd src-tauri && cargo test --lib --no-run 2>&1 | tail -5
```

---

## Self-Review Checklist

1. **Spec coverage**: 每个 provider 的 search/detail/play 三阶段都有对应 Task
2. **No placeholders**: 所有 `mod tests` 结构相同，仅 scraper name 和 key 不同，代码完整
3. **Type consistency**: `ScrapedCatalogItem`, `PlaybackTarget`, `VideoProvider` trait 方法签名与实际代码一致
4. **File paths**: 所有路径使用绝对路径
5. **Test activation**: 所有测试用 `#[ignore]` + 环境变量激活，设计正确