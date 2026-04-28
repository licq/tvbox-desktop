# VodDetail 搜索结果展示重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 重写 EpisodeGroupPanel 以统一电影和剧集的展示方式，精简 VodDetail 中搜索结果卡片样式，同步调整 DoubanMetaPanel 响应式布局。

**Architecture:** EpisodeGroupPanel 成为纯展示组件，统一处理电影（直接播放按钮）和剧集（直接展开 chip，>24 集时才显示展开按钮）。VodDetail 中三种数据流（豆瓣/搜索/目录）的 episode 展示都复用该组件，搜索结果的去重卡片保留在 VodDetail 内但使用与 EpisodeGroupPanel 一致的视觉风格。

**Tech Stack:** Vue 3 (Composition API + `<script setup>`), TypeScript, Tailwind CSS (utility classes + scoped `<style>`), Vitest + jsdom + @vue/test-utils

---

## 文件结构

| 文件 | 操作 | 职责 |
|------|------|------|
| `src/components/detail/EpisodeGroupPanel.vue` | 重写 | 统一展示源分组：电影直接显示播放按钮，剧集直接显示 chip（>24 集可展开） |
| `src/components/detail/__tests__/EpisodeGroupPanel.spec.ts` | 创建 | 测试 EpisodeGroupPanel 的电影/剧集渲染、展开收起逻辑 |
| `src/views/VodDetail.vue` | 修改 | 使用新 EpisodeGroupPanel 展示 providerEpisodes 和 catalog episodeGroups；搜索结果去重卡片改为与 EpisodeGroupPanel 一致的卡片风格；移除旧的内联样式 |
| `src/components/detail/DoubanMetaPanel.vue` | 修改 | 添加响应式媒体查询，平板以下改为两列/单列布局 |
| `src/components/detail/EpisodeGroupSkeleton.vue` | 微调 | 骨架屏头部高度与新设计对齐 |

---

### Task 1: 重写 EpisodeGroupPanel.vue

**Files:**
- Modify: `src/components/detail/EpisodeGroupPanel.vue`

- [ ] **Step 1: 重写 EpisodeGroupPanel 模板与逻辑**

完整替换文件内容：

```vue
<script setup lang="ts">
import { computed, ref } from 'vue'
import EpisodeChip from '@/components/media/EpisodeChip.vue'
import type { CatalogEpisode, CatalogEpisodeGroup, CatalogItemType } from '@/types'

const props = defineProps<{
  group: CatalogEpisodeGroup
  item_type?: CatalogItemType
}>()

const emit = defineEmits<{
  play: [episode: CatalogEpisode]
}>()

const isMovie = computed(() => props.item_type === 'movie')

const typeLabel = computed(() => {
  switch (props.item_type) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
})

// 仅剧集且集数 >24 时才需要展开状态
const EXPAND_THRESHOLD = 24
const needsExpand = computed(() => {
  return !isMovie.value && props.group.episodes.length > EXPAND_THRESHOLD
})
const expanded = ref(false)

const visibleEpisodes = computed(() => {
  if (isMovie.value || !needsExpand.value || expanded.value) {
    return props.group.episodes
  }
  return props.group.episodes.slice(0, EXPAND_THRESHOLD)
})

const remainingCount = computed(() => {
  return props.group.episodes.length - EXPAND_THRESHOLD
})
</script>

<template>
  <section class="source-group-card">
    <div class="source-group-header">
      <div class="source-group-header-left">
        <span class="source-group-name">{{ group.source_name }}</span>
        <span class="source-group-count-badge">
          {{ isMovie ? `${group.episodes.length} 个播放源` : `${group.episodes.length} 集` }}
        </span>
      </div>
      <span class="source-group-type-tag">{{ typeLabel }}</span>
    </div>

    <div class="source-group-body">
      <!-- 电影：直接显示播放按钮 -->
      <div v-if="isMovie" class="play-button-row">
        <button
          v-for="episode in group.episodes"
          :key="episode.id"
          class="play-button"
          @click="emit('play', episode)"
        >
          <span class="play-icon">▶</span>
          <span class="play-label">{{ episode.episode_label }}</span>
        </button>
      </div>

      <!-- 剧集：直接显示 chip，超过阈值时可展开 -->
      <template v-else>
        <div class="episode-chip-grid">
          <EpisodeChip
            v-for="episode in visibleEpisodes"
            :key="episode.id"
            :label="episode.episode_label"
            state="playable"
            @click="emit('play', episode)"
          />
        </div>

        <button
          v-if="needsExpand && !expanded"
          class="expand-toggle-button"
          @click="expanded = true"
        >
          <span>展开剩余 {{ remainingCount }} 集</span>
          <svg class="expand-chevron" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 6L8 10L12 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>

        <button
          v-else-if="needsExpand && expanded"
          class="expand-toggle-button"
          @click="expanded = false"
        >
          <span>收起</span>
          <svg class="expand-chevron expanded" width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M4 10L8 6L12 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </template>
    </div>
  </section>
</template>

<style scoped>
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
  transition: transform 200ms ease, border-color 200ms ease;
}
.source-group-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 255, 255, 0.12);
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-count-badge {
  font-size: 0.65rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.35);
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 0.25rem;
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.75rem 1rem;
}
.play-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.play-button {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 0.5rem;
  padding: 0.4rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.play-icon {
  color: rgba(117, 169, 195, 0.7);
  font-size: 0.65rem;
}
.play-button:hover .play-icon {
  color: rgba(200, 230, 245, 0.95);
}
.episode-chip-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.expand-toggle-button {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.4rem;
  width: 100%;
  margin-top: 0.6rem;
  padding: 0.5rem 0.8rem;
  border-radius: 0.6rem;
  font-size: 0.78rem;
  font-weight: 500;
  background: rgba(117, 169, 195, 0.06);
  border: 1px solid rgba(117, 169, 195, 0.15);
  color: rgba(200, 230, 245, 0.7);
  cursor: pointer;
  transition: all 180ms ease;
}
.expand-toggle-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.3);
  color: rgba(200, 230, 245, 0.95);
}
.expand-chevron {
  opacity: 0.6;
  transition: transform 200ms ease;
}
.expand-chevron.expanded {
  transform: rotate(180deg);
}
</style>
```

- [ ] **Step 2: 运行 TypeScript 检查**

Run: `npx vue-tsc --noEmit`
Expected: 无 EpisodeGroupPanel 相关错误

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/EpisodeGroupPanel.vue
git commit -m "feat: unify EpisodeGroupPanel for movie and series display"
```

---

### Task 2: 为 EpisodeGroupPanel 添加单元测试

**Files:**
- Create: `src/components/detail/__tests__/EpisodeGroupPanel.spec.ts`

- [ ] **Step 1: 创建测试目录和文件**

Run: `mkdir -p src/components/detail/__tests__`

- [ ] **Step 2: 编写测试代码**

创建 `src/components/detail/__tests__/EpisodeGroupPanel.spec.ts`：

```ts
import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import EpisodeGroupPanel from '../EpisodeGroupPanel.vue'
import type { CatalogEpisode, CatalogEpisodeGroup, CatalogItemType } from '@/types'

function makeEpisodes(count: number): CatalogEpisode[] {
  return Array.from({ length: count }, (_, i) => ({
    id: i + 1,
    episode_label: `第${i + 1}集`,
    play_url: `http://test.com/ep${i + 1}`,
    order_index: i + 1,
  }))
}

function makeGroup(episodes: CatalogEpisode[], sourceName = '测试源'): CatalogEpisodeGroup {
  return { source_name: sourceName, episodes }
}

describe('EpisodeGroupPanel', () => {
  it('renders play buttons directly for movies', () => {
    const group = makeGroup([
      { id: 1, episode_label: 'HD', play_url: 'http://a', order_index: 1 },
      { id: 2, episode_label: '1080P', play_url: 'http://b', order_index: 2 },
    ])

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    const buttons = wrapper.findAll('.play-button')
    expect(buttons).toHaveLength(2)
    expect(buttons[0].text()).toContain('HD')
    expect(buttons[1].text()).toContain('1080P')

    // 不应有 episode-chip-grid
    expect(wrapper.find('.episode-chip-grid').exists()).toBe(false)
    // 不应有展开按钮
    expect(wrapper.find('.expand-toggle-button').exists()).toBe(false)
  })

  it('renders all chips directly for series with <=24 episodes', () => {
    const episodes = makeEpisodes(12)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(12)

    // 不应有展开按钮
    expect(wrapper.find('.expand-toggle-button').exists()).toBe(false)
  })

  it('renders first 24 chips + expand button for series with >24 episodes', () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(24)

    const expandBtn = wrapper.find('.expand-toggle-button')
    expect(expandBtn.exists()).toBe(true)
    expect(expandBtn.text()).toContain('展开剩余 6 集')
  })

  it('expands to show all chips when expand button is clicked', async () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.find('.expand-toggle-button').trigger('click')

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(30)

    const collapseBtn = wrapper.find('.expand-toggle-button')
    expect(collapseBtn.text()).toContain('收起')
  })

  it('collapses back to 24 chips when collapse button is clicked', async () => {
    const episodes = makeEpisodes(30)
    const group = makeGroup(episodes)

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.find('.expand-toggle-button').trigger('click') // expand
    await wrapper.find('.expand-toggle-button').trigger('click') // collapse

    const chips = wrapper.findAll('.episode-chip')
    expect(chips).toHaveLength(24)

    expect(wrapper.find('.expand-toggle-button').text()).toContain('展开剩余 6 集')
  })

  it('emits play event with episode when play button is clicked (movie)', async () => {
    const group = makeGroup([
      { id: 1, episode_label: 'HD', play_url: 'http://a', order_index: 1 },
    ])

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    await wrapper.find('.play-button').trigger('click')

    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([group.episodes[0]])
  })

  it('emits play event with episode when chip is clicked (series)', async () => {
    const group = makeGroup(makeEpisodes(5))

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    await wrapper.findAll('.episode-chip')[2].trigger('click')

    expect(wrapper.emitted('play')).toHaveLength(1)
    expect(wrapper.emitted('play')![0]).toEqual([group.episodes[2]])
  })

  it('shows source name and type tag in header', () => {
    const group = makeGroup(makeEpisodes(5), '来源A')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-name').text()).toBe('来源A')
    expect(wrapper.find('.source-group-type-tag').text()).toBe('剧集')
  })

  it('shows correct count badge for movies', () => {
    const group = makeGroup(makeEpisodes(3), '来源B')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'movie' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-count-badge').text()).toBe('3 个播放源')
  })

  it('shows correct count badge for series', () => {
    const group = makeGroup(makeEpisodes(8), '来源C')

    const wrapper = mount(EpisodeGroupPanel, {
      props: { group, item_type: 'series' as CatalogItemType },
    })

    expect(wrapper.find('.source-group-count-badge').text()).toBe('8 集')
  })
})
```

- [ ] **Step 3: 安装 @vue/test-utils（如未安装）**

Run: `npm list @vue/test-utils`
If not found: `npm install -D @vue/test-utils`

- [ ] **Step 4: 运行测试**

Run: `npx vitest run src/components/detail/__tests__/EpisodeGroupPanel.spec.ts`
Expected: 全部 9 个测试通过

- [ ] **Step 5: Commit**

```bash
git add src/components/detail/__tests__/EpisodeGroupPanel.spec.ts
git commit -m "test: add EpisodeGroupPanel unit tests"
```

---

### Task 3: 精简并更新 VodDetail.vue

**Files:**
- Modify: `src/views/VodDetail.vue`

- [ ] **Step 1: 更新搜索结果区的去重卡片样式**

在 `src/views/VodDetail.vue` 中，找到 `<section v-else-if="dedupSearchItems.length" class="source-list space-y-4">` 块，替换为：

```vue
<section v-else-if="dedupSearchItems.length" class="source-list space-y-4">
  <div
    v-for="item in dedupSearchItems"
    :key="item.title"
    class="source-group-card"
  >
    <div class="source-group-header">
      <div class="source-group-header-left">
        <img v-if="item.poster" :src="item.poster" class="dedup-poster" />
        <div class="dedup-search-info">
          <span class="source-group-name">{{ item.title }}</span>
          <span class="source-group-count-badge">{{ item.sources.length }} 个播放源</span>
        </div>
      </div>
      <span class="source-group-type-tag">{{ typeLabel(item.item_type) }}</span>
    </div>
    <div class="source-group-body">
      <div class="play-button-row">
        <button
          v-for="src in item.sources"
          :key="src.detail_url"
          class="play-button"
          @click="handleSourceClick(item, src)"
        >
          <span class="play-icon">▶</span>
          <span class="play-label">{{ src.source_name }}</span>
        </button>
      </div>
    </div>
  </div>
</section>
```

并在 `<script setup>` 中添加辅助函数：

```ts
function typeLabel(itemType: string): string {
  switch (itemType) {
    case 'movie': return '电影'
    case 'series': return '剧集'
    case 'variety': return '综艺'
    case 'anime': return '动漫'
    default: return '剧集'
  }
}
```

- [ ] **Step 2: 替换旧的内联样式**

在 `<style scoped>` 中，删除原有的 `.dedup-search-card`、`.dedup-search-card-header`、`.dedup-search-card-info` 和 `.version-button`、`.version-button-row` 规则（因为它们现在与 EpisodeGroupPanel 的样式类冲突或重复）。

保留并新增以下样式到 VodDetail.vue 的 `<style scoped>`：

```css
/* 搜索结果卡片复用 EpisodeGroupPanel 的 source-group-card 风格 */
.dedup-poster {
  width: 3rem;
  height: 4.5rem;
  object-fit: cover;
  border-radius: 0.4rem;
  flex-shrink: 0;
}
.dedup-search-info {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
}

/* 复用 EpisodeGroupPanel 的样式类，但保留在 VodDetail 以防组件隔离 */
.source-group-card {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  overflow: hidden;
  transition: transform 200ms ease, border-color 200ms ease;
}
.source-group-card:hover {
  transform: translateY(-2px);
  border-color: rgba(255, 255, 255, 0.12);
}
.source-group-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255, 255, 255, 0.03);
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.source-group-header-left {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.source-group-name {
  font-size: 0.9rem;
  font-weight: 600;
  color: rgba(255, 255, 255, 0.85);
}
.source-group-count-badge {
  font-size: 0.65rem;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.35);
  padding: 0.15rem 0.4rem;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 0.25rem;
}
.source-group-type-tag {
  font-size: 0.65rem;
  color: rgba(255, 255, 255, 0.3);
}
.source-group-body {
  padding: 0.75rem 1rem;
}
.play-button-row {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
}
.play-button {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  border-radius: 0.5rem;
  padding: 0.4rem 0.9rem;
  font-size: 0.78rem;
  font-weight: 500;
  border: 1px solid rgba(255, 255, 255, 0.08);
  background: rgba(255, 255, 255, 0.04);
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: all 180ms ease;
}
.play-button:hover {
  background: rgba(117, 169, 195, 0.12);
  border-color: rgba(117, 169, 195, 0.25);
  color: rgba(200, 230, 245, 0.95);
}
.play-icon {
  color: rgba(117, 169, 195, 0.7);
  font-size: 0.65rem;
}
.play-button:hover .play-icon {
  color: rgba(200, 230, 245, 0.95);
}
```

- [ ] **Step 3: 验证 EpisodeGroupPanel 的引用无需改动**

确认 VodDetail.vue 中两处使用 `<EpisodeGroupPanel>` 的地方（providerEpisodes 和 detailStore.episodeGroups）props 仍然匹配：
- `:group="group"`
- `:item_type="providerItemType"` 或 `:item_type="detailStore.item?.item_type"`
- `@play="handleProviderEpisodePlay"` 或 `@play="handlePlay"`

这些接口未变，应该无需修改。

- [ ] **Step 4: 运行 TypeScript 检查**

Run: `npx vue-tsc --noEmit`
Expected: 无 VodDetail 相关错误

- [ ] **Step 5: Commit**

```bash
git add src/views/VodDetail.vue
git commit -m "refactor: restyle VodDetail search result cards to match unified design"
```

---

### Task 4: 更新 DoubanMetaPanel.vue 响应式布局

**Files:**
- Modify: `src/components/detail/DoubanMetaPanel.vue`

- [ ] **Step 1: 在 scoped style 中添加响应式媒体查询**

找到 `<style scoped>` 区块末尾（在 `@keyframes skeleton-pulse` 之后），添加：

```css
@media (max-width: 1023px) {
  .douban-meta-panel {
    grid-template-columns: 160px 1fr;
  }
  .douban-meta-summary {
    grid-column: 1 / -1;
    border-left: none;
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-left: 0;
    padding-top: 1rem;
  }
  .poster-img,
  .poster-fallback {
    width: 160px;
    height: 230px;
  }
}

@media (max-width: 639px) {
  .douban-meta-panel {
    grid-template-columns: 1fr;
    gap: 1rem;
  }
  .douban-meta-poster {
    position: static;
    width: 140px;
    margin: 0 auto;
  }
  .poster-img,
  .poster-fallback {
    width: 140px;
    height: 200px;
  }
  .douban-meta-title {
    font-size: 1.4rem;
    text-align: center;
  }
  .douban-meta-rating {
    justify-content: center;
  }
  .douban-meta-summary {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    padding-top: 1rem;
  }
}
```

- [ ] **Step 2: 运行 TypeScript 检查**

Run: `npx vue-tsc --noEmit`
Expected: 无 DoubanMetaPanel 相关错误

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/DoubanMetaPanel.vue
git commit -m "style: add responsive breakpoints to DoubanMetaPanel"
```

---

### Task 5: 更新 EpisodeGroupSkeleton.vue

**Files:**
- Modify: `src/components/detail/EpisodeGroupSkeleton.vue`

- [ ] **Step 1: 调整骨架屏与新设计对齐**

原文件内容替换为：

```vue
<script setup lang="ts">
defineProps<{
  count?: number
}>()
</script>

<template>
  <div class="episode-group-skeleton">
    <div class="skeleton-header">
      <div class="skeleton-title"></div>
      <div class="skeleton-tag"></div>
    </div>
    <div class="skeleton-chips">
      <div
        v-for="i in (count ?? 8)"
        :key="i"
        class="skeleton-chip"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.episode-group-skeleton {
  border-radius: 1rem;
  background: linear-gradient(180deg, rgba(18, 24, 34, 0.94), rgba(10, 14, 21, 0.9));
  border: 1px solid rgba(255, 255, 255, 0.08);
  padding: 0.75rem 1rem;
}
.skeleton-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.75rem;
  padding-bottom: 0.75rem;
  border-bottom: 1px solid rgba(255, 255, 255, 0.06);
}
.skeleton-title {
  height: 1.15rem;
  width: 6rem;
  border-radius: 0.375rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-tag {
  height: 0.75rem;
  width: 2rem;
  border-radius: 0.25rem;
  background: rgba(255, 255, 255, 0.04);
  animation: pulse 1.5s ease-in-out infinite;
}
.skeleton-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 0.35rem;
}
.skeleton-chip {
  height: 2rem;
  width: 3.5rem;
  border-radius: 0.5rem;
  background: rgba(255, 255, 255, 0.06);
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 0.5; }
  50% { opacity: 1; }
}
</style>
```

- [ ] **Step 2: 运行 TypeScript 检查**

Run: `npx vue-tsc --noEmit`
Expected: 无错误

- [ ] **Step 3: Commit**

```bash
git add src/components/detail/EpisodeGroupSkeleton.vue
git commit -m "style: update EpisodeGroupSkeleton to match new card design"
```

---

### Task 6: 全量验证

- [ ] **Step 1: 运行全部前端测试**

Run: `npm run test`
Expected: 全部通过（包含新加的 EpisodeGroupPanel 测试和原有测试）

- [ ] **Step 2: 运行构建检查**

Run: `npm run build`
Expected: 构建成功，无 TypeScript 或 Vite 错误

- [ ] **Step 3: Commit（如有未提交的变更）**

```bash
git diff --quiet || git add -A && git commit -m "chore: final polish after VodDetail redesign"
```

---

## Self-Review

### 1. Spec Coverage

| 设计需求 | 对应任务 |
|----------|----------|
| 统一卡片结构（header + body） | Task 1 EpisodeGroupPanel 重写 |
| 电影直接显示播放按钮 | Task 1 `v-if="isMovie"` 分支 + `.play-button` 样式 |
| 剧集直接显示 chip，>24 集可展开 | Task 1 `needsExpand` + `visibleEpisodes` + `.expand-toggle-button` |
| 搜索结果卡片视觉统一 | Task 3 去重卡片改用 `.source-group-card` 等共享类 |
| VodDetail 精简 | Task 3 删除旧 `.dedup-search-card` / `.version-button` 内联样式 |
| DoubanMetaPanel 响应式 | Task 4 `@media (max-width: 1023px)` 和 `@media (max-width: 639px)` |
| 动画过渡 | Task 1 `transition` / `transform` 样式 |
| 单元测试 | Task 2 9 个测试覆盖电影/剧集/展开/收起/emit |

无遗漏。

### 2. Placeholder Scan

- 无 "TBD"、"TODO"、"implement later"
- 所有步骤都包含实际代码或精确命令
- 无 "similar to Task N" 引用

### 3. Type Consistency

- `CatalogEpisodeGroup` 接口未变，与新组件 props 兼容
- `CatalogItemType` 使用一致
- `typeLabel` 辅助函数在 VodDetail 中独立定义，不影响其他文件
