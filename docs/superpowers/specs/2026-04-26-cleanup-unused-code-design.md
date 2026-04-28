# Code Cleanup Design - 2026-04-26

## Context

After recent architecture and UI changes, several pieces of code are no longer used. This spec defines what to clean up.

## Scope

### Frontend (src/)

| File | Status | Reason |
|------|--------|--------|
| `stores/vod.ts` | DELETE | Never imported by any view |
| `stores/player.ts` | DELETE | Never imported by any view |
| `stores/douban.ts` | DELETE | Only used by HotDetail.vue |
| `views/HotDetail.vue` | DELETE | User confirmed - dedicated Douban hot detail page is no longer needed |
| `components/home/ContinueRail.vue` | DELETE | Never imported anywhere |
| `components/home/HomeHero.vue` | DELETE | Never imported anywhere |
| `components/home/MediaRail.vue` | DELETE | Never imported anywhere |
| `components/home/SourceHealthPanel.vue` | DELETE | Never imported anywhere |

### Frontend References to Clean

- `src/router/index.ts` - Remove `/detail/hot/:doubanId` route (HotDetail)
- `src/types/index.ts` - `DoubanHotItem` interface can be removed (only used by deleted stores)

### Backend (src-tauri/src/)

| File | Status | Reason |
|------|--------|--------|
| `services/search.rs` | DELETE | Only used by commands/douban.rs for HotDetail feature |
| `commands/douban.rs` | KEEP | Used by library.ts for main home page Douban hot display |
| `services/douban.rs` | KEEP | Used by commands/douban.rs for main home page |

### Backend References to Clean

- `src-tauri/src/services/mod.rs` - Remove `pub mod search` and `pub use douban::DoubanCrawler` (not needed since search.rs is deleted, but DoubanCrawler is still used by commands)
- Actually keep `pub use douban::DoubanCrawler` since commands/douban.rs still uses it
- Only remove `pub mod search`

## Clarifications

- `library.ts` store uses Douban backend commands directly - NOT through `stores/douban.ts`
- `Home.vue` renders Douban hot content via `libraryStore.doubanHot` - this functionality is preserved
- The separate `/detail/hot/:doubanId` route and `HotDetail.vue` page are what get removed

## Implementation Order

1. Delete frontend files (stores, components, views)
2. Update router to remove HotDetail route
3. Clean types/index.ts
4. Delete src-tauri/src/services/search.rs
5. Update src-tauri/src/services/mod.rs to remove search module
6. Run build to verify no breaking changes
