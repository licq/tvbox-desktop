# 详情页重新设计规范

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 重新设计详情页，上半部分展示豆瓣风格的详细信息，下半部分展示所有搜索线路及结果

**Architecture:** 三栏布局 - 左侧海报、中间元数据、右侧剧情简介，下方展示所有可用播放源

**Tech Stack:** Vue 3 + Pinia + Tauri 2.x

---

## 布局结构

```
+----------+------------------------+--------------------+
|          |                        |                    |
|  海报    |  标题 (H1)             |                    |
|          |  ★ 9.5 (50万人评价)    |                    |
|  Poster  |  导演 / 编剧 / 主演...  |    剧情简介        |
|  180x260 |  类型 / 国家 / 语言...  |    Summary text    |
|          |  上映日期 / 片长        |    here...         |
|          |                        |                    |
+----------+------------------------+--------------------+
|                                                        |
|  Sources Section (所有线路，展开显示)                  |
|  +--------------------------------------------------+ |
|  | 线路名称 (数量)     [结果1] [结果2] [结果3] ...   | |
|  +--------------------------------------------------+ |
|                                                        |
+--------------------------------------------------------+
```

---

## 详细设计

### 1. 上半部分：三栏元数据展示

**左侧 — 海报 (180x260)**
- 优先使用传入的 `poster` prop，其次使用 `meta.poster`
- 如果都没有，显示占位符（标题文字 + 背景色）

**中间 — 元数据**
- 标题 (H1, 1.75rem, 白色)
- 评分 (★ 9.5 格式，带评价人数)
- 列表数据：导演、编剧、主演（最多显示5个 + "..."）、类型、国家/地区、语言、上映日期、片长

**右侧 — 剧情简介**
- 标题 "剧情简介"
- 剧情内容文字
- 如果没有简介则不显示此区域

### 2. 下半部分：播放源列表

**展示逻辑：**
- 对每个 source 调用搜索 API
- 有结果的 source 显示：source_name + 结果数量 + 可点击结果
- 有错误但无结果的 source 显示：错误信息
- 0 结果的 source 完全隐藏（不显示）

**搜索结果项：**
- 显示 poster、title
- 点击后直接跳转到播放器（使用 `result.play_url`）
- 不再跳转到 detail 页

**加载状态：**
- 显示 skeleton loading
- 每个 source 独立加载状态

---

## 数据流

### 从豆瓣热播入口进入 (isFromDouban = true)
1. 从路由获取 `itemId` 作为 `douban_id`
2. 调用 `fetch_douban_metadata_by_id` 获取元数据
3. 使用 `meta.title` 调用 `search_vod_sources` 搜索播放源
4. 按 source 分组显示结果

### 从普通目录项进入 (isFromDouban = false)
1. 从路由获取 `itemId` 作为 catalog ID
2. 调用 `detailStore.fetchDetail` 获取目录详情
3. 调用 `fetch_douban_subject_metadata` 获取豆瓣元数据
4. 显示目录项自带的 episodes + 豆瓣元数据

---

## 组件变更

### DoubanMetaPanel.vue
- 改为三栏布局 (poster | meta | summary)
- 新增 `summary` 区域显示剧情简介
- props 保持不变

### VodDetail.vue
- 保留 DoubanMetaPanel 显示上半部分
- 下半部分：
  - 使用 `EpisodeGroupSkeleton` 显示加载状态
  - 有结果时显示 source name + count + result cards
  - 有错误时显示错误信息
  - 0 结果的 source 隐藏

---

## API 变更

### search_vod_sources 返回值
需要确保每个 result 包含：
- `play_url`: 播放地址（用于直接播放）
- `title`: 结果标题
- `poster`: 海报图片
- `source_name`: 来源名称

### handleSearchResultPlay
```typescript
function handleSearchResultPlay(result: SearchResult) {
  // 直接跳转到播放器，使用 result.play_url
  router.push(`/player/vod/${result.detail_url}?episode=${encodeURIComponent(result.play_url)}`)
}
```

---

## 错误处理

- 每个 source 独立显示错误状态
- 错误显示为红色提示，不阻止其他 source 正常显示
- 0 结果的 source 完全隐藏

---

## 验收标准

1. ✅ 从豆瓣热播点击进入详情页，显示海报、标题、评分、导演、编剧、主演、类型、国家、语言、上映日期、片长、剧情简介
2. ✅ 从普通目录项进入，显示目录自带的 episodes 和豆瓣元数据
3. ✅ 下方显示所有搜索到的 source，每个 source 标注结果数量
4. ✅ 有错误的 source 显示错误信息，不显示 0 结果的 source
5. ✅ 点击搜索结果直接进入播放页面
6. ✅ 加载状态显示 skeleton