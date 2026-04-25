<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import type { DoubanHot, CatalogCard } from '@/types'

const route = useRoute()
const router = useRouter()

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
    // Get hot data from home payload
    const homePayload = await invoke<any>('get_library_home')
    const hot = homePayload.douban_hot?.find((h: DoubanHot) => h.id === doubanId.value)
    if (hot) {
      doubanHot.value = hot
      // Search for matched video
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
          <span>搜索匹配视频中...</span>
        </div>

        <!-- Matched result -->
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
