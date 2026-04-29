# 剧集去重合并展示设计

## 问题

同一部剧在多个源中都有40集时，界面会出现80个播放按钮（每个源40个）。不同源的集数标签格式也可能不一致，如 `第1集` 与 `第01集`，视觉上更混乱。用户的核心痛点是**找集数困难**。

## 目标

- 剧集类型内容（series/variety/anime）的集数列表按**归一化编号合并**，40集只展示40个按钮
- 电影（movie）保持现有行为，每个 `episode_label` 独立展示（画质/线路标识有意义）
- 用户点击合并按钮后，系统**自动选择最佳源并播放**，失败时静默切换
- 该逻辑同时作用于 `VodDetail` 详情页和 `PlaybackDrawer` 播放抽屉的选集 tab

## 归一化规则

提取集数编号用于判断是否为同一集：

| 原始标签 | 归一化结果 | 说明 |
|---------|-----------|------|
| `第1集` / `第1期` | `1` | 中文集数/期数 |
| `第01集` / `第01期` | `1` | 前导零忽略 |
| `01` | `1` | 纯数字 |
| `S01E01` / `E01` | `1` | 季集格式 |
| `HD` / `1080P` / `蓝光` | — | 不匹配任何模式，视为独立标识 |

**判定逻辑**：
1. 若标签匹配任意归一化模式，提取数字作为 `normalizedIndex`
2. 若 `normalizedIndex` 相同，则视为同一集进行合并
3. 若标签不匹配任何归一化模式（如 `HD`），则保持原样

### 归一化函数实现

```ts
function extractEpisodeIndex(label: string): number | null {
  const trimmed = label.trim()

  // 匹配 "第1集"、"第01集"、"第1期"、"第01期"
  const chineseMatch = trimmed.match(/第\s*(\d+)\s*[集期]/)
  if (chineseMatch) return parseInt(chineseMatch[1], 10)

  // 匹配 "S01E01"、"E01"
  const seasonMatch = trimmed.match(/S\d+E(\d+)/i)
  if (seasonMatch) return parseInt(seasonMatch[1], 10)

  const epMatch = trimmed.match(/^E(\d+)$/i)
  if (epMatch) return parseInt(epMatch[1], 10)

  // 匹配纯数字 "01"、"1"
  const pureNum = trimmed.match(/^(\d+)$/)
  if (pureNum) return parseInt(pureNum[1], 10)

  return null
}
```

## 数据结构

### UnifiedEpisode

```ts
export interface UnifiedEpisode {
  normalizedIndex: number
  displayLabel: string
  sources: UnifiedEpisodeSource[]
}

export interface UnifiedEpisodeSource {
  sourceKey: string
  sourceName: string
  episode: CatalogEpisode
}
```

### 合并函数

```ts
function mergeEpisodes(
  groups: CatalogEpisodeGroup[],
  itemType: CatalogItemType
): UnifiedEpisode[] {
  if (itemType === 'movie') {
    // 电影不做合并，每个 episode 独立
    return groups.flatMap(g =>
      g.episodes.map(ep => ({
        normalizedIndex: ep.id,
        displayLabel: ep.episode_label,
        sources: [{
          sourceKey: g.source_name,
          sourceName: g.source_name,
          episode: ep,
        }],
      }))
    )
  }

  // 剧集：按 normalizedIndex 合并
  const map = new Map<number, UnifiedEpisode>()

  for (const group of groups) {
    for (const ep of group.episodes) {
      const idx = extractEpisodeIndex(ep.episode_label)
      if (idx === null) {
        // 无法归一化，作为独立项
        map.set(ep.id, {
          normalizedIndex: ep.id,
          displayLabel: ep.episode_label,
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            episode: ep,
          }],
        })
        continue
      }

      const existing = map.get(idx)
      if (existing) {
        existing.sources.push({
          sourceKey: group.source_name,
          sourceName: group.source_name,
          episode: ep,
        })
      } else {
        map.set(idx, {
          normalizedIndex: idx,
          displayLabel: formatDisplayLabel(ep.episode_label),
          sources: [{
            sourceKey: group.source_name,
            sourceName: group.source_name,
            episode: ep,
          }],
        })
      }
    }
  }

  return Array.from(map.values()).sort((a, b) => a.normalizedIndex - b.normalizedIndex)
}
```

### 展示标签格式化

归一化后统一展示格式：

```ts
function formatDisplayLabel(original: string, itemType?: CatalogItemType): string {
  const idx = extractEpisodeIndex(original)
  if (idx === null) return original
  const unit = itemType === 'variety' ? '期' : '集'
  return `第${idx}${unit}`
}
```

## UI 组件变更

### 1. EpisodeGroupPanel → UnifiedEpisodePanel（新组件）

替换原有的 `EpisodeGroupPanel`，改为按合并后的集数展示：

- **剧集**：`episode-chip-grid` 中每个 chip 代表一个 `UnifiedEpisode`
  - chip 显示 `displayLabel`
  - chip 右侧显示源数徽章（如 `2源`），仅当 `sources.length > 1` 时显示
  - 点击后触发 `play` 事件，携带整个 `UnifiedEpisode`
- **电影**：保持现有行为，每个 `episode_label` 一个 `play-button`

### 2. PlaybackDrawer 选集 tab

- 接收 `unifiedEpisodes: UnifiedEpisode[]` 替代 `episodes: CatalogEpisode[]`
- 剧集：网格展示合并后的按钮，带源数徽章
- 电影：保持现有行为

### 3. 源数徽章样式

```css
.source-count-badge {
  font-size: 0.6rem;
  background: rgba(160, 120, 200, 0.2);
  color: rgba(220, 200, 245, 0.9);
  padding: 0.05rem 0.3rem;
  border-radius: 0.2rem;
  margin-left: 0.3rem;
}
```

## 播放行为：自动源切换

### 源优先级排序

每个 `UnifiedEpisode.sources` 按以下规则排序：

1. `kind === 'hls'` 优先（流畅度最好）
2. `kind === 'http'` 次之
3. `external` / `embed` 排最后

排序依据 `PlaybackCandidate.kind`，在 `playbackStore.resolve()` 返回的候选中已有该字段。

### 点击播放流程

```
用户点击 UnifiedEpisode
  → 按优先级排序 sources
  → 尝试播放第1个 source
    → 成功：正常播放
    → 失败：自动切换到下一个 source
      → 循环直到成功或所有源失败
        → 所有源失败：显示错误 "该集所有线路均不可用"
```

### VodDetail 中的处理

`handlePlay(unifiedEpisode: UnifiedEpisode)`：

1. 按优先级排序 `unifiedEpisode.sources`
2. 逐个尝试播放（复用现有 `playbackStore.resolve` + `initVodPlayback` 逻辑）
3. 首次成功即停止，失败则自动尝试下一个
4. 全部失败时显示全局错误提示

### PlayerPage 中的处理

`switchToEpisode(unifiedEpisode: UnifiedEpisode)`：

1. 按优先级排序
2. 优先走 catalog 流程（`itemId > 0`）
3. 其次是 provider 流程（`provider_play`）
4. 自动切换逻辑同上

## 边缘情况处理

### 归一化失败

若某个 `episode_label` 无法提取编号（如 `HD`、`预告片`），视为独立项，不参与合并，单独展示。

### 来源缺失某些集

合并后 `UnifiedEpisode.sources` 可能只有1个或0个源：
- 1个源：不显示源数徽章
- 0个源：理论上不会出现，因为每个 episode 至少来自一个源

### 集数不连续

某些源可能只提供部分集数（如前20集）。合并后只展示有源的集数，不补全缺失集数。

### 排序

合并结果始终按 `normalizedIndex` 升序排列，保证集数顺序正确。

## 电影的特殊处理

电影不做任何合并：
- `VodDetail` 中每个源的 episode 独立展示（现有 `play-button-row` 行为）
- `PlaybackDrawer` 中电影不显示选集 tab（或保持现有行为）

判断依据：`item_type === 'movie'`。

## 范围说明

### 本迭代内（核心）

以下场景的所有源数据已加载完毕，可直接合并：

- **`VodDetail` 普通目录流**：`detailStore.episodeGroups` 已包含所有源
- **`PlaybackDrawer` 目录流**：`activeGroup` 来自 `detailStore.episodeGroups`

### 本迭代外（后续）

- **`SearchResultCard` 搜索/Douban 流**：各源按需懒加载，需先 eager-load 全部源才能合并，交互改动较大，另作规划

## 影响范围

| 文件 | 变更 |
|------|------|
| `src/types/index.ts` | 新增 `UnifiedEpisode`、`UnifiedEpisodeSource` 类型 |
| `src/utils/episode.ts`（新） | 新增 `extractEpisodeIndex`、`mergeEpisodes`、`formatDisplayLabel` |
| `src/components/detail/EpisodeGroupPanel.vue` | 改造为使用 `UnifiedEpisode`，剧集显示合并按钮+徽章 |
| `src/components/player/PlaybackDrawer.vue` | 选集 tab 接收 `UnifiedEpisode[]`，去重展示 |
| `src/views/VodDetail.vue` | `handlePlay` 改为接收 `UnifiedEpisode`，实现自动源切换 |
| `src/views/PlayerPage.vue` | `switchToEpisode` 改为接收 `UnifiedEpisode`，实现自动源切换 |
