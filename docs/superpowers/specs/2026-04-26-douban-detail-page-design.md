# 详情页重新设计 — Douban 风格详情 + 全线路展示

## 背景

用户希望重新设计详情页，使其：
1. 上半部分像豆瓣一样，有详细的metadata展示（导演/编剧/主演/类型/制片国家/地区/语言/上映日期/片长/评价/剧情简介）
2. 下半部分直接列出所有线路的搜索结果，线路旁显示结果数量，点击链接直接播放
3. 不再需要之前的"推荐线路"和"按需展开"交互

## 数据现状

| 数据 | 来源 | 是否已有 |
|------|------|---------|
| title, poster, summary | `catalog_items` 表 | ✓ |
| episode_groups (play URLs) | `catalog_episodes` 表 | ✓ |
| Douban rating | `douban_hot` 表 | ✓ (但未关联到 catalog_items) |
| Douban id | `douban_hot` 表 (id 字段) | ✓ (但未关联到 catalog_items) |
| 导演/编剧/主演/类型/制片国家/语言/上映日期/片长/详情简介 | 豆瓣 subject 页面 | ✗ (需要抓取) |

`detail_json` 字段仅存储播放路由元数据（source, guard_key, url），不是 Douban 风格的元数据。

Douban subject 页面有 PoW (SHA-512 proof-of-work) 保护，直接 HTTP 请求会被阻止。Playwright/WKWebView 可以自然解决此挑战。

## 总体布局

```
┌─────────────────────────────────────────────┐
│  上半部分: DoubanMetaPanel                   │
│  ┌──────────┐  标题                          │
│  │          │  类型标签 · 年份                │
│  │  海报    │  ★ 8.2 (641642人评价)           │
│  │          │  导演: xxx  编剧: xxx           │
│  └──────────┘  主演: xxx / xxx / xxx ...    │
│               类型: 喜剧 / 奇幻 / 冒险        │
│               制片国家/地区: 美国             │
│               语言: 英语 / 汉语普通话 / 粤语   │
│               上映日期: 2022-03-11...         │
│               片长: 139分钟                   │
│               ─────────────────────────────  │
│               剧情简介:                       │
│               在美国某个普普通通的亚裔社区...   │
├─────────────────────────────────────────────┤
│  下半部分: 线路列表 (全部展开)                │
│  ┌─────────────────────────────────────────┐ │
│  │ 线路A (共12个)              [播放]     │ │
│  │ [第1集] [第2集] [第3集] ...            │ │
│  └─────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────┐ │
│  │ 线路B (共8个)               [播放]      │ │
│  │ [第1集] [第2集] ...                    │ │
│  └─────────────────────────────────────────┘ │
│  ...                                         │
└─────────────────────────────────────────────┘
```

## 数据流程

### 阶段 1: 订阅刷新时 (douban_id 关联)

```
subscription refresh → get_matched_hot_list
  → 匹配 douban_hot.id 和 catalog_item.title+year
  → 将 douban_id 存入 catalog_items 或 vod_items
```

需要: 在 `catalog_items` 表增加 `douban_id` 字段 (INTEGER, 可NULL)。

### 阶段 2: 详情页加载

```
用户进入详情页 (/detail/:itemId)
  1. fetchDetail(itemId) → 加载 catalog_items + catalog_episodes (已有)
  2. 查找 douban_id: 根据 title+year 在 douban_hot 表模糊匹配
  3. 如果找到 douban_id:
     → fetchDoubanDetail(douban_id) via WebView
     → 提取 director/writer/actors/genre/country/language/release/runtime/summary
  4. 渲染页面
```

## Douban Metadata 抓取 (WebView 方案)

### 方案选择
使用 Tauri WebView (WKWebView on macOS) 加载 Douban subject 页面，利用 WKWebView 原生执行 JavaScript 的能力自动解决 PoW 挑战。

### 实现步骤

1. **添加 WebView 权限** (`src-tauri/capabilities/main.json`):
   ```json
   {
     "permissions": [
       "core:default",
       "core:window:allow-is-fullscreen",
       "core:window:allow-set-fullscreen",
       "core:webview:allow-create-webview-window"
     ]
   }
   ```

2. **新增 Rust 命令** `fetch_douban_subject_metadata`:
   - 创建隐藏的 `WebviewWindow`
   - 导航到 `https://movie.douban.com/subject/:douban_id/`
   - 等待 DOM ready (通过 `webview.on_window_event` 或定时轮询)
   - 执行 JS 提取 `#info` 区块的 HTML 和 `#link-report` 简介
   - 解析 HTML 结构 (使用 `scraper` crate):
     - `property="v:directedBy"` → 导演
     - `property="v:writer"` → 编剧
     - `property="v:starring"` → 主演 (取前5个，剩余显示"更多...")
     - `property="v:genre"` → 类型
     - `#info` 中的文本匹配 → 制片国家/地区、语言、上映日期、片长
     - `property="v:summary"` → 剧情简介
     - `.rating_num` → 评分
   - 通过 Tauri event 将结构化数据返回
   - 关闭 webview window

3. **DoubanSubjectMeta 数据结构**:
   ```rust
   struct DoubanSubjectMeta {
       douban_id: i64,
       title: String,
       poster: Option<String>,     // 从 catalog_items 已有的 poster 或 douban_hot.poster
       rating: Option<f64>,
       rating_count: Option<i64>,
       director: Vec<String>,
       writer: Vec<String>,
       actors: Vec<String>,       // 最多显示5个
       genre: Vec<String>,
       country: Vec<String>,
       language: Vec<String>,
       release_date: Vec<String>, // 可能多个
       runtime: Option<String>,   // "139分钟"
       summary: Option<String>,
   }
   ```

4. **错误处理**:
   - WebView 创建失败 → 返回空 metadata，前端降级只显示已有信息
   - Douban 页面加载超时 (10s) → 返回空 metadata
   - Douban ID 不存在 → 返回空 metadata

5. **性能考虑**:
   - WebView 首次创建有冷启动成本，考虑复用 window 或使用单例模式
   - 可以添加 5-10s 缓存 (存 Rust 内存或 SQLite)，避免每次进入详情页都重新抓取
   - 错误时显示骨架屏，不阻塞页面渲染

## 前端组件变更

### 页面结构 (VodDetail.vue)

```
<template>
  <div class="detail-page">
    <!-- 顶部: Douban 元信息面板 -->
    <DoubanMetaPanel
      v-if="doubanMeta"
      :meta="doubanMeta"
      :poster="detailStore.item?.poster"
      class="top-zone"
    />
    <DetailMetaSkeleton v-else-if="loadingDouban" class="top-zone" />

    <!-- 底部: 全部线路列表 (始终展开) -->
    <section class="source-list">
      <SourceGroupPanel
        v-for="group in detailStore.episodeGroups"
        :key="group.source_name"
        :group="group"
        @play="handlePlay"
      />
    </section>
  </div>
</template>
```

### 新组件

#### DoubanMetaPanel.vue
- 左侧: 海报图片 (使用 catalog_items.poster 或 douban_hot.poster)
- 右侧:
  - 标题 + 年份 + 类型标签
  - 评分 badge: `★ 8.2 (641642人评价)`
  - metadata 列表: 导演 / 编剧 / 主演 / 类型 / 制片国家/地区 / 语言 / 上映日期 / 片长
  - 剧情简介 (可折叠)

#### DetailMetaSkeleton.vue
- 骨架屏，与 DoubanMetaPanel 布局一致
- 动画脉冲效果

#### SourceGroupPanel.vue (原 EpisodeGroupPanel.vue 重命名/重构)
- 始终展开，无推荐/非推荐区分
- 标题行: `{source_name} ({episodeCount}个)` + 播放按钮
- EpisodeChip 网格: 每个 chip 可点击播放

#### 删除组件
- `RecommendedSourcePanel.vue` — 不再需要
- `DetailHero.vue` — 被 DoubanMetaPanel 替代

### 路由变更
- `/detail/:itemId` 保持不变，后端逻辑不变
- 点击 EpisodeChip → 导航到 `/player/vod/:itemId?episodeId=:id`

### Store 变更 (detail.ts)

```typescript
// 新增
const doubanMeta = ref<DoubanSubjectMeta | null>(null)
const loadingDouban = ref(false)

async function fetchDetail(itemId: number) {
  // 现有逻辑...
  await detailStore.fetchDetail(itemId)
  // 同时获取 Douban metadata
  loadingDouban.value = true
  try {
    doubanMeta.value = await invoke<DoubanSubjectMeta | null>('fetch_douban_subject_metadata', {
      itemId,
      title: detailStore.item?.title,
      year: null // 可从 douban_hot 匹配得到
    })
  } catch {
    doubanMeta.value = null
  } finally {
    loadingDouban.value = false
  }
}
```

## 数据库变更

### 迁移 SQL
```sql
ALTER TABLE catalog_items ADD COLUMN douban_id INTEGER REFERENCES douban_hot(id);
```

### douban_id 关联逻辑
在 `get_matched_hot_list` 或订阅刷新时:
- 对每个 catalog_item，通过 `title` + `year` (如果有) 在 `douban_hot` 表中模糊匹配
- 匹配成功则更新 `catalog_items.douban_id`

## 文件变更清单

### 新增
- `src/components/detail/DoubanMetaPanel.vue` — Douban 风格元信息展示
- `src/components/detail/DetailMetaSkeleton.vue` — 加载骨架屏
- `src-tauri/src/commands/douban.rs` — 新增 `fetch_douban_subject_metadata` 命令
- `src-tauri/src/services/douban.rs` — 新增 `DoubanSubjectMeta` 结构和 WebView 抓取逻辑

### 修改
- `src/views/VodDetail.vue` — 新布局，整合 DoubanMetaPanel + 全线路列表
- `src/components/detail/EpisodeGroupPanel.vue` — 改为始终展开，移除推荐逻辑
- `src/stores/detail.ts` — 新增 doubanMeta state 和 fetch 逻辑
- `src-tauri/capabilities/main.json` — 添加 webview 权限
- `src-tauri/src/services/storage.rs` — 添加数据库迁移，douban_id 字段

### 删除
- `src/components/detail/RecommendedSourcePanel.vue`
- `src/components/detail/DetailHero.vue`

## 风险与限制

1. **WebView 冷启动**: 首次创建 WKWebView 有延迟，考虑添加缓存
2. **Douban 反爬策略变化**: 如果 Douban 修改 PoW 机制，可能需要更新抓取逻辑
3. **隐私**: 加载 Douban 页面会产生真实用户流量，需考虑是否需要代理
4. **移动端**: 此方案针对 macOS WebView，如果未来要支持移动端，需要单独处理

## 实现顺序

1. 数据库迁移 + douban_id 关联逻辑
2. Rust 端: 新增 `fetch_douban_subject_metadata` 命令 + WebView 抓取
3. 前端: 新增 DoubanMetaPanel + DetailMetaSkeleton 组件
4. 前端: 修改 VodDetail.vue 布局
5. 前端: 重构 EpisodeGroupPanel (始终展开)
6. 前端: 删除废弃组件
7. Store 集成: detail.ts + doubanMeta fetch
