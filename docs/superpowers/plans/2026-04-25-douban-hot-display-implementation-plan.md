# 豆瓣热播展示实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在各页面显示豆瓣热播入口，点击后能匹配到目录中的视频并播放

**Architecture:** 后端在 HomePayload 中添加 douban_hot 字段；前端添加热播专区组件；热播详情页实现标题匹配搜索

**Tech Stack:** Vue 3 + Pinia + TypeScript, Rust + SQLite

---

## 文件映射

| 文件 | 变更 |
|---|---|
| `src-tauri/src/models/mod.rs:117-122` | HomePayload 添加 douban_hot 字段 |
| `src-tauri/src/services/storage.rs:715-747` | get_library_home 添加 douban_hot 查询 |
| `src/types.ts` | 添加 DoubanHot 类型 |
| `src/stores/library.ts` | 添加 doubanHot 状态 |
| `src/views/Home.vue` | 添加热播专区组件 |
| `src/views/HotDetail.vue` | 新建热播详情页 |
| `src/router/index.ts` | 添加 /detail/hot/:doubanId 路由 |

---

## Task 1: 后端 HomePayload 添加 douban_hot 字段

**Files:**
- Modify: `src-tauri/src/models/mod.rs:117-122`

- [ ] **Step 1: 修改 HomePayload 结构体**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomePayload {
    pub continue_watching: Vec<HomeCatalogItem>,
    pub latest_updates: Vec<HomeCatalogItem>,
    pub featured: Vec<HomeCatalogItem>,
    pub douban_hot: Vec<DoubanHot>,  // 新增
}
```

---

## Task 2: Storage get_library_home 添加 douban_hot 查询

**Files:**
- Modify: `src-tauri/src/services/storage.rs:715-747`

- [ ] **Step 1: 在 get_library_home 函数中添加 douban_hot 查询**

在 return 之前添加：
```rust
let douban_hot = self.get_douban_hot()?;
```

在 HomePayload 构建时添加字段：
```rust
Ok(HomePayload {
    continue_watching,
    latest_updates,
    featured,
    douban_hot,  // 新增
})
```

完整修改后：
```rust
pub fn get_library_home(&self) -> SqliteResult<HomePayload> {
    let conn = self.conn.lock().unwrap();

    let continue_watching = Vec::new();
    let latest_updates = query_home_catalog_items(
        &conn,
        "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress
         FROM catalog_items ci
         INNER JOIN subscriptions s ON ci.subscription_id = s.id
         WHERE s.enabled = 1
           AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
         ORDER BY ci.updated_at DESC, ci.id DESC
         LIMIT 12",
        [],
    )?;
    let featured = query_home_catalog_items(
        &conn,
        "SELECT ci.id, ci.title, ci.item_type, ci.poster, NULL as progress
         FROM catalog_items ci
         INNER JOIN subscriptions s ON ci.subscription_id = s.id
         WHERE s.enabled = 1
           AND COALESCE(json_extract(ci.detail_json, '$.source'), '') != 'zxzj'
         ORDER BY ci.id DESC
         LIMIT 12",
        [],
    )?;

    let douban_hot = self.get_douban_hot()?;

    Ok(HomePayload {
        continue_watching,
        latest_updates,
        featured,
        douban_hot,
    })
}
```

---

## Task 3: 前端 types.ts 添加 DoubanHot 类型

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: 添加 DoubanHot 类型定义**

在文件末尾添加：
```typescript
export interface DoubanHot {
  id: number
  name: string
  year: number | null
  poster: string | null
  rating: number | null
  rank: number
  updated_at: number
}
```

同时需要在 HomePayloadInput 中添加字段。找到 `export interface HomePayloadInput` 添加：
```typescript
douban_hot?: DoubanHot[]
doubanHot?: DoubanHot[]
```

---

## Task 4: libraryStore 添加 doubanHot 状态

**Files:**
- Modify: `src/stores/library.ts:34-104`

- [ ] **Step 1: 添加 doubanHot 状态**

在 `featured` 后面添加：
```typescript
const doubanHot = ref<DoubanHot[]>([])
```

- [ ] **Step 2: 更新 applyHomePayload 函数**

在 applyHomePayload 函数中添加：
```typescript
doubanHot.value = normalizeDoubanHot(payload.douban_hot ?? payload.doubanHot ?? [])
```

添加 normalizeDoubanHot 辅助函数（在 normalizeCards 附近）：
```typescript
function normalizeDoubanHot(items?: DoubanHot[]): DoubanHot[] {
  return items ?? []
}
```

- [ ] **Step 3: 更新 return 对象**

在 return 中添加：
```typescript
doubanHot,
```

---

## Task 5: Home.vue 添加热播专区组件

**Files:**
- Modify: `src/views/Home.vue:200-215`

- [ ] **Step 1: 在 LiveNowPanel 后面添加热播专区**

在 `<section class="home-secondary-browser">` 之前添加：
```vue
<section v-if="libraryStore.doubanHot.length" class="hot-section mb-8">
  <div class="flex items-center gap-2 mb-4">
    <span class="text-xl">🔥</span>
    <span class="text-lg font-semibold text-white">豆瓣热播</span>
  </div>
  <div class="flex gap-4 overflow-x-auto pb-4">
    <VodCard
      v-for="hot in libraryStore.doubanHot.slice(0, 10)"
      :key="hot.id"
      :item="hot"
      @click="handleHotClick(hot)"
    />
  </div>
</section>
```

- [ ] **Step 2: 添加 handleHotClick 方法**

在 handleVodClick 方法附近添加：
```typescript
function handleHotClick(hot: DoubanHot) {
  router.push(`/detail/hot/${hot.id}`)
}
```

导入 DoubanHot 类型：
```typescript
import type { CatalogCard, CatalogItemType, LiveChannel, VodItem, DoubanHot } from '@/types'
```

---

## Task 6: 创建 HotDetail.vue 热播详情页

**Files:**
- Create: `src/views/HotDetail.vue`

- [ ] **Step 1: 创建 HotDetail.vue 组件**

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import { useLibraryStore } from '@/stores/library'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { DoubanHot, CatalogCard } from '@/types'

const route = useRoute()
const router = useRouter()
const libraryStore = useLibraryStore()

const doubanId = computed(() => Number(route.params.doubanId))
const doubanHot = ref<DoubanHot | null>(null)
const matchedItem = ref<CatalogCard | null>(null)
const loading = ref(true)
const searchLoading = ref(false)
const error = ref<string | null>(null)

async function loadHotDetail() {
  loading.value = true
  error.value = null
  try {
    // 获取热播数据
    const homePayload = await invoke<any>('get_library_home')
    const hot = homePayload.douban_hot?.find((h: DoubanHot) => h.id === doubanId.value)
    if (hot) {
      doubanHot.value = hot
      // 搜索匹配的视频
      await searchMatchedVideo(hot.name)
    } else {
      error.value = '热播数据不存在'
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function searchMatchedVideo(keyword: string) {
  searchLoading.value = true
  try {
    const results = await invoke<CatalogCard[]>('search_vod', { keyword })
    if (results.length > 0) {
      matchedItem.value = results[0]
    }
  } catch (e) {
    console.warn('搜索失败:', e)
  } finally {
    searchLoading.value = false
  }
}

function handlePlay(catalogItem: CatalogCard) {
  router.push(`/detail/${catalogItem.id}`)
}

onMounted(loadHotDetail)
</script>

<template>
  <div class="app-shell">
    <div class="mx-auto max-w-[1400px]">
      <button class="action-button action-button-secondary" type="button" @click="router.back()">
        返回
      </button>

      <div v-if="loading" class="flex min-h-[320px] items-center justify-center">
        <LoadingSpinner size="lg" />
      </div>

      <div v-else-if="error" class="mt-6 text-center text-white/50">
        {{ error }}
      </div>

      <div v-else-if="doubanHot" class="mt-6 space-y-6">
        <!-- 热播基本信息 -->
        <div class="flex gap-6">
          <img
            v-if="doubanHot.poster"
            :src="doubanHot.poster"
            :alt="doubanHot.name"
            class="w-48 rounded-xl"
          />
          <div>
            <h1 class="text-2xl font-bold text-white">{{ doubanHot.name }}</h1>
            <p v-if="doubanHot.year" class="text-white/50">{{ doubanHot.year }}</p>
            <p v-if="doubanHot.rating" class="mt-2">
              <span class="text-yellow-400">⭐</span>
              <span class="text-white">{{ doubanHot.rating }}</span>
            </p>
            <p class="mt-2 text-sm text-white/30">数据来源: 豆瓣热播</p>
          </div>
        </div>

        <!-- 搜索匹配结果 -->
        <div v-if="searchLoading" class="flex items-center gap-2 text-white/50">
          <LoadingSpinner size="sm" />
          <span>搜索匹配视频中...</span>
        </div>

        <div v-else-if="matchedItem">
          <div class="border-t border-white/10 pt-4">
            <h2 class="mb-4 text-lg font-semibold text-white">在目录中找到:</h2>
            <div class="flex items-center gap-4 rounded-xl bg-white/5 p-4">
              <img
                v-if="matchedItem.poster"
                :src="matchedItem.poster"
                :alt="matchedItem.title"
                class="w-24 rounded-lg"
              />
              <div class="flex-1">
                <h3 class="text-white">{{ matchedItem.title }}</h3>
                <p class="text-sm text-white/50">{{ matchedItem.item_type }}</p>
              </div>
              <button
                class="action-button"
                type="button"
                @click="handlePlay(matchedItem)"
              >
                播放
              </button>
            </div>
          </div>
        </div>

        <div v-else class="border-t border-white/10 pt-4">
          <p class="text-white/50">当前源没有找到与此热播匹配的视频</p>
        </div>
      </div>
    </div>
  </div>
</template>
```

---

## Task 7: 添加路由

**Files:**
- Modify: `src/router/index.ts`

- [ ] **Step 1: 添加 HotDetail 路由**

在路由配置中添加：
```typescript
{
  path: '/detail/hot/:doubanId',
  name: 'HotDetail',
  component: () => import('@/views/HotDetail.vue')
}
```

---

## Task 8: 验证编译通过

- [ ] **Step 1: 运行 npm run build**

```bash
npm run build 2>&1
```

Expected: 无 TypeScript 错误

---

## Task 9: 运行测试

- [ ] **Step 1: 运行 npm run test**

```bash
npm run test 2>&1
```

Expected: 所有测试通过

---

## 验证清单

1. `cargo check` (Rust) 编译通过
2. `npm run build` (前端) 编译通过
3. `npm run test` 测试通过
4. 手动测试：刷新订阅后豆瓣热播数据入库
5. 手动测试：Home 页显示热播卡片
6. 手动测试：点击热播卡片跳转到详情页并匹配到视频

---

## 风险与注意事项

1. **DoubanHot 类型需要正确映射**：前端需要正确接收后端返回的 DoubanHot[] 数据
2. **搜索匹配**：标题匹配可能不够精确，需要后续优化匹配算法
3. **豆瓣数据刷新**：热播数据需要定时刷新，建议每天一次