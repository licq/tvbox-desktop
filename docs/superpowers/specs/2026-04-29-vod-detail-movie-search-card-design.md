# VodDetail 电影搜索结果卡片改进

## 背景与问题

当前 VodDetail 搜索结果页中，电影卡片右侧显示「立即播放」按钮，用户需要点击后才能展开下方的 `EpisodeGroupPanel` 看到播放链接。这个流程对电影来说体验不好：

1. 电影通常只有 1-2 条播放线路，不需要先点「立即播放」再展开
2. 点击后展开到下方面板，打破了卡片内的上下文
3. 播放按钮统一显示 "HD"，缺乏源站信息和清晰度/版本信息

## 目标

- 电影在搜索结果卡片内直接展示可播放链接，横向排列，点击即播放
- 剧集保持现有「源选择器 + EpisodeGrid」模式不变
- 播放按钮显示 `源简称 · episode_label`，信息更完整

## 方案

### 1. SearchResultCard 渲染逻辑

#### 电影（itemType === 'movie'）

右侧直接渲染所有已预加载源的播放按钮：

- 按钮横向排列，`flex-wrap: wrap`，自动换行
- 每个按钮对应一个 `episode`（即一条播放线路）
- 按钮文字：`源简称 · episode_label`
- 样式沿用现有的 `.source-btn` 风格，hover 高亮
- 未预加载完成时，显示「加载中…」骨架
- 预加载失败或空结果时，该源不显示按钮

#### 剧集（series / variety / anime）

保持现有逻辑不变：源选择器 + EpisodeGrid。

### 2. 源名称去重/简化

按钮文字渲染规则（优先级从高到低）：

1. **完全包含检测**：若 `episode_label` 已包含 `source_name` 的任意子串（如「文才HD」包含「文才」），直接显示 `episode_label`
2. **简称组合**：否则显示 `source_short + " · " + episode_label`
3. **源简称取法**：`source_name` 取前 2-4 个字符（如"文才影视"→"文才"、"Libvio"→"Libvio"）

### 3. MovieActionPanel 处理

`MovieActionPanel` 组件不再被 `SearchResultCard` 使用，但在正常详情页（非搜索场景）中可能仍有用处，暂时保留组件文件，仅移除 `SearchResultCard` 中对它的 import 和引用。

### 4. 数据流

数据流完全复用现有机制：

1. `VodDetail.vue` 在搜索结果加载完成后，为每个电影 dedup item 调用 `preloadFirstSource(item)`
2. 用户切换源时调用 `preloadSource(item, sourceKey)`
3. `SearchResultCard` 通过 `sourceDetails` prop 读取已预加载的 `ProviderDetailResult`
4. 点击播放按钮时发射 `play-episode` 事件，由 `VodDetail.vue` 中 `handleCardEpisodePlay` 处理

### 5. 错误与边界处理

| 场景 | 行为 |
|------|------|
| 某源正在预加载 | 该源区域显示小型「加载中…」提示 |
| 某源预加载失败 | 静默忽略，不显示该源按钮 |
| 某源返回 0 个 episodes | 静默忽略，不显示该源按钮 |
| 所有源都失败/为空 | 卡片右侧显示「暂无播放链接」 |

### 6. 组件变更清单

| 文件 | 变更 |
|------|------|
| `src/components/detail/SearchResultCard.vue` | 电影渲染逻辑改为直接展示 episodes 按钮；新增按钮文字去重函数 |
| `src/components/detail/MovieActionPanel.vue` | 移除（从 SearchResultCard 中取消引用），文件本身保留 |
| `src/views/VodDetail.vue` | 无需改动（数据流和事件处理已完备） |

### 7. 测试

为 `SearchResultCard` 新增 vitest 测试：

- 电影 item 直接渲染 episode 按钮（不渲染 MovieActionPanel）
- 剧集 item 保持渲染源选择器 + EpisodeGrid
- 按钮文字去重逻辑：source_name 已包含在 episode_label 中时不再重复拼接
- 预加载中状态显示 loading 提示
- 空 episodes 时不显示按钮
