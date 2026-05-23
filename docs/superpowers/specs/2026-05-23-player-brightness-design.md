# Video Player Brightness Control — Design

## Overview

Add a brightness adjustment feature to the TVBox video player. Some streaming sources produce overly dark video; users need a quick way to compensate.

---

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| UI Placement | Quick-adjust overlay | Cleanest UI — no clutter in control bar |
| Interaction | Press & hold | Simple, intuitive, no conflict with existing controls |
| Range | 50% – 150% | Subtle range sufficient for most dark video issues |
| Persistence | Session only | No state to manage; resets per video |
| Implementation | CSS `filter: brightness()` | Hardware-accelerated, zero performance cost |

---

## Behavior

### Interaction Flow
1. User presses and holds anywhere on the video area
2. After 200ms hold threshold, a floating brightness slider appears centered over the video
3. While holding, user drags left/right (or slider thumb) to adjust brightness
4. Release dismisses the overlay immediately
5. Brightness resets to 100% when video source changes

### Overlay Details
- **Position**: Centered on video, vertically offset slightly above center
- **Background**: Semi-transparent dark (`rgba(0,0,0,0.85)`)
- **Contents**: Sun icon (☀), horizontal slider, current percentage label
- **Timeout**: If user holds for 3 seconds without interaction, overlay auto-dismisses
- **Z-index**: Above video but below player controls (controls appear on top if overlap)

### Edge Cases
- **Seek during hold**: Allowed — seeking does not trigger or cancel brightness overlay
- **Touch vs mouse**: Works with both (`mousedown`/`touchstart` + `mouseup`/`touchend`)
- **Live mode**: Disabled in live streaming mode (no seek, brightness less relevant)

---

## Implementation

### State
```ts
const brightness = ref(1) // 1.0 = 100%, range [0.5, 1.5]
const showBrightnessOverlay = ref(false)
let holdTimer: ReturnType<typeof setTimeout> | null = null
let dismissTimer: ReturnType<typeof setTimeout> | null = null
```

### Video Element
Apply brightness via CSS `filter` on `.player-video`:
```css
.player-video {
  filter: brightness(var(--video-brightness, 1));
}
```
JavaScript updates the property:
```ts
videoRef.value?.style.setProperty('--video-brightness', brightness.value.toString())
```

### Overlay Template
```vue
<div v-if="showBrightnessOverlay" class="brightness-overlay">
  <span class="brightness-icon">☀</span>
  <input
    type="range"
    min="0.5"
    max="1.5"
    step="0.05"
    :value="brightness"
    @input="handleBrightnessChange"
  />
  <span class="brightness-label">{{ Math.round(brightness * 100) }}%</span>
</div>
```

### Event Handlers
- `videoRef.value?.addEventListener('mousedown', startHold)` + touch equivalent
- `document.addEventListener('mouseup', endHold)` + touch equivalent
- Hold threshold: 200ms before showing overlay
- `handleBrightnessChange`: updates `brightness.value` and applies to video element

---

## Files to Modify

| File | Change |
|------|--------|
| `src/views/PlayerPage.vue` | Add overlay template, event handlers, brightness ref |
| `src/style.css` | Add `.brightness-overlay` styles and `--video-brightness` CSS variable |

---

## Out of Scope

- Contrast, saturation, hue-rotate adjustments
- Saving brightness preference to localStorage
- Keyboard shortcut for brightness