# Source Line Name Differentiation — Design

## Overview

当同一来源有多个播放链接（如 YPanSo 的"高清"、"标清"）时，目前全部显示相同的名称，无法区分。方案：提取 `player_aaaa` JSON 中的 `name` 字段，通过 `meta` 传递到前端，在播放源面板和控制栏显示。

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|----------|
| 面板显示 | 用线路名完全替换 sourceName | 更简洁有意义 |
| 控制栏显示 | 显示当前线路名 | 帮助用户确认当前线路 |
| 数据来源 | `player_aaaa.name` 字段 | 已有数据，无需额外请求 |
| 降级 | 无 `name` 时回退到 sourceKey | 保持向后兼容 |

---

## Behavior

### 播放源面板（PlaybackDrawer）

**当前（显示 sourceName）：**
```
本集播放源
YPanSo               本次失败
YPanSo               待探测
```

**方案 B 后（用 meta/线路名替换 sourceName）：**
```
本集播放源
高清                 本次失败
标清                 待探测
```

当 `meta` 存在且非空时，显示 `meta` 而非 `sourceName`。

### 控制栏

控制栏右侧添加当前线路名标签（如"高清"），在 playback drawer 关闭时仍能看到当前线路。

---

## Data Flow

```
player_aaaa JSON: {"url": "...", "name": "高清线路"}
                         ↓
extract_ypanso_player_url() — 提取 name 字段
                         ↓
PlaybackTarget { meta: "高清线路", ... }
                         ↓
play() — 返回 PlaybackTarget
                         ↓
Frontend — 通过 meta 字段获取
                         ↓
PlaybackDrawer — 显示 meta 而非 sourceName
PlayerPage — 控制栏显示 meta
```

---

## Implementation

### Backend Changes

**`src-tauri/src/services/provider/ypanso_scraper.rs`**

修改 `extract_ypanso_player_url` 函数，同时返回 `url` 和 `name`：

```rust
// 返回 (video_url, line_name)
fn extract_ypanso_player_url(body: &str) -> Option<(String, Option<String>)> {
    let player_regex = Regex::new(r"(?s)player_[a-z]{4}\s*=\s*(\{.*?\})</script>").ok()?;
    player_regex.captures(body).and_then(|captures| {
        let json_str = captures.get(1).map(|m| m.as_str())?;
        let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
        let url = parsed.get("url").and_then(|v| v.as_str()).map(|s| s.to_string())?;
        let name = parsed.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        Some((url, name))
    })
}
```

**`play()` 方法** — 修改返回结构：

```rust
pub async fn play(&self, flag: &str, play_url: &str) -> Result<Vec<PlaybackTarget>, ProviderError> {
    let body = self.base.fetch_text(play_url).await?;
    let (video_url, line_name) = extract_ypanso_player_url(&body)
        .unwrap_or_else(|| (play_url.to_string(), None));

    Ok(vec![PlaybackTarget {
        episode_id: None,
        source_key: "YpanSo".to_string(),
        target_url: video_url,
        target_kind: PlaybackTargetKind::Direct,
        resolver_key: None,
        headers: None,
        sort_hint: 0,
        meta: line_name,  // 现在使用提取的线路名
        referer: Some(play_url.to_string()),
    }])
}
```

### Frontend Changes

**`src/types/index.ts`** — `UnifiedEpisodeSource` 添加可选 `lineName`：

```ts
export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  lineName?: string  // 新增：线路名（如"高清"、"标清"）
  episode: CatalogEpisode
}
```

**`src/views/PlayerPage.vue`** — 控制栏显示当前线路名：

在控制栏右侧添加：
```vue
<span v-if="currentLineName" class="line-name-badge">{{ currentLineName }}</span>
```

**`src/components/player/PlaybackDrawer.vue`** — 面板显示线路名：

```vue
<!-- 在 episode source list 中 -->
<span class="source-row-label">
  {{ attempt.source.lineName || attempt.source.sourceName }}
</span>
```

---

## Files to Modify

| File | Change |
|------|--------|
| `src-tauri/src/services/provider/ypanso_scraper.rs` | 改 `extract_ypanso_player_url` 返回 `(url, name)`，修改 `play()` |
| `src/types/index.ts` | `UnifiedEpisodeSource` 添加 `lineName?` |
| `src/views/PlayerPage.vue` | 添加 `currentLineName` 计算属性，控制栏显示 |
| `src/components/player/PlaybackDrawer.vue` | 使用 `lineName \|\| sourceName` |

---

## Out of Scope

- 其他数据源的线路名提取（仅 YPanSo）
- 自动选择最优线路的逻辑
- 线路名持久化