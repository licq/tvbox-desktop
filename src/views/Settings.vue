<script setup lang="ts">
import { useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'

const router = useRouter()

async function clearSourceSearchCache() {
  try {
    const count = await invoke<number>('clear_source_search_cache')
    alert(`已清除 ${count} 条源搜索缓存`)
  } catch (e) {
    alert('清除失败: ' + e)
  }
}

async function clearDoubanSearchCache() {
  try {
    const count = await invoke<number>('clear_douban_search_cache')
    alert(`已清除 ${count} 条豆瓣搜索缓存`)
  } catch (e) {
    alert('清除失败: ' + e)
  }
}
</script>

<template>
  <div class="app-shell">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="router.back()"
      >
        ← 返回
      </button>
      <h1 class="text-2xl font-bold">⚙️ 设置</h1>
    </header>

    <div class="max-w-2xl space-y-6">
      <!-- Playback Settings -->
      <div class="surface-panel">
        <h2 class="text-lg font-bold mb-4">播放设置</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span>默认播放画质</span>
            <select class="bg-gray-700 px-3 py-1 rounded">
              <option>自动</option>
              <option>1080P</option>
              <option>720P</option>
              <option>480P</option>
            </select>
          </div>
          <div class="flex items-center justify-between">
            <span>启用硬解</span>
            <input type="checkbox" class="w-5 h-5" checked />
          </div>
        </div>
      </div>

      <!-- Interface Settings -->
      <div class="surface-panel">
        <h2 class="text-lg font-bold mb-4">界面设置</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span>主题</span>
            <select class="bg-gray-700 px-3 py-1 rounded">
              <option>深色</option>
              <option>浅色</option>
              <option>自动</option>
            </select>
          </div>
        </div>
      </div>

      <!-- Cache Management -->
      <div class="surface-panel">
        <h2 class="text-lg font-bold mb-4">缓存管理</h2>
        <div class="space-y-3">
          <div class="flex items-center justify-between">
            <span>源搜索缓存</span>
            <button
              class="px-3 py-1 bg-red-700 rounded hover:bg-red-600 transition"
              @click="clearSourceSearchCache"
            >
              清除
            </button>
          </div>
          <div class="flex items-center justify-between">
            <span>豆瓣搜索缓存</span>
            <button
              class="px-3 py-1 bg-red-700 rounded hover:bg-red-600 transition"
              @click="clearDoubanSearchCache"
            >
              清除
            </button>
          </div>
        </div>
      </div>

      <!-- About -->
      <div class="surface-panel">
        <h2 class="text-lg font-bold mb-4">关于</h2>
        <div class="text-gray-400">
          <p>TVBox 影视仓 v0.1.0</p>
          <p class="mt-2">基于 Rust + Tauri + Vue 构建</p>
        </div>
      </div>
    </div>
  </div>
</template>
