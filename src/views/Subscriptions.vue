<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useSubscriptionStore } from '@/stores/subscription'
import type { SourceSubscription } from '@/types'

const subStore = useSubscriptionStore()

const showAddForm = ref(false)
const newName = ref('')
const newUrl = ref('')
const refreshingId = ref<number | null>(null)

const subscriptions = computed(() => subStore.subscriptions)
const totalSubscriptions = computed(() => subscriptions.value.length)
const enabledSubscriptions = computed(() => subscriptions.value.filter((sub) => sub.enabled).length)
const disabledSubscriptions = computed(() => totalSubscriptions.value - enabledSubscriptions.value)
const activeRefreshSubscription = computed(() =>
  refreshingId.value === null
    ? null
    : subscriptions.value.find((sub) => sub.id === refreshingId.value) ?? null,
)
const activeRefreshCount = computed(() => (subStore.isRefreshing ? 1 : 0))
const activeRefreshLabel = computed(() =>
  activeRefreshSubscription.value ? activeRefreshSubscription.value.name : '当前没有刷新任务',
)

onMounted(async () => {
  try {
    await subStore.fetchSubscriptions()
  } catch (e) {
    alert('加载订阅失败: ' + e)
  }
})

async function handleAdd() {
  const name = newName.value.trim()
  const url = newUrl.value.trim()
  if (!name || !url) return

  try {
    await subStore.addSubscription(name, url)
    newName.value = ''
    newUrl.value = ''
    showAddForm.value = false
  } catch (e) {
    alert('添加失败: ' + e)
  }
}

async function handleRefresh(sub: SourceSubscription) {
  refreshingId.value = sub.id
  subStore.setRefreshing(sub.name, 1, 1)
  try {
    await subStore.refreshSubscription(sub.id)
  } catch (e) {
    alert('刷新失败: ' + e)
  } finally {
    subStore.clearRefreshing()
    refreshingId.value = null
  }
}

async function handleToggle(sub: SourceSubscription) {
  try {
    await subStore.toggleSubscription(sub.id, !sub.enabled)
  } catch (e) {
    alert('切换失败: ' + e)
  }
}

async function handleDelete(sub: SourceSubscription) {
  if (!confirm(`确定删除订阅 "${sub.name}" 吗？`)) return

  try {
    await subStore.deleteSubscription(sub.id)
  } catch (e) {
    alert('删除失败: ' + e)
  }
}
</script>

<template>
  <div class="app-shell subscriptions-page">
    <header class="page-hero">
      <div class="page-hero-copy">
        <p class="eyebrow">订阅任务</p>
        <h1 class="page-hero-title">订阅管理</h1>
        <p class="page-hero-subtitle">
          在这里维护订阅源、查看启用状态和刷新任务。页面保持单一任务流，方便快速添加、更新或清理订阅。
        </p>
      </div>

      <div class="page-hero-actions">
        <RouterLink to="/library/live" class="action-button action-button-secondary">
          ← 返回
        </RouterLink>
        <button
          class="action-button action-button-primary"
          type="button"
          @click="showAddForm = !showAddForm"
        >
          {{ showAddForm ? '收起表单' : '添加订阅' }}
        </button>
      </div>
    </header>

    <section class="task-summary-strip">
      <article class="task-summary-card">
        <p class="panel-kicker">Total</p>
        <div class="task-summary-value">{{ totalSubscriptions }}</div>
        <p class="mt-2 text-sm leading-6 text-white/55">订阅总数</p>
      </article>

      <article class="task-summary-card">
        <p class="panel-kicker">Enabled</p>
        <div class="task-summary-value">{{ enabledSubscriptions }}</div>
        <p class="mt-2 text-sm leading-6 text-white/55">当前启用</p>
      </article>

      <article class="task-summary-card">
        <p class="panel-kicker">Disabled</p>
        <div class="task-summary-value">{{ disabledSubscriptions }}</div>
        <p class="mt-2 text-sm leading-6 text-white/55">当前停用</p>
      </article>

      <article class="task-summary-card">
        <p class="panel-kicker">Refreshing</p>
        <div class="task-summary-value">{{ activeRefreshCount }}</div>
        <p class="mt-2 text-sm leading-6 text-white/55">{{ activeRefreshLabel }}</p>
      </article>
    </section>

    <section v-if="showAddForm" class="surface-panel mt-4 rounded-[2rem] p-5 md:p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <p class="panel-kicker">Create</p>
          <h2 class="panel-header-title">添加订阅</h2>
          <p class="panel-header-subtitle">输入订阅名称和地址，保存后会立刻加入任务列表。</p>
        </div>
      </div>

      <form class="space-y-4" @submit.prevent="handleAdd">
        <div class="field-row">
          <div>
            <label class="field-label" for="subscription-name">名称</label>
            <p class="field-help">用于在任务面板中识别这条订阅。</p>
          </div>
          <input
            id="subscription-name"
            v-model="newName"
            class="field-control"
            type="text"
            placeholder="例如: 我的收藏"
          />
        </div>

        <div class="field-row">
          <div>
            <label class="field-label" for="subscription-url">订阅地址</label>
            <p class="field-help">支持 JSON 或 TVBox 订阅地址，保持和后端现有逻辑一致。</p>
          </div>
          <input
            id="subscription-url"
            v-model="newUrl"
            class="field-control"
            type="text"
            placeholder="https://example.com/subscription.json"
          />
        </div>

        <div class="flex flex-wrap items-center gap-3 pt-1">
          <button class="action-button action-button-primary" type="submit">
            添加订阅
          </button>
          <button
            class="action-button action-button-secondary"
            type="button"
            @click="showAddForm = false"
          >
            取消
          </button>
        </div>
      </form>
    </section>

    <section class="surface-panel mt-4 rounded-[2rem] p-5 md:p-6">
      <div class="panel-header">
        <div class="panel-header-copy">
          <p class="panel-kicker">Tasks</p>
          <h2 class="panel-header-title">订阅列表</h2>
          <p class="panel-header-subtitle">每条记录都可以单独启用、刷新或删除，刷新中的任务会被高亮显示。</p>
        </div>
      </div>

      <div v-if="subStore.loading" class="flex min-h-[220px] items-center justify-center">
        <div
          class="h-10 w-10 animate-spin rounded-full border-2 border-[rgba(216,154,87,0.95)] border-t-transparent"
        ></div>
      </div>

      <div v-else-if="subscriptions.length === 0" class="empty-panel">
        <p class="panel-kicker">Empty</p>
        <h3 class="text-lg font-semibold text-[var(--text-strong)]">暂无订阅</h3>
        <p class="max-w-md text-sm leading-7 text-[var(--text-soft)]">
          添加一个订阅后，这里会显示任务卡片、启用状态和刷新操作。
        </p>
        <button
          class="action-button action-button-primary"
          type="button"
          @click="showAddForm = true"
        >
          添加订阅
        </button>
      </div>

      <div v-else class="space-y-3">
        <div
          v-for="sub in subscriptions"
          :key="sub.id"
          class="task-row transition"
          :class="
            refreshingId === sub.id
              ? 'border-[rgba(240,179,107,0.42)] bg-[rgba(216,154,87,0.1)] shadow-[0_18px_48px_rgba(216,154,87,0.08)]'
              : ''
          "
        >
          <div class="task-row-main">
            <div class="flex flex-wrap items-center gap-3">
              <h3 class="task-row-title">{{ sub.name }}</h3>
              <span
                class="rounded-full border px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em]"
                :class="
                  sub.enabled
                    ? 'border-[rgba(120,200,140,0.24)] bg-[rgba(120,200,140,0.1)] text-[#c9f3d0]'
                    : 'border-white/10 bg-white/5 text-white/55'
                "
              >
                {{ sub.enabled ? '启用' : '停用' }}
              </span>
              <span
                v-if="refreshingId === sub.id"
                class="inline-flex items-center gap-2 rounded-full border border-[rgba(240,179,107,0.22)] bg-[rgba(240,179,107,0.08)] px-3 py-1 text-[11px] font-semibold uppercase tracking-[0.22em] text-[#ffd8aa]"
              >
                <span class="h-2 w-2 rounded-full bg-current animate-pulse"></span>
                刷新中
              </span>
            </div>
            <p class="task-row-subtitle">{{ sub.url }}</p>
          </div>

          <div class="task-row-actions">
            <button
              type="button"
              class="inline-flex items-center gap-3 rounded-full border border-white/10 bg-white/5 px-3 py-2 text-sm text-white/80 transition hover:border-white/20 hover:bg-white/10"
              :aria-pressed="sub.enabled"
              :title="sub.enabled ? '停用订阅' : '启用订阅'"
              @click="handleToggle(sub)"
            >
              <span
                class="relative inline-flex h-6 w-11 items-center rounded-full border border-white/10 transition"
                :class="sub.enabled ? 'bg-[rgba(216,154,87,0.18)]' : 'bg-white/10'"
              >
                <span
                  class="h-4 w-4 rounded-full bg-white shadow-sm transition-transform"
                  :class="sub.enabled ? 'translate-x-5' : 'translate-x-1'"
                ></span>
              </span>
              <span class="font-medium">{{ sub.enabled ? '已启用' : '已停用' }}</span>
            </button>

            <button
              type="button"
              class="action-button action-button-secondary"
              :disabled="refreshingId === sub.id"
              @click="handleRefresh(sub)"
            >
              {{ refreshingId === sub.id ? '刷新中…' : '刷新' }}
            </button>

            <button
              type="button"
              class="danger-button action-button"
              @click="handleDelete(sub)"
            >
              删除
            </button>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
