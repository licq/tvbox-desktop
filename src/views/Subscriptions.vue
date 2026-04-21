<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useSubscriptionStore } from '@/stores/subscription'
import type { Subscription } from '@/types'

const subStore = useSubscriptionStore()

const showAddForm = ref(false)
const newName = ref('')
const newUrl = ref('')
const refreshing = ref<number | null>(null)

onMounted(async () => {
  try {
    await subStore.fetchSubscriptions()
  } catch (e) {
    alert('加载订阅失败: ' + e)
  }
})

async function handleAdd() {
  if (!newName.value || !newUrl.value) return
  try {
    await subStore.addSubscription(newName.value, newUrl.value)
    newName.value = ''
    newUrl.value = ''
    showAddForm.value = false
  } catch (e) {
    alert('添加失败: ' + e)
  }
}

async function handleRefresh(sub: Subscription) {
  refreshing.value = sub.id
  try {
    await subStore.refreshSubscription(sub.id)
    alert('刷新成功')
  } catch (e) {
    alert('刷新失败: ' + e)
  } finally {
    refreshing.value = null
  }
}

async function handleToggle(sub: Subscription) {
  try {
    await subStore.toggleSubscription(sub.id, !sub.enabled)
  } catch (e) {
    alert('切换失败: ' + e)
  }
}

async function handleDelete(sub: Subscription) {
  if (!confirm(`确定删除订阅 "${sub.name}" 吗？`)) return
  try {
    await subStore.deleteSubscription(sub.id)
  } catch (e) {
    alert('删除失败: ' + e)
  }
}
</script>

<template>
  <div class="subscriptions-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6 flex items-center justify-between">
      <h1 class="text-2xl font-bold">📡 订阅管理</h1>
      <button
        class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition"
        @click="showAddForm = !showAddForm"
      >
        {{ showAddForm ? '取消' : '+ 添加订阅' }}
      </button>
    </header>

    <!-- Add Form -->
    <div v-if="showAddForm" class="bg-gray-800 p-4 rounded-lg mb-6">
      <div class="mb-4">
        <label class="block text-sm text-gray-400 mb-1">名称</label>
        <input
          v-model="newName"
          type="text"
          class="w-full bg-gray-700 rounded px-3 py-2 outline-none focus:ring-2 focus:ring-primary"
          placeholder="例如: 我的收藏"
        />
      </div>
      <div class="mb-4">
        <label class="block text-sm text-gray-400 mb-1">订阅地址 (JSON)</label>
        <input
          v-model="newUrl"
          type="text"
          class="w-full bg-gray-700 rounded px-3 py-2 outline-none focus:ring-2 focus:ring-primary"
          placeholder="https://example.com/subscription.json"
        />
      </div>
      <button
        class="px-4 py-2 bg-primary rounded hover:bg-blue-600 transition"
        @click="handleAdd"
      >
        添加
      </button>
    </div>

    <!-- Subscription List -->
    <div v-if="subStore.loading" class="flex justify-center py-8">
      <div class="animate-spin w-8 h-8 border-2 border-primary border-t-transparent rounded-full"></div>
    </div>

    <div v-else-if="subStore.subscriptions.length === 0" class="text-center py-8 text-gray-400">
      暂无订阅
    </div>

    <div v-else class="space-y-3">
      <div
        v-for="sub in subStore.subscriptions"
        :key="sub.id"
        class="bg-gray-800 p-4 rounded-lg flex items-center justify-between"
      >
        <div class="flex items-center gap-3">
          <button
            :class="['w-12 h-6 rounded-full transition', sub.enabled ? 'bg-primary' : 'bg-gray-600']"
            @click="handleToggle(sub)"
          >
            <div :class="['w-5 h-5 bg-white rounded-full transition transform', sub.enabled ? 'translate-x-6' : 'translate-x-0.5']"></div>
          </button>
          <div>
            <div class="font-medium">{{ sub.name }}</div>
            <div class="text-sm text-gray-400 truncate max-w-md">{{ sub.url }}</div>
          </div>
        </div>
        <div class="flex gap-2">
          <button
            :disabled="refreshing === sub.id"
            class="px-3 py-1 bg-gray-700 rounded hover:bg-gray-600 transition disabled:opacity-50"
            @click="handleRefresh(sub)"
          >
            {{ refreshing === sub.id ? '刷新中...' : '🔄 刷新' }}
          </button>
          <button
            class="px-3 py-1 bg-red-600 rounded hover:bg-red-700 transition"
            @click="handleDelete(sub)"
          >
            🗑️
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
