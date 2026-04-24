# UI 优化 v2 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 删除热映tab，优化tab系统，添加播放器快捷键

**Architecture:**
- 前端：Home.vue 动态tabs，PlayerPage.vue 快捷键
- 后端：jianpian.rs 类型推断，storage.rs 新增方法，commands/vod.rs 新增command

**Tech Stack:** Vue 3, Tailwind CSS, Tauri, Rust

---

## Task 1: 删除热映 Tab 及相关代码

**Files:**
- Modify: `src/views/Home.vue:28-35` (tabs 数组)
- Modify: `src/views/Home.vue:314-347` (热映内容区域)
- Modify: `src/views/Home.vue:131-157` (watch 中的 hot 处理)

- [ ] **Step 1: 从 tabs 数组删除 hot entry**

在 Home.vue 第 28-35 行，将：
```typescript
const tabs: { key: HomeTabKey; label: string; eyebrow: string }[] = [
  { key: 'live', label: '直播', eyebrow: 'Live' },
  { key: 'hot', label: '热映', eyebrow: 'Hot' },
  { key: 'movie', label: '电影', eyebrow: 'Movie' },
  { key: 'series', label: '剧集', eyebrow: 'Series' },
  { key: 'variety', label: '综艺', eyebrow: 'Shows' },
  { key: 'anime', label: '动漫', eyebrow: 'Anime' }
]
```

改为：
```typescript
const tabs: { key: HomeTabKey; label: string; eyebrow: string }[] = [
  { key: 'live', label: '直播', eyebrow: 'Live' },
  { key: 'movie', label: '电影', eyebrow: 'Movie' },
  { key: 'series', label: '剧集', eyebrow: 'Series' },
  { key: 'variety', label: '综艺', eyebrow: 'Shows' },
  { key: 'anime', label: '动漫', eyebrow: 'Anime' }
]
```

- [ ] **Step 2: 从 Home.vue 模板删除热映内容区域**

删除第 314-347 行的 `v-else-if="activeTab === 'hot'"` 区域。

- [ ] **Step 3: 清理 watch 中的 hot 处理**

在 watch 回调中，删除 `nextTab !== 'hot'` 的条件判断（如果存在）。

- [ ] **Step 4: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 2: 删除占位内容

**Files:**
- Modify: `src/views/Home.vue:236` (HomeHero summary)
- Modify: `src/views/Home.vue:260` (说明文字)

- [ ] **Step 1: 删除 HomeHero 的 summary 属性**

将 `<HomeHero ... summary="...">` 中的 summary 属性删除。

- [ ] **Step 2: 删除 home-secondary-browser 的说明文字**

删除 `<p>保留原有路由分类...</p>` 这一行。

- [ ] **Step 3: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 3: 后端 - 修改 jianpian.rs 区分短剧/网剧

**Files:**
- Modify: `src-tauri/src/services/jianpian.rs:322-342`

- [ ] **Step 1: 修改 infer_item_type 函数**

在 jianpian.rs 的 `infer_item_type` 函数中，将"短剧"和"网剧"从 series 判断中分离出来：

找到当前代码（约第 322-329 行）：
```rust
if ["连续剧", "欧美剧", "国产剧", "香港剧", "台湾剧", "日本剧", "韩国剧", "海外剧", "泰国剧", "短剧"]
    .iter()
    .any(|needle| page_text.contains(needle))
{
    return "series".to_string();
}
```

改为：
```rust
if ["短剧"].iter().any(|needle| page_text.contains(needle)) {
    return "short_drama".to_string();
}
if ["网剧"].iter().any(|needle| page_text.contains(needle)) {
    return "web_drama".to_string();
}
if ["连续剧", "欧美剧", "国产剧", "香港剧", "台湾剧", "日本剧", "韩国剧", "海外剧", "泰国剧"]
    .iter()
    .any(|needle| page_text.contains(needle))
{
    return "series".to_string();
}
```

- [ ] **Step 2: 验证 Rust 编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

---

## Task 4: 后端 - 新增 get_distinct_item_types storage 方法

**Files:**
- Modify: `src-tauri/src/services/storage.rs`

- [ ] **Step 1: 在 storage.rs 添加 get_distinct_item_types 方法**

在 `get_catalog_items` 方法之后（约第 800 行），添加：

```rust
pub fn get_distinct_item_types(&self) -> SqliteResult<Vec<String>> {
    let conn = self.conn.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT item_type FROM catalog_items
         WHERE item_type IS NOT NULL
         AND item_type != ''
         ORDER BY item_type"
    )?;
    let types = stmt.query_map([], |row| row.get(0))?;
    Ok(types.collect::<Result<Vec<_>, _>>()?)
}
```

- [ ] **Step 2: 验证 Rust 编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

---

## Task 5: 后端 - 新增 get_catalog_types command

**Files:**
- Modify: `src-tauri/src/commands/vod.rs`

- [ ] **Step 1: 在 vod.rs 添加 get_catalog_types command**

在 `get_catalog_items` command 之后（约第 63 行），添加：

```rust
#[tauri::command]
pub fn get_catalog_types(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let storage = state.storage.clone();
    storage
        .get_distinct_item_types()
        .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: 在 lib.rs 中注册新 command**

如果 `get_catalog_types` 需要在 `lib.rs` 中导出，确保添加到适当位置。

- [ ] **Step 3: 验证 Rust 编译**

Run: `cd src-tauri && cargo check`
Expected: 编译成功

---

## Task 6: 前端 - 动态 Tab 系统

**Files:**
- Modify: `src/stores/library.ts`
- Modify: `src/views/Home.vue`

- [ ] **Step 1: 在 library store 添加 availableTypes 状态**

在 `src/stores/library.ts` 的 store 定义中，添加：

```typescript
const availableTypes = ref<string[]>([])

// 在 fetchCatalog 成功后，更新 availableTypes
async function fetchCatalog(itemType?: string, keyword?: string) {
  loading.value = true
  error.value = null
  try {
    const payload = await invoke<CatalogCardInput[]>('get_catalog_items', {
      itemType: itemType || null,
      keyword: keyword || null
    })
    catalogItems.value = normalizeCards(payload)

    // 更新 availableTypes（去重）
    const allTypes = new Set(payload.map(item => item.item_type))
    availableTypes.value = [...allTypes]
  } catch (e) {
    error.value = String(e)
    throw e
  } finally {
    loading.value = false
  }
}

// 更新 return 对象
return {
  availableTypes,
  // ... 其他 exports
}
```

- [ ] **Step 2: 修改 Home.vue 使用动态 tabs**

将硬编码的 tabs 数组改为 computed：

```typescript
const tabLabels: Record<string, { label: string; eyebrow: string }> = {
  live: { label: '直播', eyebrow: 'Live' },
  movie: { label: '电影', eyebrow: 'Movie' },
  series: { label: '剧集', eyebrow: 'Series' },
  variety: { label: '综艺', eyebrow: 'Shows' },
  anime: { label: '动漫', eyebrow: 'Anime' },
  short_drama: { label: '短剧', eyebrow: 'Short' },
  web_drama: { label: '网剧', eyebrow: 'Web' }
}

const tabs = computed(() => {
  return ['live', ...libraryStore.availableTypes]
    .filter(type => tabLabels[type]) // 只显示有标签的类型
    .map(key => ({
      key,
      ...tabLabels[key]
    }))
})
```

- [ ] **Step 3: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 7: 播放器 - 删除 title 属性

**Files:**
- Modify: `src/views/PlayerPage.vue:455`

- [ ] **Step 1: 删除 video 元素的 title 属性**

在 PlayerPage.vue 第 455 行，删除：
```vue
:title="currentSource?.url || ''"
```

- [ ] **Step 2: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## Task 8: 播放器 - 添加快捷键

**Files:**
- Modify: `src/views/PlayerPage.vue`

- [ ] **Step 1: 添加 usePlayerKeyboard composable**

在 `<script setup>` 中，在 `toggleFullscreen` 函数之后添加：

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

function usePlayerKeyboard() {
  function handleKeydown(e: KeyboardEvent) {
    // 忽略在 input/select 等元素上的按键
    if (['INPUT', 'SELECT', 'TEXTAREA'].includes((e.target as Element)?.tagName)) return

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

usePlayerKeyboard()
```

- [ ] **Step 2: 验证构建**

Run: `npm run build`
Expected: 编译成功

---

## 自检清单

- [ ] 热映 tab 已删除
- [ ] 占位内容已删除
- [ ] 短剧和网剧在 jianpian.rs 中被区分
- [ ] get_distinct_item_types 方法已添加
- [ ] get_catalog_types command 已添加
- [ ] Home.vue 使用动态 tabs
- [ ] library store 有 availableTypes
- [ ] 播放器 video 不显示 URL title
- [ ] 播放器支持快捷键
- [ ] 所有构建通过
