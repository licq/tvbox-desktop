# 影视点播分类与热门榜单设计

## 概述

对现有 TVBox 应用进行重构，实现：
1. 直播频道按名称合并，支持多源自动切换
2. 点播按豆瓣热门榜单分类展示（电影/电视剧/综艺/动漫）
3. 服务端每日爬取豆瓣排行榜，数据本地缓存

---

## 1. Tab 结构

首页 Tab 导航如下：

| Tab | 内容 |
|-----|------|
| 直播 | 电视频道**分区展示**（央视/卫视/港台/运动体育等），支持搜索 |
| 热门 | 豆瓣电影实时排行榜 Top20，与订阅源匹配后展示 |
| 电影 | 订阅源中 type=movie，最热前20，支持搜索 |
| 电视剧 | 订阅源中 type=tv，最热前20，支持搜索 |
| 综艺 | 订阅源中 type=variety，最热前20，支持搜索 |
| 动漫 | 订阅源中 type=anime，最热前20，支持搜索 |

---

## 2. 直播频道合并与多源切换

### 2.1 频道合并

多个订阅源中名称和类别相同的频道合并为一条记录。

**合并逻辑**：
- 两个频道 `name` 相同且 `category` 相同 → 视为同一频道
- 合并后保留多个播放 URL 作为候选源

**合并后的数据结构**：

```rust
struct LiveChannel {
    id: i64,                    // 合并后生成的新ID
    name: String,               // 频道名称
    logo: Option<String>,       // 频道图标
    category: String,           // 频道分类
    sources: Vec<ChannelSource>, // 多个播放源
}

struct ChannelSource {
    url: String,                // 播放地址
    subscription_id: i64,        // 来源订阅ID
}
```

### 2.3 直播分区展示

**分组逻辑**：
- 按 `category` 字段分组显示
- 固定排序优先级：央视 → 卫视 → 港台 → 运动体育 → 其他
- 过滤掉 "原创IP"、"视频源"、"手工绘画" 等非电视频道分类

**分组展示结构**：
```
📺 央视频道
  [CCTV1] [CCTV2] [CCTV13] ... (前20)
  [展开更多]

📺 卫视频道
  [湖南卫视] [浙江卫视] ... (前20)
  [展开更多]

⚽ 运动体育
  [英超直播] [篮球解说] ... (前20)
  [展开更多]

📺 其他频道
  [频道1] [频道2] ... (前20)
  [展开更多]
```

**现有数据库中的分类映射**：
- `央视频道` / `央视IPV4` → 央视分组
- `卫视频道` / `卫视IPV4` → 卫视分组
- `运动体育` → 运动体育分组（包含英超、篮球、赛车等直播）

### 2.2 播放时自动切换

**自动切换规则**：
- 当前源播放失败（网络错误、解码错误、超时）时，自动切换到下一源
- 依次尝试所有源，都失败后提示用户"所有源均不可用"

**手动切换**：
- 播放控件显示当前源序号（如"源1/3"）
- 用户可点击切换到其他源

---

## 3. 豆瓣热门榜单

### 3.1 数据来源

**目标页面**：豆瓣电影排行榜（实时）
- URL：`https://movie.douban.com/chart`
- 爬取内容：电影名称、上映年份、海报URL、评分、排名

### 3.2 爬取机制

**定时任务**：每天凌晨 3:00 执行一次爬取

**存储表设计**：

```sql
CREATE TABLE douban_hot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,          -- 电影名称
    year INTEGER,                -- 上映年份
    poster TEXT,                 -- 海报URL
    rating REAL,                 -- 豆瓣评分
    rank INTEGER,                -- 榜单排名
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_douban_name ON douban_hot(name);
```

### 3.3 匹配逻辑

将豆瓣榜单与订阅源中的 `VodItem` 进行匹配：

1. **年份提取**：从 `VodItem.name` 中正则提取 4 位数字作为年份
   - 正则：`(\d{4})` 取最后一个匹配
   - 例如 "功夫熊猫3 (2016)" → 年份 2016

2. **模糊匹配**：使用相似度算法（如 Levenshtein 距离）判断名称相似度
   - 相似度阈值：≥ 80% 视为匹配
   - 同时要求年份一致（误差 1 年内）

3. **只展示可播放**：只展示匹配成功的影视，未匹配的不显示

---

## 4. 各类型最热前20

### 4.1 热度定义

- **热门 Tab**：直接采用豆瓣榜单的排名顺序
- **其他类型 Tab（电影/电视剧/综艺/动漫）**：按订阅源返回的原始顺序

"最热前20"指的是每个 Tab 初始最多显示 20 条记录，不是额外计算的热度值。

### 4.2 前20限制

每个 Tab 初始只显示 20 条记录，点击"加载更多"展开完整列表。

---

## 5. 搜索功能

### 5.1 直播频道搜索

- 输入频道名称，实时过滤
- 支持模糊匹配

### 5.2 点播搜索

- 输入影视名称，实时过滤
- 按类型搜索时，只在当前类型内搜索
- 搜索结果不受前20限制，显示所有匹配项

---

## 6. 前端组件变更

### 6.1 Home.vue Tab 重构

将现有的「直播/点播」Tab 改为 6 个 Tab：

```vue
<template>
  <div class="flex border-b border-gray-700">
    <button
      v-for="tab in tabs"
      :key="tab.key"
      :class="['px-6 py-3 text-lg', activeTab === tab.key ? 'border-b-2 border-primary text-primary' : 'text-gray-400']"
      @click="activeTab = tab.key"
    >
      {{ tab.icon }} {{ tab.label }}
    </button>
  </div>
</template>
```

Tab 配置：

```typescript
const tabs = [
  { key: 'live', label: '直播', icon: '📺' },
  { key: 'hot', label: '热门', icon: '🔥' },
  { key: 'movie', label: '电影', icon: '🎬' },
  { key: 'tv', label: '电视剧', icon: '📺' },
  { key: 'variety', label: '综艺', icon: '🎭' },
  { key: 'anime', label: '动漫', icon: '🅰️' }
]
```

### 6.2 LiveChannel 数据结构变更

```typescript
interface LiveChannel {
  id: number
  name: string
  logo?: string
  category: string
  sources: Array<{
    url: string
    subscription_id: number
  }>
}
```

### 6.3 PlayerPage 多源切换

- `currentSourceIndex`：当前播放源的索引
- 播放失败时自动 `currentSourceIndex++` 并重新加载
- 播放控件添加源切换按钮

---

## 7. 后端变更

### 7.1 Rust 模块

| 模块 | 职责 |
|------|------|
| `services/douban.rs` | 爬取豆瓣排行榜，解析HTML，存入SQLite |
| `services/parser.rs` | 解析订阅源（已有），新增频道合并逻辑 |
| `commands/live.rs` | 新增获取合并后频道列表的API |
| `commands/vod.rs` | 新增按类型获取+搜索的API |
| `commands/douban.rs` | 新增获取热门榜单API |

### 7.2 新增 API

| API | 方法 | 描述 |
|-----|------|------|
| `/api/live/channels` | GET | 获取合并后的频道列表 |
| `/api/vod/items` | GET | 获取VOD列表，支持 `?type=movie&search=xxx` |
| `/api/douban/hot` | GET | 获取豆瓣热门榜单（已匹配） |
| `/api/douban/fetch` | POST | 手动触发豆瓣数据爬取 |

---

## 8. 数据库 Schema 变更

### 8.1 新增表

```sql
-- 豆瓣热门榜单
CREATE TABLE douban_hot (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    year INTEGER,
    poster TEXT,
    rating REAL,
    rank INTEGER,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_douban_name ON douban_hot(name);
```

### 8.2 现有表变更

`live_channels` 和 `vod_items` 表结构不变，合并逻辑在查询时处理。

---

## 9. 实现优先级

1. **Phase 1**：Rust 豆瓣爬虫 + 数据库存储
2. **Phase 2**：频道合并 + 多源切换
3. **Phase 3**：前端 6 Tab 重构
4. **Phase 4**：热门榜单展示 + 匹配逻辑
5. **Phase 5**：搜索功能增强

---

## 10. 风险与注意事项

1. **豆瓣反爬**：需要设置合理的 User-Agent 和请求间隔
2. **页面结构变化**：豆瓣页面结构可能变化，需监控爬虫成功率
3. **名称匹配误差**：模糊匹配可能产生误匹配，年份作为辅助校验
4. **冷启动**：首次运行时豆瓣数据为空，需等待凌晨爬取或手动触发
