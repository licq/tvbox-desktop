# Video Player Brightness Control — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add press-and-hold brightness adjustment overlay to the TVBox video player via CSS `filter: brightness()`.

**Architecture:** Brightness is applied directly to the video element using a CSS custom property (`--video-brightness`) with a hardware-accelerated `filter: brightness()` transform. A floating overlay appears after 200ms hold and auto-dismisses after 3s without interaction. Brightness is session-only (resets per video).

**Tech Stack:** Vue 3 Composition API, vanilla TypeScript, CSS filters

---

## File Map

| File | Change |
|------|--------|
| `src/views/PlayerPage.vue:70-96` | Add brightness state vars, timer lets, event handler functions |
| `src/views/PlayerPage.vue:2030-2034` | Add brightness overlay template after video element |
| `src/style.css:1076-1081` | Add `--video-brightness` CSS variable to `.player-video` |
| `src/style.css` (new) | Add `.brightness-overlay` styles |

---

## Task 1: Add brightness state and timers to PlayerPage.vue

**Files:**
- Modify: `src/views/PlayerPage.vue:70-82`

Add after line 82 (`const isSeeking = ref(false)`):

```ts
const brightness = ref(1) // 1.0 = 100%, range [0.5, 1.5]
const showBrightnessOverlay = ref(false)
let holdTimer: ReturnType<typeof setTimeout> | null = null
let dismissTimer: ReturnType<typeof setTimeout> | null = null
```

- [ ] **Step 1: Add brightness state to PlayerPage.vue**

```ts
const isSeeking = ref(false)

const brightness = ref(1) // 1.0 = 100%, range [0.5, 1.5]
const showBrightnessOverlay = ref(false)
let holdTimer: ReturnType<typeof setTimeout> | null = null
let dismissTimer: ReturnType<typeof setTimeout> | null = null
```

- [ ] **Step 2: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add brightness state refs and timer lets"
```

---

## Task 2: Add brightness overlay template to PlayerPage.vue

**Files:**
- Modify: `src/views/PlayerPage.vue:2030-2034`

Add after line 2033 (`<div class="player-vignette-bottom"></div>`) and before line 2035 (`<div class="player-overlay"...`):

```vue
            <div
              v-if="showBrightnessOverlay"
              class="brightness-overlay"
            >
              <span class="brightness-icon">☀</span>
              <input
                type="range"
                min="0.5"
                max="1.5"
                step="0.05"
                :value="brightness"
                class="brightness-slider"
                @input="handleBrightnessChange"
              />
              <span class="brightness-label">{{ Math.round(brightness * 100) }}%</span>
            </div>
```

- [ ] **Step 1: Add brightness overlay template after player-vignette-bottom div**

- [ ] **Step 2: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add brightness overlay template"
```

---

## Task 3: Add brightness CSS variable and overlay styles

**Files:**
- Modify: `src/style.css:1076-1081`

Add to `.player-video` rule:
```css
filter: brightness(var(--video-brightness, 1));
```

Add after the `.player-video` block (around line 1082):

```css
.brightness-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  background: rgba(0, 0, 0, 0.85);
  padding: 16px 24px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  gap: 12px;
  z-index: 100;
  pointer-events: auto;
}

.brightness-icon {
  font-size: 20px;
}

.brightness-slider {
  width: 150px;
}

.brightness-label {
  font-size: 12px;
  color: #fff;
  min-width: 40px;
}
```

- [ ] **Step 1: Add brightness CSS to .player-video**

- [ ] **Step 2: Add .brightness-overlay styles to style.css**

- [ ] **Step 3: Commit**

```bash
git add src/style.css
git commit -m "feat(player): add brightness CSS variable and overlay styles"
```

---

## Task 4: Add brightness event handler functions

**Files:**
- Modify: `src/views/PlayerPage.vue` — add functions near existing handlers (around where `handleVolumeChange` is defined)

Find `handleVolumeChange` in the script block and add these functions before it:

```ts
function startHold() {
  if (mode.value === 'live') return
  holdTimer = setTimeout(() => {
    showBrightnessOverlay.value = true
    dismissTimer = setTimeout(() => {
      showBrightnessOverlay.value = false
    }, 3000)
  }, 200)
}

function endHold() {
  if (holdTimer) {
    clearTimeout(holdTimer)
    holdTimer = null
  }
  if (dismissTimer) {
    clearTimeout(dismissTimer)
    dismissTimer = null
  }
  showBrightnessOverlay.value = false
}

function handleBrightnessChange(e: Event) {
  const target = e.target as HTMLInputElement
  const value = parseFloat(target.value)
  brightness.value = value
  if (videoRef.value) {
    videoRef.value.style.setProperty('--video-brightness', value.toString())
  }
  // Reset dismiss timer on interaction
  if (dismissTimer) {
    clearTimeout(dismissTimer)
    dismissTimer = setTimeout(() => {
      showBrightnessOverlay.value = false
    }, 3000)
  }
}

function resetBrightness() {
  brightness.value = 1
  if (videoRef.value) {
    videoRef.value.style.setProperty('--video-brightness', '1')
  }
  showBrightnessOverlay.value = false
}
```

- [ ] **Step 1: Add brightness event handlers (startHold, endHold, handleBrightnessChange, resetBrightness)**

- [ ] **Step 2: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): add brightness hold/input/reset handlers"
```

---

## Task 5: Wire up brightness event listeners

**Files:**
- Modify: `src/views/PlayerPage.vue` — add event listeners in `onMounted`/`onUnmounted`

Find `onMounted` in the script block. Add mouse/touch event listeners to the video element:

```ts
onMounted(() => {
  // ... existing code
  videoRef.value?.addEventListener('mousedown', startHold)
  videoRef.value?.addEventListener('touchstart', startHold)
  document.addEventListener('mouseup', endHold)
  document.addEventListener('touchend', endHold)
})
```

In `onUnmounted`:

```ts
onUnmounted(() => {
  // ... existing cleanup
  videoRef.value?.removeEventListener('mousedown', startHold)
  videoRef.value?.removeEventListener('touchstart', startHold)
  document.removeEventListener('mouseup', endHold)
  document.removeEventListener('touchend', endHold)
  endHold() // clear any pending timers
})
```

Also call `resetBrightness()` when a new video starts loading — find the existing source switching logic (where `sources.value` is set or `loadVideo` is called) and add `resetBrightness()` there.

- [ ] **Step 1: Add mousedown/touchstart listeners to video element in onMounted**

- [ ] **Step 2: Add mouseup/touchend listeners to document in onMounted**

- [ ] **Step 3: Remove listeners in onUnmounted and clear timers**

- [ ] **Step 4: Call resetBrightness() when video source changes**

- [ ] **Step 5: Commit**

```bash
git add src/views/PlayerPage.vue
git commit -m "feat(player): wire up brightness hold event listeners"
```

---

## Verification

1. Run `npm run build` — should compile without errors
2. Open the app, navigate to a video, press and hold on the video area — brightness overlay should appear after 200ms
3. Drag the slider — video brightness should change in real-time
4. Release — overlay should disappear immediately
5. After 3 seconds of holding without interaction — overlay should auto-dismiss
6. Start a new video — brightness should reset to 100%
7. In live mode — hold should have no effect (brightness control disabled)