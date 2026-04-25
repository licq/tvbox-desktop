# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

TVBox is a Tauri desktop application (Vue 3 frontend + Rust backend) for streaming media. It aggregates multiple Chinese streaming sources with runtime-aware playback resolution.

## Commands

```bash
npm run dev        # Start Vite dev server (port 1420)
npm run build      # TypeScript check + Vite build
npm run test       # Run vitest
npm run tauri dev  # Run Tauri app in development
npm run tauri build # Build production binary
vitest run src/stores/__tests__/library.spec.ts  # Run single test file
```

## Architecture

**Frontend**: Vue 3 + Pinia + Vue Router + Tailwind CSS, served from `/dist`
**Backend**: Rust + Tauri 2.x, source in `src-tauri/src/`

### Frontend Structure (src/)
- `views/` - Page components (Home, Vod, Live, PlayerPage, VodDetail, Settings, Subscriptions)
- `stores/` - Pinia stores (library, playback, detail, player, vod, live, douban, subscription)
- `components/home/` - Home page sections (Hero, ContinueRail, MediaRail, LiveNowPanel, SourceHealthPanel)
- `components/detail/` - Detail page (DetailHero, EpisodeGroupPanel, RecommendedSourcePanel)
- `components/player/` - Player UI (PlaybackDrawer, PlaybackNotice)
- `components/media/` - Shared media components (MediaCard, EpisodeChip, SourceBadge)
- `utils/` - Player helpers, fullscreen utilities
- `@/` path alias maps to `src/`

### Backend Structure (src-tauri/src/)
- `lib.rs` - AppState with Storage, exports Parser and Storage services
- `commands/` - Tauri command handlers (subscription, vod, live, player, douban)
- `services/` - Source scrapers and playback runtime:
  - `tvbox.rs`, `xb6v.rs`, `auete.rs`, `libvio.rs`, `jianpian.rs`, `wencai.rs`, `zxzj.rs` - Source scrapers
  - `guard.rs`, `guard_jpj.rs`, `guard_jpys.rs` - Playback guard/DRM handlers
  - `douban.rs` - Douban hot list crawler
  - `parser.rs`, `resolver.rs` - Catalog parsing and playback resolution
  - `storage.rs` - SQLite persistence
  - `playback_runtime.rs`, `playback_types.rs` - Runtime playback management

### Routes
- `/` → redirects to `/library/live`
- `/library/:type` - Library home (type: live, movie, series, variety, anime)
- `/player/:mode/:id` - Player page
- `/detail/:itemId` - VOD detail page
- `/subscriptions` - Source subscriptions
- `/settings` - App settings

## Key Patterns

- **Store normalization**: Library store normalizes external API payloads into owned copies (tests verify immutability)
- **Source scrapers**: Each source is a separate Rust module with `is_<source>_site()`, `scrape_<source>_catalog()`, `scrape_<source>_detail()`, and `extract_player_url()` functions
- **Guard system**: Sources with playback protection use guard adapters (`guard_jpj`, `guard_jpys`) that encode/decode playback targets
- **Playback resolution**: `resolver.rs` classifies playback targets and determines visibility/sort rank
- **TypeScript strict**: `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true` - all types must be explicit

## Testing

Uses vitest with jsdom environment. Store tests require `setActivePinia(createPinia())` before using stores.

## Documentation

Design specs and implementation plans are in `docs/superpowers/specs/` and `docs/superpowers/plans/`. The media center UI redesign is documented in `2026-04-23-tvbox-media-center-ui-design.md`.