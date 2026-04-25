# 豆瓣热播展示设计

## 背景

当前 TVBox 实现的问题：
- 各个页面（Movie/Series/Variety/Anime tab）只显示本地数据库的目录内容
- 缺少吸引用户的内容入口
- 用户没有快速找到热门内容的途径

用户观察到的现象：
- TVBox 页面显示快，但初始内容不吸引人
- 标准 TVBox 显示热播内容作为入口

## 设计目标

1. **显示热播入口**：在各个页面顶部显示豆瓣热播推荐
2. **可播放体验**：点击热播后能匹配到目录中的实际视频并播放
3. **热播只作为推荐**：不存储播放链接，只显示标题/封面/评分

---

## 页面布局

### 各 Tab 页面（Movie/Series/Variety/Anime 等）

```
┌────────────────────────────────────────┐
│ 🔥 热播专区  (横向滚动卡片)             │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│ │海报│ │海报│ │海报│ │海报│ │海报│    │
│ └────┘ └────┘ └────┘ └────┘ └────┘    │
│ 评分  评分  评分  评分  评分             │
└────────────────────────────────────────┘
│                                        │
│ 其他目录内容...                         │
```

热播专区固定显示在顶部（各 Tab 共享），横向滚动浏览。

### 热播详情页

```
┌────────────────────────────────────────┐
│ ← 返回                                 │
│                                        │
│ [海报]        寻龙诀 (2024)             │
│               豆瓣评分: 8.5             │
│               数据来源: 豆瓣热播        │
│                                        │
│ ┌─────────────────────────────────┐    │
│ │ 🔍 在目录中搜索                 │    │
│ └─────────────────────────────────┘    │
│                                        │
│ ─ 或 ─                                 │
│                                        │
│ 已找到 3 个播放源                       │
│ [云播D线] [云播M线] [云播Y线]           │
└────────────────────────────────────────┘
```

---

## 数据流

### 数据获取

1. 订阅刷新时（或定时任务）：
   - `DoubanCrawler.fetch_hot_list()` 抓取豆瓣热播
   - `storage.upsert_douban_hot()` 存储到数据库

2. 前端 `fetchHome()` 时：
   - `get_douban_hot()` 获取豆瓣热播列表
   - 返回 `HomePayload.douban_hot` 字段

### 热播点击流程

1. 用户点击热播卡片
2. 跳转到 `/detail/hot/{douban_id}` （热播专用详情页）
3. 热播详情页用标题模糊搜索目录：
   - `search_vod(keyword)` 搜索匹配的视频
   - 如果找到：显示匹配的 catalog 详情 + 播放源
   - 如果没找到：显示热播信息 + "当前源没有此片"

### 匹配逻辑

```rust
// 点击热播后，搜索目录
fn match_hot_to_catalog(hot: &DoubanHot, catalog: &[CatalogItem]) -> Option<CatalogItem> {
    // 标题模糊匹配（去掉括号年份、特殊字符）
    let search_title = normalize_title(&hot.name);
    catalog.iter().find(|item| {
        let item_title = normalize_title(&item.title);
        item_title.contains(&search_title) || search_title.contains(&item_title)
    })
}
```

---

## 数据库变更

### douban_hot 表（已存在）

```sql
CREATE TABLE douban_hot (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    year INTEGER,
    poster TEXT,
    rating REAL,
    rank INTEGER,
    updated_at INTEGER
);
```

### 新增字段：无

无需修改数据库结构，复用现有的 `douban_hot` 表。

---

## API 变更

### HomePayload 新增字段

```rust
pub struct HomePayload {
    pub continue_watching: Vec<HomeCatalogItem>,
    pub latest_updates: Vec<HomeCatalogItem>,
    pub featured: Vec<HomeCatalogItem>,
    pub douban_hot: Vec<DoubanHot>,  // 新增
}
```

### get_library_home 返回 douban_hot

```rust
// storage.rs:715-747
pub fn get_library_home(&self) -> SqliteResult<HomePayload> {
    // ... existing code ...
    let douban_hot = self.get_douban_hot()?;  // 新增
    Ok(HomePayload {
        continue_watching,
        latest_updates,
        featured,
        douban_hot,  // 新增
    })
}
```

---

## 前端变更

### Home.vue 热播专区

```vue
<!-- 热播专区 - 各 Tab 顶部显示 -->
<section v-if="libraryStore.doubanHot.length" class="hot-section">
  <div class="section-header">
    <span class="hot-badge">🔥</span>
    <span>豆瓣热播</span>
  </div>
  <div class="hot-carousel">
    <VodCard
      v-for="hot in libraryStore.doubanHot"
      :key="hot.id"
      :item="hot"
      :is-hot="true"
      @click="handleHotClick(hot)"
    />
  </div>
</section>
```

### 新增热播详情页

路由：`/detail/hot/:doubanId`

组件：`src/views/HotDetail.vue`

职责：
1. 显示热播基本信息（标题、封面、评分、排名）
2. 用标题搜索目录匹配的视频
3. 显示匹配的 catalog 详情或"未找到"提示

### libraryStore 新增字段

```typescript
// src/stores/library.ts
const doubanHot = ref<DoubanHot[]>([])

function applyHomePayload(payload: HomePayloadInput) {
  doubanHot.value = normalizeCards(payload.douban_hot ?? payload.doubanHot)
  // ... existing ...
}
```

### types.ts 新增类型

```typescript
interface DoubanHot {
  id: number
  name: string
  year: number | null
  poster: string | null
  rating: number | null
  rank: number
  updated_at: number
}
```

---

## 匹配算法

### 标题归一化

```typescript
function normalizeTitle(title: string): string {
  return title
    .replace(/\(\d{4}\)/g, '')  // 去除年份 (2024)
    .replace(/\[.*?\]/g, '')   // 去除括号内容
    .replace(/\s+/g, '')       // 去除空格
    .toLowerCase()
}
```

### 匹配策略

1. 完全匹配：标题完全相同
2. 模糊匹配：归一化后包含关系
3. 年份参考：匹配时优先考虑同年上映的

---

## 状态处理

| 状态 | UI 显示 |
|---|---|
| 加载中 | 骨架屏（复用 EpisodeGroupSkeleton） |
| 找到匹配 | 显示目录中的视频详情 + 播放源 |
| 未找到匹配 | 显示热播信息 + "当前源没有此片" + 搜索按钮 |
| 网络错误 | 重试按钮 + 错误提示 |

---

## 实施步骤

### Phase 1: 数据层
1. 修改 `get_library_home` 返回 `douban_hot`
2. 确认 `upsert_douban_hot` 定时刷新逻辑

### Phase 2: 前端基础
3. `HomePayload` 添加 `douban_hot` 字段
4. `libraryStore` 添加 `doubanHot` 状态
5. `Home.vue` 添加热播专区组件

### Phase 3: 热播详情页
6. 创建 `HotDetail.vue` 页面
7. 实现标题搜索匹配逻辑
8. 处理找到/未找到的 UI

### Phase 4: 优化
9. 热播卡片样式（评分标签、热度标识）
10. 横向滚动优化
11. 匹配算法优化

---

## 测试验证

```bash
# 1. 刷新订阅后检查数据库
sqlite3 tvbox.db "SELECT COUNT(*) FROM douban_hot; SELECT * FROM douban_hot LIMIT 5;"

# 2. 前端检查 HomePayload 包含 douban_hot
curl -X POST "http://localhost:1420/api/subscription/1/refresh"
curl "http://localhost:1420/api/library/home" | jq '.douban_hot'

# 3. 点击热播卡片测试跳转
# 4. 检查匹配逻辑是否正确
```

---

## 风险与注意事项

1. **豆瓣反爬**：抓取频率需要控制，避免被封
2. **匹配率**：豆瓣标题可能和目录不完全一致，需要模糊匹配
3. **数据时效**：热播数据需要定期更新（建议每天一次）
4. **缓存策略**：热播数据应该缓存，避免每次刷新都抓取