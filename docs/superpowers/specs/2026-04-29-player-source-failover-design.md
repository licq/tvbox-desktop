# 播放页同集多源自动切换设计

## 背景

播放页当前已经有自动切换下一线路的代码，但状态层级混在一起：

- `sources` 表示某个播放地址解析出的候选播放 URL。
- `currentUnifiedEpisode.sources` 表示同一集在多个播放源中的条目。
- HLS fatal error、video error、解析失败、provider play 失败分别在不同函数里推进下一步。
- 剧集模式右侧抽屉只展示选集，不展示当前集的多个播放源，用户无法手动切换。

这导致一个源不可用时，自动切换不稳定；用户也看不到当前集还有哪些源、哪些失败、哪个正在播放。

## 目标

1. 同一集有多个播放源时，优先保证播放不中断。
2. 采用平衡的不可用判定，避免把自动播放限制误判为坏源。
3. 支持按需探测：只探测用户当前要播放的那一集。
4. 支持本次运行内存健康缓存，近期成功源优先，近期失败源自动跳过。
5. 提供手动切换本集播放源的 UI。
6. 页面 title 显示当前视频名称和集数。

## 非目标

- 不在本阶段做持久化健康缓存。
- 不重构 Rust 后端 unified playback runtime。
- 不提前批量探测详情页全部剧集。
- 不保证第三方源全部可播，只保证失败后能清晰推进和展示。

## 推荐方案

在前端引入播放源编排层，集中管理当前集的播放尝试顺序、状态、失败原因和内存健康缓存。后端接口保持不变，继续复用：

- `resolve_playback`
- `provider_play`
- `fetch_hls_manifest`

`PlayerPage.vue` 不再直接在每个错误分支里推断下一步，而是把错误事件交给编排层，由编排层决定下一个候选是：

- 当前源解析出的下一个 `PlaybackCandidate`
- 同一集的下一个 `UnifiedEpisodeSource`
- 结束并显示“该集所有源均不可用”

## 状态模型

### EpisodePlaybackSession

代表一次播放某一集的运行时会话。

字段：

- `episode`: 当前 `UnifiedEpisode`
- `sourceAttempts`: 当前集下每个源的尝试状态
- `activeSourceIndex`: 当前播放源索引
- `activeCandidateIndex`: 当前源下的候选播放 URL 索引
- `status`: `idle | resolving | playing | failed`
- `lastError`: 最近失败原因

### SourceAttempt

代表同一集下某个播放源的运行时状态。

字段：

- `source`: `UnifiedEpisodeSource`
- `status`: `idle | resolving | playable | playing | failed | skipped`
- `candidates`: 当前源解析出的 `PlaybackCandidate[]`
- `failedCandidateIndexes`: 当前源失败过的候选索引
- `failureReason`: 失败原因
- `lastTriedAt`: 最近尝试时间

### InMemoryPlaybackHealth

只在本次应用运行期间有效，应用重启后清空。

建议 key：

- 源级 key：`sourceKey + episode.play_url`
- 候选级 key：`candidate.url + headers/referer hash`

值：

- `status`: `success | failed`
- `reason`: 失败原因
- `checkedAt`: 时间戳

排序策略：

1. 近期成功源优先。
2. 未知源按原始顺序。
3. 近期明确失败源自动尝试时靠后或跳过。
4. 用户手动点击失败源时允许重试。

## 播放流程

### 用户点击剧集

1. 创建新的 `EpisodePlaybackSession`。
2. 根据内存健康缓存排序当前集所有源。
3. 选择第一个可尝试源。
4. 调用现有解析流程：
   - 普通 catalog episode 使用 `resolve_playback`。
   - provider 入口先调用 `provider_play`，再按 target 类型决定直接播放或进入 `resolve_playback`。
5. 得到 `PlaybackCandidate[]` 后播放第一个候选。

### 候选失败

1. 标记当前 candidate 失败。
2. 如果同源还有下一个候选，继续尝试同源候选。
3. 如果同源没有候选，标记该源失败。
4. 选择下一个源并重复解析/播放。
5. 如果所有源均失败，显示“该集所有播放源均不可用”并保留失败状态供用户手动重试。

### 手动切换源

用户在右侧抽屉点击某个源时：

1. 不受自动跳过策略限制。
2. 即使该源本次失败过，也重新解析并播放。
3. 成功后更新内存健康缓存，并把该源置为当前播放源。

## 不可用判定

采用平衡策略：

- 解析失败：立即切下一个源。
- HLS manifest 加载失败：立即切下一个候选或源。
- 首个分片加载失败：立即切下一个候选或源。
- HLS fatal error：立即切。
- video `error`：立即切。
- 自动播放被系统拦截：不切源，只提示“线路已加载，点击播放开始”。
- 非自动播放限制的 `play()` 启动失败：标记当前候选失败并切换。

## UI 设计

### 页面 title

播放页顶部 title 显示当前视频名称和集数：

- 剧集：`片名 · 第03集`
- 电影：`片名` 或 `片名 · 正片`

数据来源优先级：

1. `detailStore.item.title` 和当前 `UnifiedEpisode.displayLabel`
2. route query 中的 `title` 和 `episodeLabel`
3. 已解析源 label 的兜底文本

源名不放进页面 title，避免标题过长。源名放在右侧抽屉和状态标签中。

### 右侧抽屉

剧集模式不再只显示选集，还要显示当前集的播放源：

- 上方：当前播放上下文，显示片名/集数和状态。
- 中部：选集网格，多源剧集保留 `N源` 标记。
- 下方：本集播放源列表。
- 底部：当前 URL、失败原因、手动重试/切换入口。

本集播放源列表状态：

- `当前播放`
- `待探测`
- `解析中`
- `本次失败`
- `最近成功`

电影模式继续展示 source/candidate 列表，但复用同一套状态标记。

## 组件与文件边界

新增或抽出一个前端编排模块：

- `src/utils/playbackSession.ts` 或小型 Pinia store

职责：

- 创建当前集播放会话。
- 根据内存健康缓存排序源。
- 记录每个源和候选的状态。
- 根据失败类型推进到下一个候选或源。
- 暴露手动切换源和重试 API。

`PlayerPage.vue` 职责收窄为：

- 接收路由和详情数据。
- 把 HLS/video 事件转换成失败原因。
- 调用编排模块拿到下一个可尝试 candidate。
- 更新 video/HLS 实例。
- 维护页面 title。

`PlaybackDrawer.vue` 职责：

- 渲染选集。
- 渲染本集播放源状态。
- 发出 `selectEpisode`、`switchEpisodeSource`、`retrySource` 事件。

## 错误处理

- 源解析失败时，记录具体错误并继续下一个源。
- 所有源失败时，不清空状态；保留失败列表，方便用户手动重试。
- 自动播放限制只展示提示，不进入失败缓存。
- 外部工具线路继续按现有逻辑打开系统处理，但不计作内置播放成功。
- 用户手动切换时，如果失败，展示该源失败原因并继续允许选择其他源。

## 测试计划

前端单元测试：

- 同一集多源时，近期成功源优先尝试。
- 近期失败源在自动尝试中靠后或跳过。
- 手动点击失败源会重试。
- 当前候选失败后，先切同源候选，再切下一播放源。
- 解析失败会推进到下一源。
- 自动播放阻止不会标记源失败。
- 所有源失败后保留失败状态和错误原因。
- 页面 title 在详情加载前后、切集后正确显示 `片名 · 集数`。

组件测试：

- 剧集模式显示选集网格和本集播放源列表。
- 当前播放源、解析中源、失败源、最近成功源有明确视觉状态。
- 点击源列表会触发手动切换事件。
- 底部 URL 和失败原因仍可见。

手动验证：

- 选择一个同集多源的剧集，断开第一个源后自动切到第二个源。
- HLS manifest 失败时不会卡在黑屏。
- 自动播放被拦截时不错误切源。
- 失败源能手动重试。
- 切换剧集后页面 title 更新为新集数。

## 迁移路径

1. 先抽出播放源编排模块和测试。
2. 让 `PlayerPage.vue` 使用编排模块处理剧集多源和候选切换。
3. 扩展 `PlaybackDrawer.vue` 展示本集播放源状态。
4. 增加页面 title 更新。
5. 保持后端接口和数据库不变。

