<script setup lang="ts">
const props = withDefaults(
  defineProps<{
    label: string
    tone?: 'warm' | 'cool' | 'neutral' | 'danger'
  }>(),
  {
    tone: 'neutral'
  }
)
</script>

<template>
  <span :class="['source-badge', `source-badge-${props.tone}`]">
    {{ label }}
  </span>
</template>
