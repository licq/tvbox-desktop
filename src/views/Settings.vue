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
    <header class="page-hero">
      <div class="page-hero-copy">
        <p class="eyebrow">系统偏好</p>
        <h1 class="page-hero-title">设置</h1>
        <p class="page-hero-subtitle">
          调整播放、外观与缓存行为，保持 TVBox 的浏览体验一致、克制且流畅。
        </p>
      </div>

      <div class="page-hero-actions">
        <button class="action-button action-button-secondary" @click="router.back()">
          ← 返回
        </button>
      </div>
    </header>

    <div class="panel-grid">
      <div class="panel-stack">
        <section class="surface-panel rounded-[2rem] p-5 md:p-6">
          <div class="panel-header">
            <div class="panel-header-copy">
              <p class="panel-kicker">Playback</p>
              <h2 class="panel-header-title">播放设置</h2>
              <p class="panel-header-subtitle">控制默认画质与解码方式，优先保证播放稳定性。</p>
            </div>
          </div>

          <div class="space-y-4">
            <div class="field-row">
              <div>
                <label class="field-label" for="default-quality">默认播放画质</label>
                <p class="field-help">新播放任务会优先采用这个画质，实际结果仍会受片源可用性影响。</p>
              </div>
              <select id="default-quality" class="field-control">
                <option>自动</option>
                <option>1080P</option>
                <option>720P</option>
                <option>480P</option>
              </select>
            </div>

            <div class="field-row">
              <div>
                <label class="field-label" for="hardware-decoding">启用硬解</label>
                <p class="field-help">在支持的设备上提升播放效率，减少 CPU 占用和卡顿。</p>
              </div>
              <label
                class="field-control flex min-h-[2.75rem] items-center justify-between gap-4"
                for="hardware-decoding"
              >
                <span class="text-sm text-white/85">开启硬件解码</span>
                <input
                  id="hardware-decoding"
                  type="checkbox"
                  class="h-5 w-5 rounded border border-white/15 bg-white/5 accent-[var(--accent)]"
                  checked
                />
              </label>
            </div>
          </div>
        </section>

        <section class="surface-panel rounded-[2rem] p-5 md:p-6">
          <div class="panel-header">
            <div class="panel-header-copy">
              <p class="panel-kicker">Appearance</p>
              <h2 class="panel-header-title">界面设置</h2>
              <p class="panel-header-subtitle">保留沉浸式视觉语言，同时选择最适合当前环境的主题模式。</p>
            </div>
          </div>

          <div class="space-y-4">
            <div class="field-row">
              <div>
                <label class="field-label" for="theme-mode">主题</label>
                <p class="field-help">推荐使用自动模式，让界面跟随系统外观切换。</p>
              </div>
              <select id="theme-mode" class="field-control">
                <option>深色</option>
                <option>浅色</option>
                <option>自动</option>
              </select>
            </div>
          </div>
        </section>
      </div>

      <div class="panel-stack">
        <section class="surface-panel rounded-[2rem] p-5 md:p-6">
          <div class="panel-header">
            <div class="panel-header-copy">
              <p class="panel-kicker">Maintenance</p>
              <h2 class="panel-header-title">缓存管理</h2>
              <p class="panel-header-subtitle">仅清理本地搜索缓存，不影响订阅源或媒体库内容。</p>
            </div>
          </div>

          <div class="space-y-3">
            <div class="task-row">
              <div class="task-row-main">
                <h3 class="task-row-title">源搜索缓存</h3>
                <p class="task-row-subtitle">清除源检索结果的本地缓存，适合源更新后重新拉取最新数据。</p>
              </div>
              <div class="task-row-actions">
                <button class="danger-button action-button" @click="clearSourceSearchCache">
                  清除缓存
                </button>
              </div>
            </div>

            <div class="task-row">
              <div class="task-row-main">
                <h3 class="task-row-title">豆瓣搜索缓存</h3>
                <p class="task-row-subtitle">清除豆瓣热榜与搜索结果缓存，便于修正旧数据或刷新列表。</p>
              </div>
              <div class="task-row-actions">
                <button class="danger-button action-button" @click="clearDoubanSearchCache">
                  清除缓存
                </button>
              </div>
            </div>
          </div>
        </section>

        <section class="surface-panel rounded-[2rem] p-5 md:p-6">
          <div class="panel-header">
            <div class="panel-header-copy">
              <p class="panel-kicker">About</p>
              <h2 class="panel-header-title">关于</h2>
              <p class="panel-header-subtitle">应用信息与构建栈，便于快速确认当前版本和技术底座。</p>
            </div>
          </div>

          <div class="grid gap-4 rounded-[1.5rem] border border-white/8 bg-white/3 p-4">
            <div class="flex items-center justify-between gap-4">
              <div class="grid gap-1">
                <p class="text-sm font-medium text-white/85">TVBox 影视仓</p>
                <p class="text-sm text-white/50">v0.1.0</p>
              </div>
              <span class="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs font-medium uppercase tracking-[0.22em] text-white/65">
                Desktop
              </span>
            </div>

            <p class="text-sm leading-7 text-white/60">
              基于 Rust + Tauri + Vue 构建，采用统一的电影感面板与柔和高对比交互层，适配桌面端浏览与播放场景。
            </p>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>
