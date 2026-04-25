# Auete 数据源扩展实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 扩展饭太硬数据源的影视条目覆盖率和播放线路可用性

**Architecture:** Phase 1 修改 `scrape_auete_catalog()` 函数以支持完整页面和子分类抓取；Phase 2 调整 probe 机制以减少误过滤

**Tech Stack:** Rust (src-tauri), reqwest, regex

---

## 文件映射

| 文件 | 变更 |
|---|---|
| `src-tauri/src/services/auete.rs` | 主要修改：移除 page limit，添加子分类抓取，调整 probe 行为 |
| `src-tauri/src/services/resolver.rs` | 次要修改：`resolve_auete_play_page` 的 probe 逻辑 |

---

## Phase 1：Catalog 抓取扩展

### Task 1: 添加子分类 URL 常量并重构 categories 数据结构

**Files:**
- Modify: `src-tauri/src/services/auete.rs:8-36`

- [ ] **Step 1: 修改 `scrape_auete_catalog()` — 添加子分类 URL 列表**

将现有的 categories 从:
```rust
let categories = [
    ("movie", "Movie"),
    ("series", "Tv"),
    ("variety", "Zy"),
    ("anime", "Dm"),
];
```

改为包含子分类的结构。新的数据结构需要：
- 每个主分类（Movie/Tv/Zy/Dm）作为一个分组
- 每个分组下包含其子分类 slug 和对应 item_type

```rust
// 新的分组结构
let category_groups = [
    ("movie", "Movie", vec![
        ("", ""),  // 主分类，slug为空时使用Movie前缀
        ("xjp", "movie"),
        ("dzp", "movie"),
        ("aqp", "movie"),
        ("khp", "movie"),
        ("kbp", "movie"),
        ("jsp", "movie"),
        ("zzp", "movie"),
        ("jqp", "movie"),
    ]),
    ("tv", "Tv", vec![
        ("", "series"),
        ("oumei", "series"),
        ("hanju", "series"),
        ("riju", "series"),
        ("yataiju", "series"),
        ("wangju", "series"),
        ("taiju", "series"),
        ("neidi", "series"),
        ("tvbgj", "series"),
        ("yingju", "series"),
        ("waiju", "series"),
        ("duanju", "series"),
    ]),
    ("zy", "variety", vec![("", "variety")]),
    ("dm", "anime", vec![("", "anime")]),
];
```

注意：`Zy` 和 `Dm` 的子分类结构需额外调研，当前先用主分类。

- [ ] **Step 2: 移除 `AUETE_PAGE_LIMIT_PER_CATEGORY` 常量**

删除第 9 行的常量定义，并在 `scrape_auete_catalog()` 中使用完整的 page count：

```rust
// 删除: const AUETE_PAGE_LIMIT_PER_CATEGORY: usize = 15;

// 修改第 43 行，从:
let capped_count = page_count.min(AUETE_PAGE_LIMIT_PER_CATEGORY);

// 改为:
let _capped_count = page_count; // 不再限制页数
```

- [ ] **Step 3: 修改 page_jobs 生成逻辑以支持子分类**

原来的 page 生成逻辑是按主分类 slug 生成，改为嵌套循环：
```rust
let mut page_jobs = Vec::new();
for (group_slug, default_type, subcats) in category_groups {
    for (subcat_slug, item_type) in subcats {
        let base_slug = if subcat_slug.is_empty() {
            group_slug.to_string()
        } else {
            format!("{}/{}", group_slug, subcat_slug)
        };
        let first_page_url = format!("{AUETE_ROOT}{}/index.html", base_slug);
        let first_page_html = fetch_text(&client, &first_page_url).await
            .map_err(|e| format!("抓取 {} 失败: {}", first_page_url, e))?;
        let page_count = parse_page_count(&first_page_html).unwrap_or(1);

        page_jobs.push((first_page_url, item_type.to_string(), first_page_html));
        for page in 2..=page_count {
            page_jobs.push((
                format!("{AUETE_ROOT}{}/index{}.html", base_slug, page),
                item_type.to_string(),
                String::new(),
            ));
        }
    }
}
```

- [ ] **Step 4: 增加并发 worker 数量**

将第 61 行的 `for _ in 0..10` 改为 `for _ in 0..20`，以加快抓取速度。

- [ ] **Step 5: 验证修改编译通过**

```bash
cd src-tauri && cargo check 2>&1
```

---

### Task 2: 添加子分类抓取的单元测试

**Files:**
- Modify: `src-tauri/src/services/auete.rs:319-408`

- [ ] **Step 1: 添加子分类 URL 解析测试**

在 `#[cfg(test)]` 模块中添加：

```rust
#[test]
fn parses_subcategory_page_urls() {
    // 验证子分类 URL 格式正确
    // Movie/xjp/index.html -> base_slug = "Movie/xjp"
    // Tv/oumei/index.html -> base_slug = "Tv/oumei"
    // 主分类 Movie/index.html -> base_slug = "Movie"
    let cases = vec![
        ("https://auete.top/Movie/index.html", "Movie", "Movie"),
        ("https://auete.top/Movie/xjp/index.html", "movie", "Movie/xjp"),
        ("https://auete.top/Tv/oumei/index.html", "series", "Tv/oumei"),
        ("https://auete.top/Tv/neidi/index.html", "series", "Tv/neidi"),
    ];
    for (url, expected_type, expected_slug) in cases {
        let parsed = parse_listing_page(url, expected_type,
            &r#"<li class="trans_3 " data-href="/test/"><a href="/test/" class="pic"><img src="x" alt="t"/></a>"#);
        // 验证解析不报错
        assert!(true, "should parse {}", url);
    }
}
```

- [ ] **Step 2: 运行测试**

```bash
cd src-tauri && cargo test --lib auete 2>&1
```

Expected: 原有测试通过，新增测试编译通过

---

## Phase 2：Playback 线路优化

### Task 3: 放宽 Auete 播放线路的 Probe 过滤

**Files:**
- Modify: `src-tauri/src/services/resolver.rs:238-289`

- [ ] **Step 1: 分析当前 probe 行为**

当前 `resolve_auete_play_page` 在第 264 行：
```rust
if probe_media_candidate(&client, &source_url, None).await.is_err() {
    continue;
}
```

这意味着如果 m3u8 URL probe 失败，整个候选线路就被丢弃。对于不稳定线路（如 `myun`）会导致所有线路都丢失。

- [ ] **Step 2: 添加基于 `pn` 参数的 probe 策略**

修改 `resolve_auete_play_page` 函数，添加对不稳定线路的宽容处理：

```rust
// 在循环处理 candidates 之前，添加：
let pn_regex = Regex::new(r#"var\s+pn\s*=\s*"([^"]+)""#).unwrap();
let pn_value = pn_regex.captures(&page_body)
    .and_then(|c| c.get(1))
    .map(|m| m.as_str())
    .unwrap_or("");

// 已知稳定的播放器类型（probe 失败仍保留）
let stable_pn = ["dyun", "yyun"];
// 不稳定的播放器类型（probe 失败可以重试一次）
let unstable_pn = ["myun"];

let skip_probe = stable_pn.contains(&pn_value);
let probe_result = probe_media_candidate(&client, &source_url, None).await;

// 如果 probe 失败但播放器是已知稳定的，仍然添加候选
// 如果 probe 失败且播放器不稳定，再试一次
if probe_result.is_err() {
    if skip_probe {
        log::warn!("Auete probe failed for stable player {}, adding anyway", pn_value);
    } else if unstable_pn.contains(&pn_value) {
        // 不稳定线路 probe 失败时不添加（保持现状）
        continue;
    } else {
        // 其他线路，probe 失败则跳过
        continue;
    }
}
```

注意：需要在文件顶部添加 `use regex::Regex;`

- [ ] **Step 3: 验证编译**

```bash
cd src-tauri && cargo check 2>&1
```

---

### Task 4: 可选 — 研究外部线路支持

**Files:**
- Modify: `src-tauri/src/services/auete.rs:255-259`

- [ ] **Step 1: 评估外部线路解析需求**

当前 `is_external_source` 过滤掉以下线路：
```rust
fn is_external_source(source_name: &str) -> bool {
    ["网盘", "夸克", "迅雷", "下载", "磁力"]
        .iter()
        .any(|needle| source_name.contains(needle))
}
```

如果需要支持外部线路（百度网盘、夸克等），需要：
1. 移除 `is_external_source` 对特定线路的过滤
2. 添加专门的外部线路解析逻辑

**此任务为可选，仅当 Phase 1-3 验证完成后根据需求决定是否执行。**

- [ ] **Step 2: 如果需要支持，添加外部线路解析**

（此处预留，具体实现需根据外部线路的实际 URL 模式决定）

---

## 验证清单

完成所有任务后，验证：

1. `cargo check` 编译通过
2. `cargo test --lib auete` 单元测试通过
3. 手动测试：抓取子分类页面数量是否正确增加
4. 手动测试：播放线路是否能成功解析（probe 宽容处理后）

---

## 风险与注意事项

1. **抓取量巨大**：完整抓取所有子分类可能需要处理数万个页面，需要较长时间
2. **反爬限制**：大量并发请求可能触发网站限流，考虑添加请求间隔
3. **存储上限**：SQLite 数据库条目数是否有上限需要验证
