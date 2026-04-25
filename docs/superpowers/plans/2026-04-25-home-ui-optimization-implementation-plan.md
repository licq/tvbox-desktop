# Home 页面优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化 Home 页面：删除 LiveNowPanel，固定 4 行显示，添加刷新进度

**Architecture:** 删除无用组件，修改 grid 布局添加 20 个固定显示，subscription store 添加进度状态

**Tech Stack:** Vue 3 + Pinia + TypeScript, Rust + SQLite

---

## 文件映射

| 文件 | 变更 |
|---|---|
| `src/components/home/LiveNowPanel.vue` | 删除 |
| `src/views/Home.vue` | 移除 LiveNowPanel，改 18 为 20 个，移除加载更多 |
| `src/stores/library.ts` | 修改 displayedVodItems slice(0, 20) |
| `src/stores/subscription.ts` | 添加 refreshProgress 状态 |
| `src-tauri/src/commands/subscription.rs` | refresh_subscription 返回 RefreshResult |
| `src-tauri/src/models/mod.rs` | 添加 RefreshResult 类型 |

---

## Task 1: 删除 LiveNowPanel 组件

**Files:**
- Delete: `src/components/home/LiveNowPanel.vue`

- [ ] **Step 1: 删除 LiveNowPanel.vue 文件**

```bash
rm src/components/home/LiveNowPanel.vue
```

- [ ] **Step 2: 提交删除**

```bash
git rm src/components/home/LiveNowPanel.vue
git commit -m "remove: delete unused LiveNowPanel component"
```

---

## Task 2: 从 Home.vue 移除 LiveNowPanel

**Files:**
- Modify: `src/views/Home.vue:11`
- Modify: `src/views/Home.vue:206`
- Modify: `src/views/Home.vue:4`

- [ ] **Step 1: 移除 import LiveNowPanel**

修改 line 11:
```typescript
// 删除这行
import LiveNowPanel from '@/components/home/LiveNowPanel.vue'
```

- [ ] **Step 2: 移除模板中的 LiveNowPanel**

修改 line 206（`<LiveNowPanel :groups="liveStore.groups" @play="handlePlayChannel" />`）:
删除整行

- [ ] **Step 3: 移除 useLiveStore (如果不再需要)**

检查是否还有用到 liveStore，如果没有也移除 import

- [ ] **Step 4: 提交更改**

```bash
git add src/views/Home.vue
git commit -m "remove: delete LiveNowPanel from Home.vue"
```

---

## Task 3: 修改视频列表为固定 20 个

**Files:**
- Modify: `src/stores/library.ts:76`
- Modify: `src/views/Home.vue:274-297`

- [ ] **Step 1: 修改 displayedVodItems 计算属性**

修改 library.ts line 76:
```typescript
// 从
return libraryStore.catalogItems.slice(0, 18)
// 改为
return libraryStore.catalogItems.slice(0, 20)
```

- [ ] **Step 2: 移除加载更多按钮**

修改 Home.vue line 293-297:
删除整个 `v-if="libraryStore.catalogItems.length > 18 && !showAllVod"` 的 div 块

- [ ] **Step 3: 移除 showAllVod ref (如果不再需要)**

检查是否还有用到 showAllVod，如果没有也移除

- [ ] **Step 4: 提交更改**

```bash
git add src/stores/library.ts src/views/Home.vue
git commit -m "feat: display 20 items (4 rows) instead of 18"
```

---

## Task 4: 后端返回刷新进度

**Files:**
- Modify: `src-tauri/src/models/mod.rs`
- Modify: `src-tauri/src/commands/subscription.rs:44`
- Modify: `src-tauri/src/services/storage.rs:968`

- [ ] **Step 1: 在 models/mod.rs 添加 RefreshResult 类型**

在文件末尾添加:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResult {
    pub subscription_name: String,
    pub live_count: i32,
    pub movie_count: i32,
    pub series_count: i32,
    pub variety_count: i32,
    pub anime_count: i32,
    pub other_count: i32,
}
```

- [ ] **Step 2: 修改 storage.rs refresh_subscription 返回 RefreshResult**

修改 `pub fn refresh_subscription` 签名和实现:
```rust
pub fn refresh_subscription(
    &self,
    id: i64,
    lives: Vec<(String, Option<String>, String, Option<String>)>,
    vods: Vec<(String, String, Option<String>, Option<String>, String)>,
) -> SqliteResult<RefreshResult> {
```

在函数开始处记录 counts，在最后返回:
```rust
let live_count = lives.len() as i32;
let mut movie_count = 0;
let mut series_count = 0;
let mut variety_count = 0;
let mut anime_count = 0;
let mut other_count = 0;

for (_, vtype, _, _, _) in &vods {
    match vtype.as_str() {
        "movie" => movie_count += 1,
        "series" => series_count += 1,
        "variety" => variety_count += 1,
        "anime" => anime_count += 1,
        _ => other_count += 1,
    }
}

// ... 现有插入逻辑 ...

Ok(RefreshResult {
    subscription_name: "".to_string(), // 暂时用空字符串
    live_count: live_count as i32,
    movie_count,
    series_count,
    variety_count,
    anime_count,
    other_count,
})
```

- [ ] **Step 3: 修改 commands/subscription.rs 的 refresh_subscription**

修改返回值类型:
```rust
pub async fn refresh_subscription(id: i64, state: State<'_, AppState>) -> Result<RefreshResult, String> {
```

在调用处:
```rust
.refresh_subscription(id, lives_data, vods_data)
```

- [ ] **Step 4: 运行 cargo check 验证编译**

```bash
cd src-tauri && cargo check 2>&1 | tail -20
```

- [ ] **Step 5: 提交更改**

```bash
git add src-tauri/src/models/mod.rs src-tauri/src/services/storage.rs src-tauri/src/commands/subscription.rs
git commit -m "feat: refresh_subscription returns RefreshResult with counts"
```

---

## Task 5: 前端 subscriptionStore 添加进度状态

**Files:**
- Modify: `src/stores/subscription.ts`

- [ ] **Step 1: 添加进度状态**

修改 subscription.ts:
```typescript
const refreshProgress = ref<{
  name: string
  live: number
  movie: number
  series: number
  variety: number
  anime: number
  other: number
}[]>([])
```

- [ ] **Step 2: 修改 refreshSubscription 返回结果**

```typescript
async function refreshSubscription(id: number, reload = true) {
  try {
    const result = await invoke<any>('refresh_subscription', { id })
    if (result) {
      refreshProgress.value.push({
        name: result.subscription_name || '订阅',
        live: result.live_count || 0,
        movie: result.movie_count || 0,
        series: result.series_count || 0,
        variety: result.variety_count || 0,
        anime: result.anime_count || 0,
        other: result.other_count || 0,
      })
    }
    if (reload) {
      await fetchSubscriptions()
    }
  } catch (e) {
    error.value = String(e)
    throw e
  }
}
```

- [ ] **Step 3: 在 return 中导出**

```typescript
return {
  subscriptions,
  loading,
  error,
  refreshProgress,  // 新增
  fetchSubscriptions,
  addSubscription,
  deleteSubscription,
  refreshSubscription,
  toggleSubscription
}
```

- [ ] **Step 4: 提交更改**

```bash
git add src/stores/subscription.ts
git commit -m "feat: add refreshProgress to subscriptionStore"
```

---

## Task 6: 验证编译通过

- [ ] **Step 1: 运行 npm run build**

```bash
npm run build 2>&1 | tail -20
```

Expected: 无 TypeScript 错误

---

## Task 7: 运行测试

- [ ] **Step 1: 运行 npm run test -- --exclude='.worktrees/**'**

```bash
npm run test -- --exclude='.worktrees/**' 2>&1
```

Expected: 所有测试通过

---

## 验证清单

1. `cargo check` (Rust) 编译通过
2. `npm run build` (前端) 编译通过
3. `npm run test` 测试通过
4. 手动测试：Home 页面不显示 LiveNowPanel
5. 手动测试：视频列表显示 20 个（4 行），无加载更多按钮
6. 手动测试：刷新订阅时控制台显示进度
