# UI 优化 v2 设计

日期: 2026-04-24

## 概述

优化 Home 页面 Tab 系统，删除无用的热映 tab，隐藏播放器 URL 显示，添加播放器快捷键支持。

---

## Issue 1: 删除热映 Tab

### 现状
- `hot` tab 调用 `get_matched_hot_list` 获取豆瓣热门匹配
- 如果没有匹配数据，页面显示空状态
- 功能不完整，用户价值低

### 目标
删除热映 tab 及其相关代码。

### 实现
1. 从 `Home.vue` 的 tabs 数组中删除 `hot` entry
2. 从 `Home.vue` 模板中删除热映内容区域 (`v-else-if="activeTab === 'hot'"`)
3. 从 `router/index.ts` 清理 `/library/hot` 路由（如果存在）
4. 从 `douban.ts` 保留 `fetchMatchedHot`（其他功能可能用到），但 Home 不调用

---

## Issue 2: 删除占位内容

### 现状
Home 页面中有一些说明性文字，缺乏实际功能价值。

### 目标
删除或简化以下内容：
- `HomeHero` 组件的 summary 描述文字
- `home-secondary-browser` 区域的说明文字 `<p>保留原有路由分类...</p>`

### 实现
删除或简化 Home.vue 中非必要的描述性文字。

---

## Issue 3: 动态 Tab 系统

### 现状
Tabs 硬编码为: `live`, `hot`, `movie`, `series`, `variety`, `anime`

### 目标
Tab 列表从数据库动态获取，支持新增的短剧和网剧分类。

### 实现

**后端：**

1. 修改 `jianpian.rs` 的 `infer_item_type` 函数：
   - "短剧" → `"short_drama"`
   - "网剧" → `"web_drama"`

2. 新增 `get_catalog_types` command (`src-tauri/src/commands/vod.rs`)：
```rust
#[tauri::command]
pub fn get_catalog_types(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        storage.get_distinct_item_types().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
```

3. 新增 `get_distinct_item_types` storage 方法 (`src-tauri/src/services/storage.rs`)：
```rust
pub fn get_distinct_item_types(&self) -> SqliteResult<Vec<String>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT item_type FROM catalog_items
         WHERE item_type IS NOT NULL
         ORDER BY item_type"
    )?;
    let types = stmt.query_map([], |row| row.get(0))?;
    Ok(types.collect::<Result<Vec<_>, _>>()?)
}
```

**前端：**

1. 新增 `catalogTypes` ref 到 Home.vue，从 store 或直接调用 command 获取
2. 将硬编码 tabs 改为 computed 属性：
```typescript
const catalogTypes = computed(() => {
  const base = ['live']
  const dbTypes = libraryStore.availableTypes // from backend
  return [...base, ...dbTypes]
})
```

3. tab 对象结构：
```typescript
type HomeTabKey = 'live' | CatalogItemType

const tabLabels: Record<string, { label: string; eyebrow: string }> = {
  live: { label: '直播', eyebrow: 'Live' },
  movie: { label: '电影', eyebrow: 'Movie' },
  series: { label: '剧集', eyebrow: 'Series' },
  variety: { label: '综艺', eyebrow: 'Shows' },
  anime: { label: '动漫', eyebrow: 'Anime' },
  short_drama: { label: '短剧', eyebrow: 'Short' },
  web_drama: { label: '网剧', eyebrow: 'Web' }
}
```

4. 只显示数据库中实际存在的分类

---

## Issue 4: 短剧和网剧独立 Tab

### 目标
在 Issue 3 的动态 Tab 系统中自然实现。

### 实现
通过 Issue 3 的后端修改，`jianpian.rs` 将短剧和网剧分类为独立的 item_type，前端自动获取并显示为独立 tab。

---

## Issue 5: 播放器 Hover 不显示网址

### 现状
video 元素有 `:title="currentSource?.url || ''"` 在 hover 时显示播放地址。

### 目标
删除 title 属性，hover 时不显示 URL。

### 实现
```vue
<!-- 删除这行 -->
:title="currentSource?.url || ''"
```

---

## Issue 6: 播放器快捷键

### 目标
为播放器添加键盘快捷键支持。

### 实现

**快捷键定义：**
| 快捷键 | 功能 |
|--------|------|
| 空格 / K | 播放/暂停 |
| J | 快退 10 秒 |
| L | 快进 10 秒 |
| 左箭头 | 快退 5 秒 |
| 右箭头 | 快进 5 秒 |
| 上箭头 | 音量+10% |
| 下箭头 | 音量-10% |
| F | 全屏切换 |
| M | 静音切换 |

**实现方式：**

1. 在 `PlayerPage.vue` 中添加 `usePlayerKeyboard` composable：

```typescript
function usePlayerKeyboard() {
  function handleKeydown(e: KeyboardEvent) {
    // 忽略在 input/select 等元素上的按键
    if (['INPUT', 'SELECT', 'TEXTAREA'].includes((e.target as Element).tagName)) return

    switch (e.key) {
      case ' ':
      case 'k':
      case 'K':
        e.preventDefault()
        togglePlay()
        break
      case 'j':
      case 'J':
        e.preventDefault()
        seek(Math.max(0, currentTime.value - 10))
        break
      case 'l':
      case 'L':
        e.preventDefault()
        seek(Math.min(duration.value, currentTime.value + 10))
        break
      case 'ArrowLeft':
        e.preventDefault()
        seek(Math.max(0, currentTime.value - 5))
        break
      case 'ArrowRight':
        e.preventDefault()
        seek(Math.min(duration.value, currentTime.value + 5))
        break
      case 'ArrowUp':
        e.preventDefault()
        setVolume(Math.min(1, volume.value + 0.1))
        break
      case 'ArrowDown':
        e.preventDefault()
        setVolume(Math.max(0, volume.value - 0.1))
        break
      case 'f':
      case 'F':
        e.preventDefault()
        toggleFullscreen()
        break
      case 'm':
      case 'M':
        e.preventDefault()
        toggleMute()
        break
    }
  }

  onMounted(() => document.addEventListener('keydown', handleKeydown))
  onUnmounted(() => document.removeEventListener('keydown', handleKeydown))
}
```

2. 新增辅助函数：
```typescript
function setVolume(v: number) {
  volume.value = v
  if (videoRef.value) videoRef.value.volume = v
}

function toggleMute() {
  if (videoRef.value) {
    videoRef.value.muted = !videoRef.value.muted
  }
}
```

---

## 变更清单

| 文件 | 操作 |
|------|------|
| `src/views/Home.vue` | 删除 hot tab，改为动态 tabs，删除占位文字 |
| `src/views/PlayerPage.vue` | 删除 title 属性，添加快捷键支持 |
| `src/stores/library.ts` | 新增 availableTypes 状态 |
| `src-tauri/src/commands/vod.rs` | 新增 get_catalog_types command |
| `src-tauri/src/services/storage.rs` | 新增 get_distinct_item_types 方法 |
| `src-tauri/src/services/jianpian.rs` | 修改 infer_item_type 区分短剧/网剧 |
| `src/router/index.ts` | 清理 /library/hot 路由 |

---

## 测试要点

1. Home 页面加载后 tabs 只显示有数据的分类
2. 短剧和网剧有独立的 tab 和内容
3. 播放器按快捷键能正确响应
4. 播放器 hover video 不显示 URL
5. 热映相关功能不影响其他功能
