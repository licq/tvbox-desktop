<script setup lang="ts">
import SourceBadge from '@/components/media/SourceBadge.vue'
import type { CatalogEpisodeGroup } from '@/types'

defineProps<{
  group: CatalogEpisodeGroup | null
}>()
</script>

<template>
  <section class="recommended-source-panel">
    <div>
      <div class="eyebrow">Recommended Source</div>
      <h2>{{ group?.source_name ?? '暂无推荐线路' }}</h2>
      <p>
        {{ group ? '优先展开当前建议来源。后续会继续接入运行时健康度，让推荐理由更准确。' : '当前条目没有可播放入口。' }}
      </p>
    </div>

    <SourceBadge
      :label="group ? `${group.episodes.length} episodes` : 'empty'"
      :tone="group ? 'cool' : 'danger'"
    />
  </section>
</template>
