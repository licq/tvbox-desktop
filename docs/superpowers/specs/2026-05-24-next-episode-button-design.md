# Next Episode Button — Design

## Overview

Add a "下一集 →" button to the player control bar that auto-plays the next episode when clicked.

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Placement | Control bar, right side | Consistent with existing controls, always accessible |
| Visibility | Always shown in series/variety/anime mode when next episode exists | User can always see the option |
| Behavior | Auto-play next episode immediately | One-click continuous viewing |

---

## Behavior

### When Visible
- **Mode is series/variety/anime** (not movie/live)
- **Next episode exists** — `normalizedIndex + 1 < unifiedEpisodes.length`
- Button shows "下一集 →" text

### When Hidden
- Movie mode (`itemType === 'movie'`)
- Last episode of a series/variety/anime
- Live mode

### On Click
1. Find next episode in `unifiedEpisodes` by `normalizedIndex + 1`
2. Call `playUnifiedEpisode(nextEpisode)` with preferred source index
3. Player transitions to next episode, auto-plays if possible

### Edge Cases
- **No more episodes**: Button is hidden (not disabled)
- **First episode**: Button is visible and navigates to episode 2
- **During playback error**: Button still visible, works normally
- **Live mode**: Button is hidden (not applicable)

---

## Implementation

### State
```ts
const currentNormalizedIndex = computed(() => {
  const id = Number(route.params.id)
  const epLabel = route.query.ep as string
  // ... existing logic
})

const hasNextEpisode = computed(() => {
  if (itemType.value === 'movie') return false
  const nextIdx = (currentNormalizedIndex.value ?? -1) + 1
  return nextIdx < unifiedEpisodes.value.length
})

const nextEpisode = computed(() => {
  if (!hasNextEpisode.value) return null
  return unifiedEpisodes.value.find(e => e.normalizedIndex === currentNormalizedIndex.value + 1) ?? null
})
```

### Button Template (in control bar)
```vue
<button
  v-if="hasNextEpisode"
  class="action-button action-button-secondary"
  type="button"
  @click="playNextEpisode"
>
  下一集 →
</button>
```

### Handler
```ts
function playNextEpisode() {
  if (nextEpisode.value) {
    playUnifiedEpisode(nextEpisode.value, undefined, true)
  }
}
```

---

## Files to Modify

| File | Change |
|------|--------|
| `src/views/PlayerPage.vue` | Add hasNextEpisode computed, nextEpisode computed, playNextEpisode handler, button in template |

---

## Out of Scope

- Auto-advance when current video ends (requires video 'ended' event handling)
- Auto-advance countdown timer UI
- Previous episode button
- Keyboard shortcut for next episode