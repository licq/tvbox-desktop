# VodDetail 搜索结果展示重构设计

## 背景

VodDetail 页面处理三种数据入口：豆瓣热门直进、搜索入口、正常目录。当前设计中电影结果以"版本"按钮展示、剧集结果需要点击展开后才显示集数，两种模式交互路径不一致，信息隐藏过多。

## 目标

- 统一电影与剧集在搜索结果中的展示方式
- 消除不必要的折叠/展开交互，关键信息直接可见
- 保持与现有媒体中心设计语言一致

## 非目标

- 不重写 DoubanMetaPanel（顶部元数据区）
- 不改写后端接口或数据模型
- 不引入新的依赖库

## 组件架构

### 现有组件变动

| 组件 | 处理方式 |
|------|----------|
| `EpisodeGroupPanel.vue` | **重写**。统一处理电影和剧集的展示，不再区分 `isMovie` 的渲染分支，而是采用同一套卡片结构 |
| `VodDetail.vue` | **精简**。搜索结果区和剧集源区使用同一组件，移除 `dedup-search-card` 的内联样式 |
| `DoubanMetaPanel.vue` | **不变**。顶部元数据区保持现状，仅同步响应式调整 |
| `EpisodeChip.vue` | **复用**。剧集的集数 chip 继续复用 |

### 统一卡片的数据接口

```ts
interface UnifiedSourceCardProps {
  sourceName: string
  itemType: 'movie' | 'series' | 'variety' | 'anime'
  episodes: CatalogEpisode[]
  onPlay: (episode: CatalogEpisode) => void
}
```

### 组件树（VodDetail 内部）

```
VodDetail
├── DoubanMetaPanel (不变)
├── SourceListSection (新)
│   └── UnifiedSourceCard × N
│       ├── CardHeader (源名 + 类型标签)
│       └── CardBody
│           ├── MovieBody: PlayButton[]
│           └── SeriesBody: EpisodeChip[] (+ optional expand)
```

## 视觉设计

### 卡片结构

```
┌─────────────────────────────────────────────┐
│  来源A                              剧集 ▲  │  ← header
├─────────────────────────────────────────────┤
│                                             │
│  ▶ 来源A-HD     ▶ 来源B-1080P              │  ← movie body
│                                             │
├─────────────────────────────────────────────┤
│                                             │
│  01   02   03   04   05   06   07   08     │  ← series body
│  09   10   11   12  ··· 剩余 24 集 ▼        │
│                                             │
└─────────────────────────────────────────────┘
```

### Header 区

- **源名称**：左对齐，字号 `0.9rem`，字重 `600`，颜色 `white/85`
- **类型标签**：右对齐，小字 `0.65rem`，颜色 `white/30`，显示"电影/剧集/综艺/动漫"
- **分隔线**：header 与 body 之间保留 `1px` 分隔线 `white/6`
- **背景**：header `white/3`，卡片整体渐变背景保持现有风格

### Movie Body（电影）

- 横向 flex-wrap，gap `0.4rem`
- 播放按钮样式：
  - 圆角 `0.5rem`，padding `0.4rem 0.9rem`
  - 背景 `white/4`，边框 `1px solid white/8`
  - 左侧有 `▶` 播放图标，颜色 `teal-300/70`
  - 文字：`来源名` + 可选质量标签（如 `-HD`）
  - hover：背景 `teal-400/12`，边框 `teal-400/25`，文字 `teal-100`
- 如果同一来源有多个版本，显示为多个独立按钮

### Series Body（剧集）

- chip 网格直接展示，不再折叠
- 默认显示 **前 24 集**
- 超过 24 集时，底部显示展开按钮：
  - 文字："展开剩余 X 集"
  - 样式同现有的 `episodes-collapsed-button`，但改为实心边框
  - 展开后显示全部，同时出现"收起"按钮

## 响应式规则

使用现有的 Tailwind 断点：

| 场景 | 卡片宽度 | 剧集 chip 每行数量 | 电影按钮布局 |
|------|----------|-------------------|-------------|
| ≥1280px (桌面) | 跟随容器 100% | 12 个 | 横向排列 |
| 1024-1279px (平板横屏) | 100% | 10 个 | 横向排列 |
| 768-1023px (平板竖屏) | 100% | 8 个 | 横向排列 |
| <768px (手机) | 100% | 6 个 | 允许换行 |

### DoubanMetaPanel 响应式同步调整

```css
.douban-meta-panel {
  display: grid;
  gap: 1.5rem;
  grid-template-columns: 220px 1fr 300px;
}

@media (max-width: 1023px) {
  .douban-meta-panel {
    grid-template-columns: 160px 1fr;
  }
  .douban-meta-summary {
    grid-column: 1 / -1;
    border-left: none;
    border-top: 1px solid white/10;
    padding-left: 0;
    padding-top: 1rem;
  }
}

@media (max-width: 639px) {
  .douban-meta-panel {
    grid-template-columns: 1fr;
  }
  .douban-meta-poster {
    width: 140px;
    margin: 0 auto;
  }
}
```

## 数据流

```
VodDetail (统一源列表数据)
├── computed: unifiedSources → UnifiedSourceCardProps[]
│   (从 dedupSearchItems + providerEpisodes + detailStore.episodeGroups 统一映射)
│
└── UnifiedSourceCard
    ├── props: sourceName, itemType, episodes, onPlay
    └── internal state: expanded (仅剧集且 >24 集时)
```

`UnifiedSourceCard` 是纯展示组件，无外部数据获取，便于测试。

## 错误处理

- **源级错误**：单个源的 provider_detail 失败，显示红色提示在该卡片底部，不影响其他源
- **空状态**：某个源返回 0 个可播放项，卡片 body 显示 "暂无可用播放地址"
- **全局错误**：保持现有的 `searchError` / `providerDetailError` 顶部提示

## 动画与过渡

| 场景 | 效果 |
|------|------|
| 卡片 hover | `translateY(-2px)` + 边框亮度提升，duration `200ms` |
| 剧集展开/收起 | `grid-template-rows` 动画或 height transition，duration `250ms ease-out` |
| 播放按钮 hover | 背景色渐变 + border-color 变化，duration `180ms` |
| 页面加载 skeleton | 保持现有 pulse 动画 |

## 测试策略

- `UnifiedSourceCard`：vitest + jsdom 测试渲染逻辑
  - 电影：渲染正确数量的播放按钮
  - 剧集 ≤24 集：直接渲染全部 chips，无展开按钮
  - 剧集 >24 集：渲染 24 个 chips + "展开剩余 X 集"按钮
  - 展开后：渲染全部 chips + "收起"按钮

## 边界情况

- **集数恰好为 24**：不显示展开按钮
- **单集剧集**：显示 1 个 chip，无展开按钮
- **0 集**：显示空状态文案
- **电影多版本**：每个版本一个独立按钮，横向排列换行

## 变更文件清单

| 文件 | 操作 |
|------|------|
| `src/components/detail/EpisodeGroupPanel.vue` | 重写 |
| `src/views/VodDetail.vue` | 修改（精简搜索结果渲染、统一使用 EpisodeGroupPanel） |
| `src/components/detail/DoubanMetaPanel.vue` | 修改（仅响应式 CSS） |
| `src/components/detail/EpisodeGroupSkeleton.vue` | 可能调整以匹配新结构 |

## 弃用

- `EpisodeGroupPanel.vue` 中的 `episodesExpanded` ref 及折叠按钮逻辑被移除
- `VodDetail.vue` 中的 `dedup-search-card` 内联样式和模板块被移除
