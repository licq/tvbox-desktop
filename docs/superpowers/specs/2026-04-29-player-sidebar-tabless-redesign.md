# 播放页右边栏无 Tab 设计

## 问题

当前 `PlaybackDrawer` 使用两个标签页（"线路"和"选集"）来组织内容，用户需要在标签间切换才能看到剧集列表或链路信息，操作路径长、信息密度低。

具体痛点：
- 切换剧集和切换线路是不同操作，却被放在同一级标签中
- 链路信息（URL、错误提示）在切换标签时不可见
- 选集和线路是两个独立维度，不适合用标签切换来隔离

## 目标

- 移除标签页，改为上下分区布局
- **上方**：剧集网格（连续剧/综艺/动漫）或来源按钮（电影），可滚动
- **下方**：链路信息面板，固定底部始终可见
- URL 支持点击复制，方便调试
- 错误信息在底部面板中显示，不遮挡上方剧集操作

## 布局

```
┌─────────────────────────────┐
│ PlaybackHeader              │
│ 当前名称 / 状态标签          │
├─────────────────────────────┤
│                             │
│ EpisodeSection              │
│ ┌───┬───┬───┬───┬───┬───┐  │
│ │ 1 │ 2 │ 3 │ 4 │ 5 │ 6 │  │  ← 剧集网格 / 电影来源按钮
│ ├───┼───┼───┼───┼───┼───┤  │
│ │ 7 │ 8 │...│   │   │   │  │
│ └───┴───┴───┴───┴───┴───┘  │
│      (可滚动区域)           │
├─────────────────────────────┤
│ LinkInfoPanel (fixed bottom)│
│ [线路1] [线路2] [线路3]     │ ← 线路切换按钮
│ https://...                 │ ← URL（点击复制）
│ ⚠ 错误信息                  │
└─────────────────────────────┘
```

## 组件架构

```
PlaybackDrawer.vue
├── PlaybackHeader        ← 当前剧集/来源名称 + 状态标签
├── EpisodeSection        ← 剧集网格 / 来源按钮（可滚动）
└── LinkInfoPanel         ← 链路信息面板（固定底部）
    ├── LineSwitcher      ← 线路切换按钮行
    ├── UrlDisplay        ← 当前 URL（点击复制）
    └── ErrorDisplay      ← 错误信息
```

### PlaybackDrawer (容器组件)

**Props**（聚合 PlayerPage 传入的数据）：

| Prop | 类型 | 说明 |
|------|------|------|
| `sources` | `PlayerSource[]` | 当前播放候选来源 |
| `currentSourceIndex` | `number` | 当前选中来源索引 |
| `failedSourceIndexes` | `number[]` | 播放失败的来源索引 |
| `status` | `string` | 播放器状态文本 |
| `errorMessage` | `string \| null` | 播放错误信息 |
| `unifiedEpisodes` | `UnifiedEpisode[]` | 归一化剧集列表 |
| `currentNormalizedIndex` | `number \| undefined` | 当前播放剧集索引 |
| `itemType` | `string` | 内容类型（movie/series/variety/anime） |

**判断逻辑**：
- `itemType === 'movie'` → EpisodeSection 渲染来源按钮列表
- `itemType` 为 series/variety/anime → EpisodeSection 渲染剧集网格

**Emits**：
- `select-episode(index: number)` → 切换到指定剧集
- `switch-line(index: number)` → 切换到指定线路

### PlaybackHeader

显示当前播放上下文：
- 连续剧模式：`正在播放: 第3集`
- 电影模式：`播放线路选择`
- 右侧显示当前来源的类型标签（HLS / Direct / Embed），复用 `SourceBadge`

### EpisodeSection

根据 `itemType` 渲染不同内容：

**连续剧/综艺/动漫模式**：
- 6 列网格，`gap: 0.5rem`
- 每个剧集渲染为正方形按钮
- 当前播放的剧集：accent 背景色高亮
- 多来源的剧集：右上角显示 `N源` badge
- 播放失败的剧集：半透明 + 删除线
- 最大高度 320px，超出纵向滚动

**电影模式**：
- 垂直列表，每行一个来源按钮
- 显示来源名称 + 类型标签
- 当前播放的来源：accent 背景色高亮
- 播放失败的来源：红色边框标记

### LinkInfoPanel

固定底部面板，始终可见。

**LineSwitcher**：
- 横向排列的线路切换按钮
- 当前线路：accent 背景色高亮
- 失败线路：danger 背景色，显示为红色
- 解析中线路：显示旋转加载动画
- 线路数量自适应，flex-wrap 换行

**UrlDisplay**：
- 单行显示，`font-family: monospace`
- 超长部分 CSS `text-overflow: ellipsis` 截断
- 点击整个区域复制 URL 到剪贴板
- 复制后显示 `✓ 已复制` 提示（2 秒后恢复）
- 复制前后边框/背景色变化以提供视觉反馈
- Hover 时高亮提示可点击

**ErrorDisplay**：
- 仅在 `errorMessage` 非空时显示
- 浅红色背景，红色文字
- 支持多行错误信息

## 数据流

```
PlayerPage.vue
  │
  ├─ unifiedEpisodes ──────► PlaybackDrawer ──► EpisodeSection
  ├─ sources ──────────────► PlaybackDrawer ──► EpisodeSection (电影)
  │                                           └─► LinkInfoPanel → LineSwitcher
  ├─ currentSourceIndex ───► PlaybackDrawer ──► LinkInfoPanel
  ├─ failedIndexes ────────► PlaybackDrawer ──► LinkInfoPanel
  ├─ status/text ──────────► PlaybackDrawer ──► PlaybackHeader
  └─ errorMessage ─────────► PlaybackDrawer ──► LinkInfoPanel → ErrorDisplay

  ↑ select-episode(index)    ──► PlayerPage.playUnifiedEpisode()
  ↑ switch-line(index)       ──► PlayerPage.handleSourceSelect()
```

## 交互行为

| 操作 | 行为 |
|------|------|
| 点击剧集 | 切换到该剧集的第一个可用线路 |
| 点击来源按钮（电影） | 切换到该来源 |
| 点击线路按钮 | 在当前剧集/来源下切换线路 |
| 播放失败 | 自动尝试下一条线路，失败按钮变红 |
| 点击 URL | 复制完整 URL 到剪贴板，2 秒反馈 |
| 剧集滚动 | 仅上半部分滚动，底部链路信息固定 |
| 剧集大量 | 可滚动区域，无分页，自然滚动 |

## 样式要点

- 边栏宽度维持现有 `360px`
- 剧集网格复用现有 `EpisodeChip` 组件的样式
- SourceBadge 复用现有组件
- 连接上下两区的分割线使用 `var(--stroke)` 颜色
- 可滚动区域使用 `overflow-y: auto`，保留系统滚动条风格
- 固定底部区域用 `border-top` 分割，背景色使用 `var(--bg-primary)` 以形成视觉区分

## 错误处理

- 所有线路均失败时：底部面板显示醒目错误提示，线路按钮全部标红
- 解析中：当前线路按钮显示旋转加载动画，其余线路按钮半透明禁用
- 网络错误：在 ErrorDisplay 中显示具体错误信息，线路按钮可供重试
- URL 复制失败（Clipboard API 不支持）：静默失败，不影响播放

## 实现范围

仅修改 `PlaybackDrawer.vue` 一个文件，不涉及：
- PlayerPage.vue — 只需调整传入的 props（新增 `itemType`）
- 后端/服务层 — 无改动
- 路由 — 无改动
- 类型定义 — 无新增类型

## 测试

- 剧集模式渲染正确（6 列网格，高亮当前剧集）
- 电影模式渲染正确（垂直来源列表）
- 线路切换按钮渲染正确（高亮当前，标红失败）
- URL 点击复制功能
- 错误信息显示/隐藏
- 大量剧集时内部滚动正常
- 底部面板固定不滚动
