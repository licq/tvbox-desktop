<script setup lang="ts">
import type { SourceSubscription } from '@/types'

const props = defineProps<{
  subscriptions: SourceSubscription[]
}>()

const enabledCount = () => props.subscriptions.filter(subscription => subscription.enabled).length
const failureCount = () => props.subscriptions.filter(subscription => subscription.enabled && subscription.last_error).length
</script>

<template>
  <section class="source-health-panel">
    <div class="source-health-header">
      <div>
        <div class="section-title">源状态</div>
        <p>订阅入口保留，但作为健康概览而不是首页主轴。</p>
      </div>
      <RouterLink to="/subscriptions" class="action-button action-button-secondary">订阅管理</RouterLink>
    </div>

    <div class="source-health-metrics">
      <div>
        <span>Enabled</span>
        <strong>{{ enabledCount() }}</strong>
      </div>
      <div>
        <span>Failures</span>
        <strong>{{ failureCount() }}</strong>
      </div>
      <div>
        <span>Total</span>
        <strong>{{ subscriptions.length }}</strong>
      </div>
    </div>

    <div v-if="subscriptions.length" class="source-health-list">
      <div v-for="subscription in subscriptions.slice(0, 8)" :key="subscription.id" class="source-health-row">
        <span>
          <strong>{{ subscription.name }}</strong>
          <small>{{ subscription.kind }}</small>
        </span>
        <em :class="{ 'is-off': !subscription.enabled, 'is-bad': subscription.enabled && subscription.last_error }">
          {{ !subscription.enabled ? '停用' : (subscription.last_error ? '异常' : '正常') }}
        </em>
      </div>
    </div>

    <div v-else class="home-empty-state">还没有订阅源。</div>
  </section>
</template>
