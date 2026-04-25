# 饭太硬（Auete）数据源扩展设计

## 背景

TVBox 连接饭太硬时，内容比当前实现丰富得多。经过详细调研，发现两个主要问题：

1. **影视条目数量不足** — 抓取覆盖率仅约 2%
2. **播放线路数量不足** — probe 过滤过严，部分可用线路被丢弃

---

## 问题一：影视条目数量（Catalog 抓取）

### 根因分析

1. **Page Limit 限制**：`AUETE_PAGE_LIMIT_PER_CATEGORY = 15` 导致只能抓取每个分类的前 15 页（共 844 页）
2. **Sub-category 页面缺失**：网站有独立的子分类列表页，当前代码完全未抓取

### 网站结构

**Movie 主分类（844 页）**：
- 喜剧片 (`xjp`): 187 页
- 剧情片 (`jqp`): 237 页
- 动作片 (`dzp`): 120 页
- 恐怖片 (`kbp`): 96 页
- 科幻片 (`khp`): 57 页
- 爱情片 (`aqp`): 74 页
- 惊悚片 (`jsp`): 56 页
- 战争片 (`zzp`): 20 页

**Tv 主分类（735 页）**：
- 国产 (`neidi`): 193 页
- 美剧 (`oumei`): 143 页
- 日剧 (`riju`): 92 页
- 韩剧 (`hanju`): 60 页
- 泰剧 (`yataiju`): 26 页
- 网剧 (`wangju`): 78 页
- 台剧 (`taiju`): 18 页
- 港剧 (`tvbgj`): 31 页
- 英剧 (`yingju`): 32 页
- 外剧 (`waiju`): 43 页
- 短剧 (`duanju`): 23 页

还有 `Zy`（综艺）和 `Dm`（动漫）分类未统计。

### 设计方案

**修改 `src-tauri/src/services/auete.rs` 中的 `scrape_auete_catalog()` 函数：**

1. **移除 `AUETE_PAGE_LIMIT_PER_CATEGORY`** 限制
2. **添加子分类抓取**：
   - 遍历所有 Movie 子分类（xjp, dzp, aqp, khp, kbp, jsp, zzp, jqp）
   - 遍历所有 Tv 子分类（oumei, hanju, riju, yataiju, wangju, taiju, neidi, tvbgj, yingju, waiju, duanju）
   - 同时保留主分类（Movie/Tv）抓取
3. **控制并发**：增加 worker 数量（如 20 个）以加快抓取速度
4. **去重**：使用 `HashSet` 确保同一影片不会重复添加

**注意**：`Zy`（综艺）和 `Dm`（动漫）需要先验证其子分类结构再决定是否扩展。

---

## 问题二：播放线路数量（Playback 解析）

### 根因分析

1. **Probe 过滤过严**：在 `resolve_auete_play_page` 中，所有解码后的 URL 都要通过 `probe_media_candidate` 测试。如果 probe 探测失败，该线路不会出现在最终候选列表中。
2. **外部线路被过滤**：`is_external_source` 函数过滤掉包含"网盘、夸克、迅雷、下载、磁力"的线路。

### 发现的线路类型

| 线路 | `pn` | 解码后域名示例 | 状态 |
|---|---|---|---|
| 云播D线 | `dyun` | `vip.dytt-kan.com` | ✅ 可用 |
| 云播M线 | `myun` | `voddend03.myxqqdd.com` | ⚠️ 不稳定（被 probe 过滤） |
| 云播Y线 | `yyun` | `vip.ffzy-play7.com` | ✅ 备用优质 |

### 设计方案

**Phase 1：放宽 Probe 过滤**

修改 `resolve_auete_play_page` 逻辑：
- 对于已知的稳定线路（如 `dyun`, `yyun`），降低 probe 超时阈值或跳过 probe
- 对于不稳定线路（如 `myun`），增加重试机制或标记为"可能不稳定"

**Phase 2：支持外部线路（可选）**

如果用户需要支持百度网盘、夸克等外部线路：
- 修改 `is_external_source` 逻辑，对特定外部源进行适配
- 添加专门的解析器处理网盘直链

---

## 实现计划

### Phase 1：Catalog 抓取扩展
1. 修改 `scrape_auete_catalog()` 移除 page limit
2. 添加 Movie 子分类列表页抓取
3. 添加 Tv 子分类列表页抓取
4. 增加并发 worker 数量
5. 验证抓取结果正确性

### Phase 2：Playback 线路优化
1. 研究 probe 机制，识别过严的过滤条件
2. 为不稳定线路添加降级处理
3. 可选：支持外部线路解析

---

## 风险与注意事项

1. **抓取量巨大**：完整抓取可能产生数万条目，需要分批处理和进度显示
2. **性能考虑**：大量并发请求可能触发网站反爬限制，需要添加延迟
3. **存储考虑**：SQLite 数据库条目上限需要验证
