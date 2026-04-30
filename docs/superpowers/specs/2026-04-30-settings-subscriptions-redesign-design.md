# Settings and Subscriptions Redesign Design

## Context

`src/views/Settings.vue` and `src/views/Subscriptions.vue` still use older flat layouts and mixed visual treatment. They do not match the richer cinematic system already established in the home and detail pages. This redesign keeps the same dark, warm, glass-like language from `src/style.css` and reshapes both pages into a consistent product shell.

## Goals

- Make both pages feel native to the existing app visual system.
- Reduce the rough, form-like look of the current settings page.
- Turn subscriptions into a task-oriented panel with clear status, add, refresh, enable, and delete actions.
- Keep the redesign limited to layout, hierarchy, and style. Do not change core data flow or backend behavior.

## Non-Goals

- No new settings persistence model.
- No new subscription actions beyond the current add, refresh, toggle, and delete flow.
- No route changes.
- No redesign of unrelated pages.

## Shared Visual System

Both pages will use the same page frame:

- A compact hero header with an eyebrow, a strong title, and one-line helper copy.
- Right-aligned primary and secondary actions in the header area.
- Content organized into `surface-panel` cards.
- Warm accent highlights, rounded corners, soft borders, and subtle glass blur.
- Consistent handling for loading, empty, and destructive states.

The redesign should reuse existing tokens and utility classes from `src/style.css` wherever possible:

- `app-shell`
- `surface-panel`
- `surface-muted`
- `eyebrow`
- `section-title`
- `action-button`
- `action-button-primary`
- `action-button-secondary`

## Settings Page Design

The settings page becomes a settings hub instead of a flat list of sections.

### Structure

1. Hero header
   - Title: `设置中心`
   - Subtitle: brief copy that explains this page controls playback, appearance, and cache behavior.
   - Back action stays in the top-left area.

2. Main grid
   - Desktop: two-column layout.
   - Left column: playback and appearance cards.
   - Right column: cache and about cards.
   - Mobile: single column stacked cards.

### Section Content

- Playback card
  - Keep the existing playback quality select and hardware decode toggle.
  - Present them as row-based form controls with labels and short helper text.
  - Add a compact description line so the card feels deliberate rather than bare.

- Appearance card
  - Keep the theme select.
  - Present it in the same row style as playback settings.

- Cache card
  - Show each cache item as a task row with label, short status text, and a destructive clear button.
  - Keep the two existing cache actions.
  - Make destructive actions visually distinct but still consistent with the app language.

- About card
  - Present version and build stack information in a more polished summary block.
  - Include a small footer note rather than a plain paragraph list.

## Subscriptions Page Design

The subscriptions page becomes a task panel with a summary header and clearly separated control regions.

### Structure

1. Hero header
   - Title: `订阅任务面板`
   - Subtitle: explain that subscriptions can be added, refreshed, enabled, or removed from this page.
   - Primary action: add subscription.
   - Secondary action: back to the library/home area.

2. Summary strip
   - Show top-level counts:
     - total subscriptions
     - enabled subscriptions
     - disabled subscriptions
     - current refresh state if active
   - This strip should sit above the list and feel like a compact dashboard row, not a separate page.

3. Task panel
   - The add form is promoted to a visible panel when opened.
   - The form should feel like a focused task area with clear labels and a full-width submit button.
   - Error alerts remain unchanged for now.

4. Subscription list
   - Each subscription is rendered as a wide task card.
   - Left side: enable toggle, subscription name, and URL.
   - Right side: refresh and delete actions.
   - Each card should surface state more clearly than the current row layout.

### Subscription Card Details

- Name should be the strongest text element.
- URL should be smaller, muted, and truncated.
- A small state badge or text hint should show whether the item is enabled.
- Refresh state should remain visible during refresh and should not disappear into the card background.

### Empty and Loading States

- Loading state should use a centered spinner inside the main content area.
- Empty state should become a polished empty panel with a short explanation and the add action.
- The current inline “刷新中” banner should be integrated into the summary strip or page header so it feels less bolted on.

## Interaction Rules

- Keep the current store and invoke calls unchanged unless the layout requires small binding adjustments.
- Keep destructive actions behind the existing confirmation prompt.
- Do not add new persistent state or new API calls.
- Preserve the current back-navigation behavior.

## Error Handling

- Continue to surface failed actions with alerts for now.
- Do not hide failures behind generic empty states.
- If refresh fails, keep the page usable and leave the existing subscription list visible.

## Testing

- Update or add Vue tests only if the layout refactor changes observable behavior.
- At minimum, run the existing build after implementation.
- If a loading or empty state component is extracted, cover it with a focused unit test.

## Implementation Notes

- This redesign should primarily be a template and style refactor in `src/views/Settings.vue` and `src/views/Subscriptions.vue`.
- Shared helper styles should be added to `src/style.css` if a repeated card or row pattern emerges.
- The changes should not alter the current business logic in `src/stores/subscription.ts`.
