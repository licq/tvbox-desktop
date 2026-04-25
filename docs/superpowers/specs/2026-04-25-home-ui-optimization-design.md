# Home 页面优化设计

## 背景

当前 TVBox Home 页面的问题：
1. 订阅刷新时看不到进度，不知道刷新到哪了
2. LiveNowPanel 在各 tab 页面都显示，但没有实际用途
3. 视频列表只显示 18 个，尾行两个空位不美观

---

## 设计目标

1. **刷新进度可见**：订阅刷新时显示各类别的当前数量
2. **删除 LiveNowPanel**：移除无用的热播面板，简化界面
3. **固定 4 行显示**：视频列表固定显示 4 行（20 个），而不是当前的 18 个

---

## 页面布局

### 改前

```
┌────────────────────────────────────────┐
│ 🔥 豆瓣热播                           │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  ← 横向滚动
└────────────────────────────────────────┘
│ Live Now  (无用，应该删除)              │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
└────────────────────────────────────────┘
│ [搜索框]                               │
│                                        │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  ← 3 行显示
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐           │  ← 尾行空 2 个
│ [加载更多]                              │
└────────────────────────────────────────┘
```

### 改后

```
┌────────────────────────────────────────┐
│ 🔥 豆瓣热播                           │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  ← 横向滚动
└────────────────────────────────────────┘
│ [搜索框]                               │
│                                        │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │  ← 4 行显示
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
│ ┌────┐ ┌────┐ ┌────┐ ┌────┐ ┌────┐    │
└────────────────────────────────────────┘
```

---

## 数据流

### 刷新进度

**Backend 变更：**

```rust
// refresh_subscription 返回 RefreshResult
pub struct RefreshResult {
    pub subscription_name: String,
    pub counts: RefreshCounts,
}

pub struct RefreshCounts {
    pub live: i32,
    pub movie: i32,
    pub series: i32,
    pub variety: i32,
    pub anime: i32,
    pub other: i32,
}
```

**Frontend 变更：**

```typescript
// subscriptionStore 添加进度状态
const refreshProgress = ref<{ name: string; counts: RefreshCounts }[]>([])

// hydrateSources 循环刷新时更新进度
for (const sub of enabledSubscriptions.value) {
    await subStore.refreshSubscription(sub.id, false)
    // 更新进度显示
}
```

---

## 文件变更

| 文件 | 变更 |
|---|---|
| `src/components/home/LiveNowPanel.vue` | 删除 |
| `src/views/Home.vue` | 移除 LiveNowPanel，显示 4 行 20 个 |
| `src/stores/subscription.ts` | 添加 refreshProgress 状态 |
| `src/stores/library.ts` | 修改 catalogItems 显示为 20 个 |
| `src-tauri/src/commands/subscription.rs` | 修改 refresh_subscription 返回 counts |
| `src-tauri/src/services/storage.rs` | 修改 refresh_subscription 返回 counts |

---

## 实施步骤

### Phase 1: 删除 LiveNowPanel
1. 删除 `src/components/home/LiveNowPanel.vue`
2. 从 Home.vue 移除 LiveNowPanel 引用

### Phase 2: 固定 4 行显示
1. 修改 `library.ts` - `displayedVodItems` 从 `slice(0, 18)` 改为 `slice(0, 20)`
2. 移除 "加载更多" 按钮

### Phase 3: 刷新进度
1. 后端修改 `refresh_subscription` 返回 counts
2. 前端 subscriptionStore 添加进度状态
3. Home.vue 刷新时显示进度

---

## 风险与注意事项

1. **LiveNowPanel 删除后**：live tab 仍然正常显示频道，不受影响
2. **固定 20 个显示**：适合 1920px 宽屏，窄屏仍使用响应式 grid
