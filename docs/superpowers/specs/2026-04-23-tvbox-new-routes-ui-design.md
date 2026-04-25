# TVBox Media Center UI — New Routes Redesign

Date: 2026-04-23
Status: Draft

## Summary

This design covers four incremental improvements targeting the new library routes (`/library/movie`, `/library/series`, `/library/variety`, `/library/anime`). The existing live and hot routes remain unchanged. Each improvement is self-contained and can be implemented independently.

## Visual System Refinement

### Label Language

All UI labels use Chinese as primary language. English eyebrows are reserved only for brand moments (app title, section labels that repeat frequently).

| Element | Before | After |
|---------|--------|-------|
| Drawer header | "Source Drawer" eyebrow + "播放线路" heading | "线路切换" heading, no eyebrow |
| Current URL display | "Current Url" eyebrow + raw URL | "当前地址" label + truncated address (max 60 chars) |
| Source kind badges | "hls", "http", "external" (English) | "HLS", "HTTP", "外部" |
| Line indicators | "Line 1", "Line 2" (English) | "线路 1", "线路 2" |
| Status badges | English tone words | Chinese: "播放中", "等待", "需要处理", "就绪" |
| Episode chip states | English class comments | Chinese labels: "可播", "解析中", "不可用" |

### Content Layer vs System Layer Colors

The dual-track palette is already in `style.css`. This spec clarifies when to use which:

**Content Layer** (warm gold accents) — used for:
- Poster image overlays and gradients
- Hero backgrounds and titles
- Primary action buttons (`action-button-primary`)
- Episode chips in "playable" state (`episode-chip-playable`)
- "Continue Watching" progress bars

**System Layer** (cool blue-gray) — used for:
- Source health indicators
- Playback status badges (cool tone)
- Episode chips in "resolving" state (`episode-chip-resolving`)
- Playback notices and warnings
- Drawer borders and backgrounds

## Home Page — New Library Routes

### Layout

The new library routes share a common page structure:

```
[Topbar: App title + Subscription + Settings links]
[Category Nav: Movies | Series | Variety | Anime — no Live/Hot]
[Hero Section — featured content]
[Continue Rail — if any items in progress]
[Category Rail — scrollable horizontal cards]
[Live Shortcut — bottom anchor, minimal]
```

### Hero Section

Replaces the current "search bar + category grid" layout on new routes.

Structure:
- Left (60%): Featured item — large poster, title, summary (2 lines max), "立即播放" button
- Right (40%): Quick stats — enabled sources count, library item count, last refresh time

When no featured item exists, show a "最近更新" editorial card instead.

### Continue Rail

Shows items with playback history progress > 5%. Each card shows:
- Poster thumbnail
- Title
- Progress bar (gold gradient)
- Episode position label ("第 12 集")

If no items have progress, the rail is hidden entirely.

### Category Rail

Horizontal scrollable rail of `MediaCard` components. Shows 20 items max per rail. "加载更多" button at the end expands to full catalog page.

Rail header: category name + item count.

### Live Shortcut

Minimal bottom section: "跳转直播" text link + current live channels count badge. Single tap navigates to `/library/live`.

### Search Behavior

Search bar at top filters current category results in-place. No page reload. Results update after 300ms debounce.

## Detail Page — New Routes

### Page Structure

```
[Back button]
[Backdrop shell with poster blur]
  [DetailHero — poster left, info right]
  [RecommendedSourcePanel — if applicable]
  [EpisodeGroupPanel — recommended group expanded]
  [EpisodeGroupPanel — other groups collapsed]
```

### Source Hierarchy

The `RecommendedSourcePanel` gets stronger visual treatment:

- Border: 1px solid `rgba(216, 154, 87, 0.32)` (warm gold, not dashed)
- Background: subtle warm gradient overlay
- Badge: "推荐" in warm tone

Non-recommended groups:
- Collapsed by default with "展开 N 条线路" toggle
- Smaller header with source name only
- No warm border accent

### EpisodeChip States

Three states with clear visual distinction:

| State | Appearance | Label |
|-------|------------|-------|
| Playable | Gold border glow (`episode-chip-playable`), solid border | "可播" badge |
| Resolving | Blue-gray tint (`episode-chip-resolving`), dashed border | "解析中" badge |
| Unavailable | Faded opacity (`episode-chip-unavailable`), dashed border | No badge |

Clicking "解析中" shows a spinner in the chip. Clicking "不可用" shows a tooltip: "当前线路暂不可用，试试切换到推荐源"。

## Player Page — New Routes

### Layout

```
[Topbar: Back + Mode label + Source label + Status]
[Stage: Video (16:9) + Vignette overlay]
[Overlay: Error notice (if any) + Controls]
[Drawer: Source list (collapsible on small screens)]
```

### Error Language Refinement

All error messages rewritten in player-first language:

| Scenario | Before | After |
|----------|--------|-------|
| Line expired | "该线路已过期" | "当前线路已过期，自动切换下一条" |
| External required | "该线路需要外部工具处理" | "此资源需要外部播放器，点击确认打开" |
| All failed | "所有线路均不可用" | "所有线路均无法播放，请稍后重试或切换源" |
| Autoplay blocked | Raw browser message | "浏览器限制了自动播放，请点击播放按钮" |
| Resolving | "解析中" | "正在解析地址..." with spinner |

### Recovery Visibility

When auto-switching to next line after failure, show a transient notice:

"正在切换线路 2..." (3 second display, then fade)

When all candidates are exhausted: final failure state with count "已尝试 N 条线路" + retry button.

### Drawer Cleanup

- Remove "Current Url" raw URL display — replace with truncated address (max 60 chars, ellipsis)
- Remove English eyebrows entirely
- Source kind badges use Chinese: "HLS", "HTTP", "外部"
- "线路 N" labels (not "Line N")

## Component Inventory

New or changed components:

| Component | Status | Location |
|-----------|--------|----------|
| `HomeHero` | New | `src/components/home/HomeHero.vue` |
| `ContinueRail` | New | `src/components/home/ContinueRail.vue` |
| `CategoryRail` | New | `src/components/home/MediaRail.vue` (extends existing) |
| `LiveShortcut` | New | `src/components/home/LiveNowPanel.vue` (extends) |
| `SourceBadge` | Existing, relabel | `src/components/media/SourceBadge.vue` |

## Implementation Order

1. Visual system cleanup (labels, colors) — no new components, only style.css + individual component text changes
2. New Home layout for library routes — new components, new route-level logic
3. Detail page source hierarchy — `RecommendedSourcePanel` styling + `EpisodeGroupPanel` collapse behavior
4. Player error language + drawer cleanup — string changes + small style adjustments

## Acceptance Criteria

- New routes (`/library/movie`, `/library/series`, `/library/variety`, `/library/anime`) show Hero + Continue Rail + Category Rail structure
- All player error messages use Chinese, player-first phrasing
- Source kind badges show Chinese labels (HLS/HTTP/外部)
- Drawer shows "线路 N" not "Line N", no English eyebrows
- EpisodeChip states are visually distinct and labeled in Chinese
- Recommended source panel has warm gold border accent
- Non-recommended episode groups are collapsed by default