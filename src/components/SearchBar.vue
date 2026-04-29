<script setup lang="ts">
import { ref, watch } from 'vue'

const props = defineProps<{
  placeholder?: string
  keyword?: string
}>()

const emit = defineEmits<{
  search: [keyword: string]
}>()

const keyword = ref(props.keyword ?? '')

watch(() => props.keyword, (val) => {
  keyword.value = val ?? ''
})

function handleSearch() {
  emit('search', keyword.value.trim())
}
</script>

<template>
  <div class="surface-muted flex items-center gap-3 rounded-full px-4 py-3">
    <span class="text-xs uppercase tracking-[0.3em] text-white/40">Search</span>
    <input
      v-model="keyword"
      type="text"
      :placeholder="placeholder || '搜索片名、频道或关键字...'"
      class="min-w-0 flex-1 bg-transparent text-sm text-white outline-none placeholder:text-white/35"
      @keyup.enter="handleSearch"
    />
    <button
      class="action-button action-button-primary h-10 min-w-10 px-4 text-xs uppercase tracking-[0.2em]"
      @click="handleSearch"
    >
      搜索
    </button>
  </div>
</template>
