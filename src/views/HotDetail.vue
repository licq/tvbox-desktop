<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { DoubanHot, SearchResult } from '@/types'

const route = useRoute()
const router = useRouter()

const doubanId = computed(() => Number(route.params.doubanId))
const itemType = computed(() => String(route.query.type || 'movie'))
const doubanHot = ref<DoubanHot | null>(null)
const searchResults = ref<SearchResult[]>([])
const loading = ref(true)
const searchLoading = ref(true)
const error = ref<string | null>(null)

async function loadHotDetail() {
  loading.value = true
  error.value = null
  try {
    const items = await invoke<DoubanHot[]>('get_douban_hot_by_type', { itemType: itemType.value })
    const hot = items.find((h: DoubanHot) => h.id === doubanId.value)
    if (hot) {
      doubanHot.value = hot
      await searchSources(hot.name)
    } else {
      error.value = '热播数据不存在'
    }
  } catch (e) {
    error.value = String(e)
  } finally {
    loading.value = false
  }
}

async function searchSources(keyword: string) {
  searchLoading.value = true
  try {
    const results = await invoke<SearchResult[]>('search_vod_sources', { title: keyword })
    searchResults.value = results
  } catch (e) {
    console.warn('搜索失败:', e)
    searchResults.value = []
  } finally {
    searchLoading.value = false
  }
}

function handleSourceSelect(result: SearchResult) {
  router.push({
    name: 'detail',
    params: { itemId: result.detail_url },
    query: { from: 'hot', source: result.source }
  })
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
        <!-- Hot info -->
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

        <!-- Search loading -->
        <div v-if="searchLoading" class="flex items-center gap-2 text-white/50">
          <LoadingSpinner size="sm" />
          <span>正在搜索播放源...</span>
        </div>

        <!-- Search results -->
        <div v-else-if="searchResults.length > 0" class="space-y-4 border-t border-white/10 pt-4">
          <h2 class="text-lg font-semibold text-white">可用播放源</h2>

          <!-- Typed sources -->
          <div v-if="searchResults.filter(r => r.item_type !== 'generic').length" class="space-y-2">
            <div
              v-for="result in searchResults.filter(r => r.item_type !== 'generic')"
              :key="result.detail_url"
              class="flex items-center gap-4 rounded-xl bg-white/5 p-4 cursor-pointer hover:bg-white/10"
              @click="handleSourceSelect(result)"
            >
              <img v-if="result.poster" :src="result.poster" class="w-16 rounded-lg" />
              <div class="flex-1">
                <h3 class="text-white">{{ result.title || doubanHot.name }}</h3>
                <p class="text-sm text-white/50">{{ result.source_name }}</p>
              </div>
              <span class="text-xs text-white/30">{{ result.item_type }}</span>
            </div>
          </div>

          <!-- Generic sources -->
          <div v-if="searchResults.filter(r => r.item_type === 'generic').length">
            <p class="text-sm text-white/30 mb-2">其他源</p>
            <div class="space-y-2">
              <div
                v-for="result in searchResults.filter(r => r.item_type === 'generic')"
                :key="result.detail_url"
                class="flex items-center gap-4 rounded-xl bg-white/5 p-4 cursor-pointer hover:bg-white/10"
                @click="handleSourceSelect(result)"
              >
                <div class="flex-1">
                  <h3 class="text-white">{{ result.title || doubanHot.name }}</h3>
                  <p class="text-sm text-white/50">{{ result.source_name }}</p>
                </div>
                <span class="text-xs text-white/30">通用</span>
              </div>
            </div>
          </div>
        </div>

        <!-- No results -->
        <div v-else class="border-t border-white/10 pt-4 text-white/50">
          暂未找到可用的播放源
        </div>
      </div>
    </div>
  </div>
</template>