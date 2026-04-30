# TVBox Desktop

TVBox Desktop is a Tauri application for browsing and playing media from multiple Chinese streaming sources. It combines a Vue 3 frontend with a Rust backend and includes a Douban-powered discovery flow, provider search, live channels, playback, and local history/cache support.

## Features

- Home page with category tabs for movie, series, variety, and anime.
- Douban hot list discovery and detail-page enrichment.
- Provider search across enabled sources.
- Live TV channel browsing and playback.
- VOD detail pages with episode groups and playback targets.
- Local SQLite persistence for subscriptions, catalog data, playback history, and caches.

## Tech Stack

- Frontend: Vue 3, TypeScript, Pinia, Vue Router, Tailwind CSS, Vite
- Backend: Rust, Tauri 2.x, rusqlite, reqwest, tokio
- Testing: Vitest, jsdom

## Prerequisites

- Node.js 18+ recommended
- Rust toolchain with Cargo
- Tauri prerequisites for your platform

## Getting Started

Install dependencies:

```bash
npm install
```

Run the frontend-only dev server:

```bash
npm run dev
```

Run the full desktop app in development:

```bash
npm run tauri -- dev
```

## Build

Frontend build:

```bash
npm run build
```

Tauri production build:

```bash
npm run tauri -- build
```

## Test

Run the test suite:

```bash
npm run test
```

Run a single store test file:

```bash
npm run test -- --run src/stores/__tests__/library.spec.ts
```

## Project Structure

```text
src/            Vue frontend
src/components/ UI components
src/views/      Pages and routes
src/stores/     Pinia stores
src-tauri/src/  Rust backend commands, services, and models
docs/           Design specs and implementation plans
```

## Notes

- The app depends on external sources, so some pages may be empty if upstream sites change or temporarily block requests.
- Douban data is cached locally and seeded on demand when the local cache is empty.
- Some pages use source-aware navigation, so detail and playback flows may vary by provider.

## Documentation

Additional design and implementation notes live under `docs/superpowers/specs/` and `docs/superpowers/plans/`.

## GitHub Releases

The repository includes a manual GitHub Actions workflow for building unsigned release artifacts for Windows, macOS, and Linux.

1. Open the `Actions` tab in GitHub.
2. Run the `release` workflow manually.
3. Set `ref` to the branch, tag, or commit SHA you want to build.
4. Set `tag_name` to the release tag you want GitHub to create or reuse.

The workflow creates a draft GitHub Release and uploads the platform artifacts produced by Tauri.

Use a stable release tag such as `v1.2.3` for `tag_name`. Re-running the workflow with the same tag will reuse the same draft release, while a new tag creates a new draft release.
