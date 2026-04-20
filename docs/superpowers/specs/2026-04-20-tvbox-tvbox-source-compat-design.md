# TVBox 配置兼容与桌面影音库重构设计

## 概述

本次重构的目标不是继续扩展现有“简单直播/点播列表播放器”，而是将应用升级为一个可直接兼容 `饭太硬 / TVBox` 类配置源的桌面影音库。

当前应用存在两类核心问题：

1. 数据模型过于简化。现有实现假设订阅源会直接返回 `lives/vods` 和最终可播放 URL，这与 `TVBox` 生态中常见的单仓配置、站点规则、二次解析链路不匹配。
2. 前端信息架构不足。首页、详情页、播放器页都缺少来源状态、线路状态、错误反馈和继续播放等关键桌面影音库能力，导致“播不出来”时既难定位，也难操作。

因此，本设计将系统重构为三层：

1. `TVBox 配置兼容层`
2. `播放解析层`
3. `桌面影音库 UI`

## 目标与非目标

### 目标

- 直接接入 `饭太硬 / TVBox` 类单仓配置源。
- 支持从配置中提取直播源、站点源、搜索入口、解析器和扩展规则引用。
- 将“点播选集 URL”升级为“可解析播放任务”，为直链、HLS、二次解析和外部规则执行结果预留统一入口。
- 将现有首页重构为桌面影音库风格，强调继续观看、更新内容、来源状态和快速进入播放。
- 将详情页和播放器页改造为可诊断、可切换来源、可恢复播放的桌面体验。

### 非目标

- 本阶段不追求完整复刻所有 `TVBox/影视仓` 壳子的全部协议与 UI。
- 本阶段不承诺兼容所有第三方脚本规则执行环境。
- 本阶段不实现云同步、账号体系、弹幕、下载管理等扩展能力。

## 当前问题总结

### 点播播放链路问题

- 前端直接将 `episode.url` 传给播放器，默认假设它已经是最终媒体地址。
- 播放器仅对 `.m3u8` 做 `hls.js` 特判，其他地址直接交给 `<video>`，无法处理常见的站点详情页、解析页或需要额外请求头的地址。
- 播放失败后缺少明确的错误态、来源状态和人工回退操作。
- 详情页和播放器路由未标准化使用 Vue Router 参数，靠手动拆解 URL，维护成本高。

### 界面与交互问题

- 首页以 Tab 堆叠内容，缺少真正的内容层级与任务优先级。
- 视觉系统薄弱，几乎没有统一的桌面应用样式语言。
- 详情页信息少，选集区只有基础按钮网格，不适合多线路、多来源、多集数内容。
- 播放器页缺少解析状态、线路切换、错误提示和最近播放恢复。

## 方案对比

### 方案 A：最小兼容

- 在现有结构上补充对部分 `TVBox` 配置字段的解析。
- 尽量把能落地成 `lives/vods` 的内容导入现有库表。
- 继续沿用当前播放模型。

优点：

- 改动小，交付快。

缺点：

- 无法稳定支撑 `饭太硬` 这类配置源。
- 复杂点播源兼容能力仍然不足。
- UI 重构价值有限，因为底层模型仍然不匹配。

### 方案 B：配置兼容层 + 解析层 + 新 UI

- 引入 `TVBox` 配置兼容层，区分简单源和 `TVBox` 源。
- 引入统一播放解析层，将点播播放升级为解析任务。
- 重构首页、详情页、播放器页为桌面影音库风格。

优点：

- 能解决当前“点播播不出来”的核心原因。
- 前后端模型和 UI 同步升级，方向一致。
- 可以逐步扩展兼容能力，而不是一次性做成巨型重构。

缺点：

- 工作量中等，需要新增中间抽象层。

### 方案 C：完整 TVBox 内核化

- 尽量完整复刻 `TVBox/影视仓` 生态的配置模型、站点模型和规则执行。

优点：

- 理论兼容性最高。

缺点：

- 范围过大，不适合当前项目阶段。
- 容易在规则兼容问题上无限扩张。

### 结论

采用方案 B。

## 总体架构

### 架构分层

```text
┌───────────────────────────────────────────────┐
│ Desktop Media Library UI                      │
│ Home / Search / Detail / Player / Sources     │
└───────────────────────┬───────────────────────┘
                        │
┌───────────────────────▼───────────────────────┐
│ Playback Resolution Layer                     │
│ Play Task / Resolver / Candidate / Fallback   │
└───────────────────────┬───────────────────────┘
                        │
┌───────────────────────▼───────────────────────┐
│ Source Compatibility Layer                    │
│ Simple JSON / TVBox Single-Warehouse Config   │
└───────────────────────┬───────────────────────┘
                        │
┌───────────────────────▼───────────────────────┐
│ Persistence + Fetching                        │
│ SQLite / HTTP / Cache / Refresh               │
└───────────────────────────────────────────────┘
```

### 模块职责

- `source_registry`
  - 管理订阅源类型与元数据。
- `tvbox_config_parser`
  - 解析 `TVBox` 单仓配置中的站点、直播、解析器、扩展配置。
- `catalog_service`
  - 将解析后的内容整理为应用内可展示目录。
- `playback_resolver`
  - 将用户点击的直播或选集项目转成解析任务，并生成候选播放地址。
- `playback_session`
  - 管理当前会话、当前线路、错误状态、自动回退与历史保存。
- `ui_shell`
  - 承载首页、详情页、播放器页和订阅页的桌面影音库布局。

## 数据模型设计

### 订阅源类型

新增统一订阅源模型：

```ts
type SourceKind = 'simple_json' | 'tvbox_config'

interface SourceSubscription {
  id: number
  name: string
  url: string
  kind: SourceKind
  enabled: boolean
  lastRefreshedAt?: string
  lastError?: string
}
```

### TVBox 配置缓存

新增本地缓存表，用于保存配置解析结果：

```sql
CREATE TABLE source_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,
    config_kind TEXT NOT NULL,
    raw_content TEXT NOT NULL,
    parsed_at TEXT NOT NULL,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);
```

### 站点与直播元数据

新增站点和直播表，将单仓配置拆成可查询结构：

```sql
CREATE TABLE source_sites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,
    site_key TEXT NOT NULL,
    site_name TEXT NOT NULL,
    api TEXT,
    ext TEXT,
    searchable INTEGER NOT NULL DEFAULT 1,
    quick_search INTEGER NOT NULL DEFAULT 0,
    filterable INTEGER NOT NULL DEFAULT 0,
    source_type TEXT NOT NULL,
    raw_json TEXT NOT NULL,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);

CREATE TABLE source_lives (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,
    group_name TEXT,
    channel_name TEXT NOT NULL,
    raw_url TEXT NOT NULL,
    normalized_url TEXT,
    raw_json TEXT,
    FOREIGN KEY (subscription_id) REFERENCES subscriptions(id) ON DELETE CASCADE
);
```

### 内容目录与选集

将当前 `vod_items` 拆成“内容实体 + 剧集项 + 来源”：

```sql
CREATE TABLE catalog_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subscription_id INTEGER NOT NULL,
    site_id INTEGER,
    source_item_key TEXT,
    title TEXT NOT NULL,
    item_type TEXT NOT NULL,
    poster TEXT,
    summary TEXT,
    detail_json TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE catalog_episodes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    catalog_item_id INTEGER NOT NULL,
    source_name TEXT,
    season_label TEXT,
    episode_label TEXT NOT NULL,
    play_url TEXT NOT NULL,
    order_index INTEGER NOT NULL DEFAULT 0,
    extra_json TEXT,
    FOREIGN KEY (catalog_item_id) REFERENCES catalog_items(id) ON DELETE CASCADE
);
```

### 播放任务与解析结果缓存

```sql
CREATE TABLE playback_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    episode_id INTEGER,
    raw_input TEXT NOT NULL,
    resolved_url TEXT,
    resolver_kind TEXT,
    headers_json TEXT,
    status TEXT NOT NULL,
    error_message TEXT,
    verified_at TEXT NOT NULL
);
```

状态值：

- `pending`
- `resolving`
- `ready`
- `failed`
- `external_required`

## 配置兼容层设计

### 输入类型

兼容层需要处理两类输入：

1. 现有简单 JSON 源
2. `TVBox` 单仓配置源

### TVBox 配置解析范围

本阶段重点支持以下字段族：

- 直播配置
- 站点配置
- 搜索入口
- 解析器列表
- 扩展配置引用

如果某些字段无法直接使用，应完整保留原始 JSON 以便后续扩展，不在第一阶段直接丢弃。

### 解析流程

1. 拉取订阅源原始内容。
2. 判断属于简单 JSON 还是 `TVBox` 配置。
3. 若为 `TVBox` 配置：
   - 解析站点列表。
   - 解析直播列表。
   - 解析解析器列表。
   - 保留扩展字段和原始配置。
4. 将可结构化内容写入缓存表和目录表。
5. 将无法立即结构化的内容以 `raw_json` 形式保留。

### 容错策略

- 订阅刷新失败时，不清空旧缓存，保留上一次成功快照。
- 局部字段解析失败时记录错误，但不中断整个订阅刷新。
- 对不认识的站点类型、脚本类型和配置字段做保留式兼容。

## 播放解析层设计

### 核心抽象

播放器不再直接消费字符串 URL，而是消费 `PlayTask`：

```ts
interface PlayTask {
  id: string
  contentType: 'live' | 'vod'
  input: string
  sourceId?: number
  episodeId?: number
  resolverHints?: string[]
}
```

解析结果统一为：

```ts
interface ResolvedPlayback {
  status: 'ready' | 'failed' | 'external_required'
  candidates: Array<{
    url: string
    label: string
    headers?: Record<string, string>
    kind: 'hls' | 'http' | 'external'
  }>
  errorMessage?: string
}
```

### 解析顺序

1. 判断是否为可直接播放的媒体地址。
2. 判断是否为 HLS。
3. 判断是否需要走已知解析器。
4. 判断是否需要外部规则或外部处理。
5. 生成候选线路列表并返回给播放器会话。

### 播放器行为

- 默认尝试首选候选线路。
- 致命错误时自动切换下一候选。
- 切换失败后展示统一错误面板。
- 用户可手动切换线路、重试解析、查看原始地址。

### 错误可视化

播放器页必须区分以下失败类型：

- 配置解析失败
- 播放地址解析失败
- 媒体加载失败
- 所有候选线路失效

## 前端信息架构

### 首页

首页改为桌面影音库结构，而不是单纯 Tab 容器。

#### 顶部区域

- 全局搜索框
- 当前订阅状态
- 最近刷新时间
- 最近失败源提示
- 设置与订阅入口

#### 主内容区

- 继续观看
- 最新加入
- 热门推荐
- 最近更新剧集

#### 次导航

- 直播
- 电影
- 剧集
- 综艺
- 动漫
- 来源

### 直播页

- 采用分组列表 + 频道卡片。
- 支持按分类和关键词过滤。
- 对每个频道展示可用线路数量和最近可用状态。

### 点播详情页

详情页重构为三段：

1. 顶部信息区
   - 海报、标题、类型、年份、来源站点、最近播放进度
2. 内容说明区
   - 剧情简介、来源标签、站点备注、更新信息
3. 选集与线路区
   - 按来源或线路分组
   - 支持排序、折叠、快速跳转最近观看

### 播放器页

播放器页包含以下面板：

- 视频区
- 线路状态条
- 解析状态与错误提示
- 线路切换区
- 节目/剧集快捷跳转

播放器必须明确显示：

- 当前来源
- 当前线路
- 当前解析状态
- 失败时的回退动作

## 视觉设计方向

视觉方向采用“桌面影音库”，不是电视大屏 UI。

### 视觉原则

- 强化内容浏览和信息层级，而不是纯卡片堆叠。
- 使用更有识别度的品牌色和统一的表面层级。
- 保持深色背景，但引入更丰富的中性色、边框、高光和状态色。
- 交互重点放在继续观看、来源切换、状态反馈和搜索效率。

### 组件基线

- 卡片分为海报卡、横向内容卡、状态卡三类。
- 页面布局使用固定头部 + 可滚动主区。
- 错误、空状态、加载状态必须形成统一组件体系。

## 路由与状态设计

### 路由调整

- 用 `useRoute()` 标准化读取路由参数。
- 点播详情与播放器页统一以实体 ID 和可选来源参数驱动。

示例：

```text
/library/:type
/detail/:itemId
/player/live/:channelId
/player/vod/:itemId?episodeId=123&source=site_a
```

### 前端状态拆分

- `sourceStore`：订阅源与刷新状态
- `libraryStore`：内容目录与分类列表
- `detailStore`：详情页数据与选集数据
- `playbackStore`：播放会话、线路、错误、进度

避免继续把所有点播行为塞进一个 `vodStore`。

## 错误处理与降级策略

### 刷新阶段

- 若订阅刷新失败，保留最近一次成功内容。
- UI 明确显示“数据为缓存快照”。

### 浏览阶段

- 某个来源无海报、无简介时用降级展示，不阻塞进入详情页。
- 某个来源选集为空时应明确标记不可播放，而不是显示空白。

### 播放阶段

- 若解析失败，展示错误类别和建议动作。
- 若候选线路全部失败，可允许打开原始链接或切换来源。
- 若外部规则尚不支持，明确标记为“当前版本不支持该解析方式”。

## 测试与验证

### 后端验证

- 为简单 JSON 与 `TVBox` 配置分别准备解析样例。
- 为配置解析器增加快照测试。
- 为播放解析层增加状态转换测试：
  - 直链成功
  - HLS 成功
  - 多候选回退
  - 解析失败
  - 外部规则要求

### 前端验证

- 首页、详情页、播放器页进行状态驱动测试：
  - 加载中
  - 空数据
  - 缓存数据
  - 解析失败
  - 多线路切换

### 人工验证

- 用实际 `饭太硬 / TVBox` 配置源做导入验证。
- 重点验证：
  - 配置能否被识别为 `tvbox_config`
  - 直播列表是否可展示
  - 站点目录是否能生成
  - 点播详情是否能形成选集
  - 播放失败时是否有明确错误与回退路径

## 分阶段实施建议

### 第一阶段：兼容层落地

- 引入 `SourceKind`
- 完成 `TVBox` 配置识别与解析缓存
- 建立站点、直播、目录、选集的新表结构

### 第二阶段：点播播放链路重构

- 引入 `PlayTask` 和 `ResolvedPlayback`
- 重写播放器状态与错误反馈
- 重构详情页选集与来源展示

### 第三阶段：桌面影音库 UI 重构

- 重做首页信息架构
- 重做卡片体系、空状态、状态提示
- 完成来源页和刷新状态可视化

## 风险与边界

- `TVBox` 配置生态并不统一，字段和规则可能存在多种变体。
- 复杂脚本规则和外部解析能力无法在本阶段一次性覆盖。
- 若直接依赖第三方站点解析，长期稳定性受外部站点变化影响。

本设计的核心边界是：

- 先把应用升级为“能理解并承载 `TVBox` 配置”的桌面壳。
- 兼容能力按抽象层逐步增强，而不是一开始承诺完整生态兼容。

## 结论

项目应从“简单聚合播放器”升级为“兼容 `TVBox` 配置的桌面影音库”。核心不在于先换一套视觉皮肤，而在于先修正订阅模型与播放模型，再让 UI 真实反映来源、线路、解析状态和继续观看等桌面场景需求。

采用“配置兼容层 + 播放解析层 + 新 UI”的方案，能够同时解决：

- `饭太硬 / TVBox` 源接入不匹配
- 点播视频播放不稳定
- 界面不具备桌面媒体中心体验
