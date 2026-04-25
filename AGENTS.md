# TVBox Developer Guide

## Project Overview

TVBox is a Tauri desktop application (Vue 3 frontend + Rust backend) for streaming media. It aggregates multiple Chinese streaming sources.

## Commands

```bash
npm run dev        # Start Vite dev server (port 1420)
npm run build      # TypeScript check + Vite build
npm run test       # Run vitest
npm run tauri     # Run Tauri CLI (e.g., tauri dev, tauri build)
```

## Architecture

- **Frontend**: Vue 3 + Pinia + Vue Router + Tailwind CSS
- **Backend**: Rust + Tauri 2.x
- **Source dir**: `src/` (Vue)
- **Backend dir**: `src-tauri/src/` (Rust)
- **Tests**: `src/stores/__tests__/*.spec.ts`, `src/utils/__tests__/*.spec.ts`

## Path Aliases

`@/` maps to `src/` (configured in vite.config.ts and tsconfig.json)

## Testing

- Uses **vitest** with jsdom environment
- Run single test: `vitest run src/stores/__tests__/library.spec.ts`
- Store tests need `setActivePinia(createPinia())` before using stores

## Tauri Commands

Backend commands are registered in `src-tauri/src/main.rs`. Key modules:
- `commands/subscription.rs` - Source subscription management
- `commands/vod.rs` - VOD catalog and search
- `commands/live.rs` - Live TV channels
- `commands/player.rs` - Playback resolution and history
- `commands/douban.rs` - Douban hot list integration

## Key Patterns

- **Store normalization**: Library store normalizes external API payloads into owned copies (tests verify immutability)
- **Source scrapers**: Implemented in `src-tauri/src/services/` as separate modules
- **Playback**: Uses hls.js for HLS streams, with fallback to external players

## Documentation

Design specs and implementation plans are in `docs/superpowers/specs/` and `docs/superpowers/plans/`.

## TypeScript Strict

`tsconfig.json` has `strict: true`, `noUnusedLocals: true`, `noUnusedParameters: true`. All types must be explicit.
