<script setup lang="ts">
// Settings page placeholder
</script>

<template>
  <div class="settings-page min-h-screen bg-gray-900 text-white p-4">
    <header class="mb-6">
      <button
        class="px-4 py-2 bg-gray-700 rounded hover:bg-gray-600 transition mb-4"
        @click="window.history.back()"
      >
        ← 返回
      </button>
      <h1 class="text-2xl font-bold">⚙️ 设置</h1>
    </header>

    <div class="max-w-2xl space-y-6">
      <!-- Playback Settings -->
      <div class="bg-gray-800 p-4 rounded-lg">
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
      <div class="bg-gray-800 p-4 rounded-lg">
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

      <!-- About -->
      <div class="bg-gray-800 p-4 rounded-lg">
        <h2 class="text-lg font-bold mb-4">关于</h2>
        <div class="text-gray-400">
          <p>TVBox 影视仓 v0.1.0</p>
          <p class="mt-2">基于 Rust + Tauri + Vue 构建</p>
        </div>
      </div>
    </div>
  </div>
</template>
