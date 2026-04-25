# 豆瓣发现架构设计方案

## 背景

当前 TVBox 架构需要预抓取所有订阅源的 catalog 才能展示内容，导致同步时间长、存储占用大、用户看到的内容与实际受欢迎程度脱节。

新架构以豆瓣热播作为发现层，用户选视频后按需搜索播放源，无需预存全量 catalog。

## 核心流程

```
用户打开 App
    ↓
豆瓣热播列表（按 tab 类型：movie/series/variety/anime）
    ↓
用户点击视频 → 进入详情页
    ↓
并行搜索 zxzj / jpvod / xb6v
    ↓
展示可用播放源（按 item_type 分类）
    ↓
用户选源 → 抓取详情页 → 提取剧集列表
    ↓
进入播放
```

## 数据层

### 豆瓣热播表变更

```sql
ALTER TABLE douban_hot ADD COLUMN item_type TEXT DEFAULT 'movie';
```

`item_type` 可选值：`movie` | `series` | `variety` | `anime`

### 豆瓣抓取逻辑

后端新增 `fetch_all_douban_hot()`，按 4 个分类并行抓取：

| item_type | 豆瓣 API |
|---|---|
| movie | `type=movie&tag=热门` |
| series | `type=tv&tag=热门` |
| variety | `type=tv&tag=综艺` |
| anime | `type=tv&tag=动漫` |

每分类取前 30 条，合计 120 条，按 `rank` 字段写入 `douban_hot` 表。

豆瓣 JSON API URL：`https://movie.douban.com/j/search_subjects?type={type}&tag={tag}&page_limit=30&page_start=0`

返回字段映射：
- `id` → 豆瓣 subject ID（用于构建详情页 URL）
- `title` → `name`
- `cover` → `poster`
- `rate` → `rating`
- `episodes_info` → 剧集信息（辅助判断类型）

### 刷新策略

- 打开 App 时检查 `douban_hot.updated_at`，超过 24 小时后台自动刷新
- 不阻塞 UI，刷新完成后更新本地数据

## 搜索 API

### 新增命令：`search_vod_sources`

**请求**：
```typescript
interface SearchRequest {
  title: string;        // 豆瓣条目标题
  year?: number;        // 发行年份（辅助过滤）
  item_type?: string;   // 可选，按类型筛选结果
}
```

**响应**：
```typescript
interface SearchResult {
  source: 'zxzj' | 'jpvod' | 'xb6v';
  source_name: string;       // 源显示名
  detail_url: string;         // 详情页 URL
  item_type: string;         // 推断出的类型：movie/series/variety/anime/generic
  title?: string;             // 搜索结果标题（可能与原始标题不同）
  poster?: string;             // 搜索结果的封面
}
```

### 并行搜索实现

```rust
async fn search_vod_sources(title: &str) -> Vec<SearchResult> {
    let zxzj = spawn(search_zxzj(title));
    let jpvod = spawn(search_jpvod(title));
    let xb6v = spawn(search_xb6v(title));

    let mut results = Vec::new();
    results.extend(zxzj.await??);
    results.extend(jpvod.await??);
    results.extend(xb6v.await??);
    results
}
```

### 各源搜索方式

**zxzjhd.com**（GET，URL 重写）
- URL：`/vodsearch/-------------.html?wd={title}&submit=`
- 类型推断：从详情页 URL 路径解析 `/movie/` → movie，`/dianshiju/` → series 等
- 解析：`parse_listing_page`（复用现有）

**jpvod.com**（GET）
- URL：`/search/-------------.html?wd={title}&submit=`
- item_type：默认 `generic`（不推断类型）
- 解析：jianpian 风格 `parse_listing_page`

**xb6v.com**（POST + 重定向）
- POST：`/e/search/1index.php`，payload `show=title&tempid=1&tbname=article&mid=1&dopost=search&submit=&keyboard={title}`
- 响应 Header `Location: result/?searchid=XXX` → 访问结果页
- item_type：默认 `generic`
- 解析：xb6v 风格 `parse_listing_page`

### 搜索结果去重与排序

- 同一标题多个结果 → 保留评分最高的
- zxzj 结果优先（类型分类能力最强）
- generic 类型结果排在最后

## 前端变化

### Tab 默认内容

| Tab | 数据源 |
|---|---|
| movie | `douban_hot WHERE item_type='movie'` |
| series | `douban_hot WHERE item_type='series'` |
| variety | `douban_hot WHERE item_type='variety'` |
| anime | `douban_hot WHERE item_type='anime'` |
| live | 保留现有逻辑（直播频道）|

### 详情页流程

用户点击热播条目后：

1. 跳转 `/detail/hot/:doubanId`
2. 显示加载状态（豆瓣封面 + 标题）
3. 并行调用 `search_vod_sources(title, item_type)`
4. 搜索完成 → 展示播放源列表
   - 有 item_type 结果 → 按 tab 类型过滤展示
   - generic 结果 → 单独分组展示
5. 用户选源 → 调用现有详情抓取逻辑 → 进入播放

### UI 状态

```
[加载中]           → 搜索进行中
[播放源列表]       → 搜索完成，有结果
[暂无播放源]       → 搜索完成，无结果
```

## 数据库 Schema

```sql
-- douban_hot 表（已存在，新增 item_type 列）
CREATE TABLE douban_hot (
    id INTEGER PRIMARY KEY,           -- 豆瓣 subject ID
    name TEXT NOT NULL,
    year INTEGER,
    poster TEXT,
    rating REAL,
    rank INTEGER,
    updated_at TEXT,
    item_type TEXT DEFAULT 'movie'    -- 新增
);

CREATE INDEX idx_douban_item_type ON douban_hot(item_type);
```

## 实施步骤

1. **豆瓣爬虫升级**：支持分类 JSON API，新增 `item_type` 字段
2. **数据库迁移**：执行 ALTER TABLE 添加 item_type 列
3. **存储层修改**：`upsert_douban_hot` 支持 item_type，`get_douban_hot` 支持按类型查询
4. **搜索 API**：新增 `search_vod_sources` 命令，实现三个源的并行搜索
5. **前端详情页**：新建 `/detail/hot/:doubanId` 路由，处理搜索流程
6. **Tab 默认数据**：修改各 tab 的数据获取逻辑，优先从 douban_hot 加载
7. **刷新逻辑**：实现 24 小时自动刷新

## 风险与限制

- 豆瓣 API 可能有频率限制，需要添加适当的请求间隔（0.5s）
- 部分搜索结果可能因标题不精确而匹配失败
- jpvod/xb6v 不支持类型推断，generic 结果无法参与类型过滤
