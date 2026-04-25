# 懒加载详情优化设计

## 背景

当前 TVBox 实现的问题：
- **刷新慢**：订阅刷新时抓取所有片目的详情 + 剧集列表，耗时很长
- **存储大**：SQLite 存储了数万条 episode 数据，但用户可能只看 5% 的内容
- **体验不一致**：标准 TVBox 是"点击后才抓详情"，我们的实现是"预抓所有"

用户观察到的现象：
- 页面显示很快，只显示豆瓣热播剧
- 筛选和搜索功能返回结果快
- 似乎是在真正选择好视频后再去检索

这说明标准 TVBox 使用的是**懒加载**机制。

---

## 设计目标

1. **加快订阅刷新速度**：从抓取所有详情 → 只抓列表
2. **减少数据库存储**：从预存所有 episodes → 按需存储
3. **保持快速浏览体验**：目录页还是显示完整内容
4. **首次点击时加载详情**：骨架屏缓解等待焦虑

---

## 数据存储变更

### 订阅刷新时

**现状**：
```rust
// scrape_auete_catalog() 返回 ScrapedCatalogItem 包含 episodes
items.push(ScrapedCatalogItem {
    // ...基本信息
    episodes: Vec::new(), // 此时已有完整剧集
});
```

**改后**：
```rust
items.push(ScrapedCatalogItem {
    // ...基本信息
    episodes: Vec::new(), // 空，让 catalog_episodes 表保持空
});
```

数据库行为：
- `catalog_items` 表：完整存储（title, poster, item_type, summary, detail_json）
- `catalog_episodes` 表：新条目默认空，不预填充

### 用户点击视频时

流程：
1. `get_catalog_detail(id)` → 检查 `catalog_episodes` 是否有数据
2. **有数据**：直接返回（离线可用）
3. **无数据**：调用 `scrape_catalog_detail_from_json(detail_json)` 抓取真实详情
4. 抓取成功 → `replace_catalog_item_detail()` 写入 → 返回
5. 抓取失败 → 返回空 episodes，UI 显示"当前无可用播放源"

---

## 数据库变更

### catalog_episodes 表（现有结构）

```sql
-- 当前：刷新时批量写入
INSERT INTO catalog_episodes (catalog_item_id, source_name, episode_label, play_url, ...)
VALUES (?, ?, ?, ?, ...);
```

### 变更：改为按需写入

```rust
// get_catalog_detail 中
let episodes = get_episodes_from_db(item_id);
if episodes.is_empty() && detail_json.is_some() {
    // 触发懒加载
    let scraped = scrape_catalog_detail_from_json(&detail_json).await?;
    if !scraped.episodes.is_empty() {
        replace_catalog_item_detail(id, &scraped)?;
        episodes = get_episodes_from_db(item_id); // 重新读取
    }
}
```

---

## 前端变更

### 详情页加载流程

**现状 (VodDetail.vue)**：
```vue
await detailStore.fetchDetail(itemId)
<!-- 直接等待，有数据就显示 -->
```

**改后**：
```vue
// 立即显示基本信息（poster, title, summary）
// 剧集列表区域显示骨架屏

if (!detailStore.hasEpisodes) {
    detailStore.fetchDetailWithLazyLoad(itemId) // 后台加载
}
```

### 骨架屏组件

新增 `EpisodeGroupSkeleton.vue`：
```vue
<div class="episode-group-skeleton">
  <div class="skeleton-header"></div>
  <div class="skeleton-list">
    <div v-for="i in 6" :key="i" class="skeleton-chip"></div>
  </div>
</div>
```

### 状态处理

| 状态 | UI 显示 |
|---|---|
| 加载中 | 骨架屏 |
| 有 episodes | 正常展示剧集列表 |
| 无 episodes（未抓取） | 空状态："需要网络获取详情" |
| 抓取失败 | 错误提示 + 重试按钮 |

---

## 向后兼容

- **已有数据的用户**：数据库已有 episodes，不受影响，首次访问时不会重复抓取
- **离线场景**：已抓过的条目可正常离线浏览，未抓过的只显示基本信息

---

## 风险与处理

| 风险 | 处理 |
|---|---|
| 首次点击慢 | 骨架屏缓解焦虑，提示"正在加载播放源..." |
| 抓取失败 | 显示友好错误："当前无可用播放源"，允许重试 |
| 重复抓取 | 已抓过的条目（通过 detail_json 判断）不重复抓 |
| 网络不稳定 | 重试机制 + 已有缓存保护 |

---

## 实施步骤

### Phase 1: 订阅刷新优化
1. 修改 `scrape_auete_catalog()` 等 scraper：返回空 episodes
2. 修改 `replace_catalog_for_subscription()`：只写基本信息
3. 数据库迁移脚本：标记现有 episodes 为"已缓存"

### Phase 2: 详情页懒加载
4. 修改 `get_catalog_detail()`：检测空 episodes → 触发抓取
5. 新增骨架屏组件 `EpisodeGroupSkeleton.vue`
6. 修改 `VodDetail.vue`：支持加载状态

### Phase 3: 优化验证
7. 对比刷新耗时变化
8. 对比数据库大小变化
9. 测试离线浏览场景

---

## 测试验证

```bash
# 刷新前后对比
time curl -X POST "http://localhost:1420/api/subscription/1/refresh"  # 刷新耗时

# 数据库大小对比
sqlite3 tvbox.db "SELECT COUNT(*) FROM catalog_items; SELECT COUNT(*) FROM catalog_episodes;"

# 测试懒加载
# 1. 清空某一条目的 episodes
# 2. 点击该视频
# 3. 确认骨架屏 + 加载 + 展开
```