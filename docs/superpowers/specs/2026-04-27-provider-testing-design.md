# Provider Testing Design

## 概述

为每一个 native scraper provider 编写完整的集成测试，验证三个核心能力：搜索、详情获取、播放链接解析。

## 验证标准

每个 provider 必须通过三个递进阶段的测试：

| 阶段 | 方法 | 验证标准 |
|------|------|---------|
| 1 | `search(keyword)` | 返回 items.len() > 0，每个 item.title 非空 |
| 2 | `detail(ids)` | 返回 Some，episodes.len() > 0，每个 episode.play_url 非空 |
| 3 | `play(flag, play_url)` | 返回 targets.len() > 0，target_url 格式合法（http/https/magnet/guard://） |

## 测试流程

```
test_scraper(provider_key, keyword):
  1. 调用 provider.search(keyword)
     - 必须返回非空 items
     - 记录第一个 item 的 source_item_key

  2. 调用 provider.detail(source_item_key)
     - 必须返回 Some
     - 必须有 episodes
     - 记录 episodes[0].play_url

  3. 调用 provider.play(flag, play_url)
     - 必须返回 targets
     - 每个 target.target_url 格式合法
```

任意步骤失败打印诊断信息，不影响后续测试。

## 测试隔离

- 默认 `#[ignore]`，需要环境变量激活：
  - `PROVIDER_TEST_LIVE=1 cargo test` — 运行所有
  - `PROVIDER_KEY=xb6v PROVIDER_TEST_LIVE=1 cargo test` — 仅特定 provider
- 使用 `#[tokio::test]` 异步执行
- 每个测试独立创建 provider 实例

## 文件结构

```
src-tauri/src/services/provider/
├── scraper_tests.rs          # 共享测试模块（test_scraper 函数）
└── [scraper]_scraper.rs       # 各 scraper 的 mod tests 调用共享模块
```

`scraper_tests.rs` 导出：
- `async fn test_scraper(provider_key, keyword)` — 主流程
- `fn assert_url_valid(url) -> bool` — URL 格式验证

## 各 scraper 测试命名

每个 scraper 文件内的测试模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const TEST_KEYWORD: &str = "功夫";

    #[tokio::test]
    #[ignore]
    async fn test_search_then_detail_then_play() {
        test_scraper("xb6v", TEST_KEYWORD).await;
    }
}
```

## 支持的 32 个 Provider

xb6v, auete, zxzj, jianpian, wencai, libvio, YGP, 抠搜, UC, 原创, 苹果, 糯米, 白白, 厂长, 溢彩, 比特, 低端, 萌米, 兄弟, 热播, 欢视, Dm84, Ysj, Anime1, YpanSo, xzso, 米搜, 夸搜, Aliso, 易搜, Bili, Biliych, fan, cc

## 实现顺序

1. 创建 `scraper_tests.rs` 共享模块
2. 为每个 scraper 添加 `mod tests` 调用共享模块
3. 验证设计：运行 `PROVIDER_TEST_LIVE=1 cargo test` 确认测试框架工作

## 预期结果

- 所有 provider 测试通过（或在网络不可用时优雅跳过）
- 测试输出清晰标识每个 provider 的成功/失败阶段