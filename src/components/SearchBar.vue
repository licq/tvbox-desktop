<script setup lang="ts">
import { ref } from 'vue'

const props = defineProps<{
  placeholder?: string
}>()

const emit = defineEmits<{
  search: [keyword: string]
}>()

const keyword = ref('')

function handleSearch() {
  emit('search', keyword.value)
}
</script>

<template>
  <div class="search-bar flex items-center bg-gray-700 rounded-lg px-3 py-2">
    <input
      v-model="keyword"
      type="text"
      :placeholder="placeholder || '搜索...'"
      class="bg-transparent flex-1 outline-none text-white"
      @keyup.enter="handleSearch"
    />
    <button @click="handleSearch" class="text-gray-400 hover:text-white">
      🔍
    </button>
  </div>
</template>
