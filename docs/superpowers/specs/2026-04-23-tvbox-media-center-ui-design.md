# TVBox Media Center UI Redesign

Date: 2026-04-23
Status: Approved for planning

## Summary

This redesign shifts the app from a tool-like source console into a desktop media center. The primary goal is not cosmetic polish in isolation; it is to make the product feel like a player and library first, while still carrying source health, playback fallback, and subscription complexity in a controlled way.

The design principle is:

- First perception: content
- Second perception: playback
- Third layer: source and runtime state

The redesign covers four surfaces:

- Home
- Detail
- Player
- Shared visual system

It does not change playback runtime rules or source parsing behavior. It changes how those capabilities are prioritized, framed, and exposed to the user.

## Goals

- Make the app feel like a mature desktop media center rather than an internal operations dashboard.
- Reorganize navigation so users decide what to watch before they manage subscriptions or inspect source state.
- Present playback confidence and failure information as part of player UX instead of raw technical logging.
- Reduce visual sameness across pages by assigning each page a clear job and different composition emphasis.
- Build a reusable UI system so future pages do not drift back into ad hoc Tailwind assembly.

## Non-Goals

- No change to source ingestion architecture in this spec.
- No new recommendation engine or personalization logic.
- No new social, rating, or collection features beyond what is needed to support layout.
- No attempt to make embedded or external-only sources feel first-class in the player.

## Product Positioning

The app should behave like a desktop media center with a lightweight runtime-aware control layer.

This means:

- Users come here to pick content and play it.
- Source health exists to support confidence, not to dominate the interface.
- Runtime state is visible when useful, but it is translated into player language.
- Administrative actions are accessible but visually secondary.

## Information Architecture

### Primary User Flow

The main flow becomes:

1. Decide what to watch
2. Decide which line or source to enter through
3. Start playback
4. If playback fails, let the system recover and explain the outcome clearly

### Navigation Model

The app navigation should stop presenting every surface as a top-level equal-weight mode switch.

Primary destinations:

- Home
- Movies
- Series
- Variety
- Anime
- Live

Secondary destinations:

- Subscriptions
- Settings

`Hot` content should be expressed as a section or rail inside Home, not as a peer mode competing with library and playback entry points.

## Home Design

### Purpose

Home is the decision page. It answers: what should I watch right now?

It should not feel like a tabbed control panel. The page should prioritize content appetite, continuity, and lightweight awareness of system health.

### Layout

Home uses a five-section structure:

1. Hero
2. Continue Watching
3. Library Rails
4. Live Now
5. Source Health

### Hero

The hero anchors the page and should feel editorial, not administrative.

Structure:

- Left: featured or continue-watching content with large title, short summary, and primary actions
- Right: a compact status stack with only high-value signals

Allowed status metrics:

- enabled source count
- library availability health
- recent failures requiring attention

The hero must not contain dense navigation or long diagnostic copy.

### Continue Watching

This section appears before category browsing because it has the highest user value.

Each card should show:

- poster
- title
- progress
- lightweight confidence signal for last-used line

This section is the strongest expression of “desktop media center” continuity.

### Library Rails

Category browsing should use horizontal rails rather than generic card blocks.

Required rails:

- Movies
- Series
- Variety
- Anime

Each rail shows a focused slice:

- recently updated
- editor-picked
- or runtime-confident entries

Home should not attempt to show full catalog density.

### Live Now

Live remains important but should no longer define the page.

This section should present:

- a handful of high-frequency channels
- shortcuts into grouped live browsing
- at-a-glance live entry points

The role of the section is to say “jump into live quickly,” not “manage live infrastructure.”

### Source Health

Source health moves to a secondary section near the bottom of the page or into a right-side secondary column where space permits.

It should include:

- recent refresh time
- enabled vs failing subscriptions
- actionable abnormal items

It should not dominate first-screen attention.

## Detail Design

### Purpose

Detail is the decision page for entry strategy. It answers:

- is this worth watching?
- which source or line should I enter through?
- does this episode look playable?

### Layout

The page is organized into three zones:

1. Content header
2. Source decision area
3. Episode matrix

### Content Header

Structure:

- Left: poster
- Center: title, summary, metadata, tags
- Right: primary actions

Primary actions:

- play now
- resume

Future-friendly but optional:

- save for later
- favorite

The header should prioritize content judgment rather than technical state.

### Source Decision Area

This replaces the current flat “all groups at once” mentality.

The page should first surface:

- recommended source
- why it is recommended

Recommendation reasons may include:

- recently successful
- higher runtime confidence
- direct or resolvable playable output

Other source groups should remain accessible but visually subordinate.

This makes source selection a guided choice rather than a blind lottery.

### Episode Matrix

Episodes remain grouped by source and season or playback group where relevant.

Behavior:

- only the recommended group is expanded by default
- alternate groups remain collapsed or secondary
- episode chips should carry status meaning

Episode chip states:

- playable
- resolving
- currently unavailable

The user should not need to enter the player just to learn a known dead episode is dead.

## Player Design

### Purpose

Player is for consumption first and recovery second. It should stop feeling like a debug console.

### Layout

The page has three layers:

- central playback surface
- lightweight controls
- secondary source drawer

### Central Playback Surface

The video area remains dominant. Surrounding chrome should be restrained.

The player must feel calm and cinematic when playback works.

### Source Drawer

Candidate switching moves into a right-side or bottom drawer rather than occupying large permanent space.

The drawer contains only:

- current episode
- current line
- alternate playable candidates
- brief status explanation

It should not expose excessive runtime internals by default.

### Failure Language

Failure states must be rewritten in player terms.

Examples:

- line has expired
- browser cannot directly access this resource
- this episode requires an external tool
- trying the next playable line

Technical context may exist as secondary text, never as the dominant message.

### Recovery Visibility

When the runtime auto-falls back, the player should show that the system is doing work on the user’s behalf.

Required moments:

- trying next line
- how many candidates were skipped
- final success or final failure

This creates trust and reduces the feeling of random breakage.

## Visual System

### Design Direction

Use a dual-track visual language:

- warm cinematic content layer
- cool technical state layer

The two systems should complement rather than fight each other.

### Content Layer

Used for:

- posters
- hero surfaces
- titles
- category rails
- immersive backgrounds

Characteristics:

- warm gold accents
- off-white content text
- film-like dark surfaces
- generous composition and hierarchy

### System Layer

Used for:

- source state
- playback health
- runtime notices
- secondary operational surfaces

Characteristics:

- muted blue-gray states
- controlled contrast
- sharper functional semantics
- less decorative emphasis

### Typography

Typography should become heavier and more directional.

Rules:

- large, stable page titles
- short secondary copy
- better separation between editorial text and system text
- fewer all-purpose medium-size labels

### Component System

Shared primitives should be formalized around page purpose.

Required components:

- Hero
- Rail
- MediaCard
- SourceBadge
- EpisodeChip
- PlaybackDrawer
- StatusNotice

These components should become the only allowed building blocks for the redesigned surfaces.

## Interaction Principles

- Each page should have one dominant job.
- Primary actions should remain visually obvious.
- Source complexity must be progressively disclosed, not dumped all at once.
- Runtime state must support user confidence, not overwhelm it.
- Visual emphasis should go to content first, then playback, then diagnostics.

## Responsive Behavior

### Desktop

Desktop is the primary target. The composition should use:

- large hero surfaces
- visible rails
- multi-column layout for supporting panels
- drawers for line management

### Smaller Widths

On smaller widths:

- hero collapses to vertical stack
- source health and side panels move below content
- episode groups remain grouped but reduce visible density
- drawers may become bottom sheets

The visual identity should remain consistent rather than degrading into generic stacked cards.

## State Expression

Source and playback state remain a product differentiator, but they must be translated into UI semantics.

State hierarchy:

- passive confidence states
- warnings that suggest caution
- active failures that change actionability

Examples:

- source healthy
- source unstable
- episode likely playable
- episode currently unavailable
- external tool required

The UI must avoid raw backend phrasing except in secondary diagnostic surfaces.

## Implementation Boundaries

This spec is intended to drive a later implementation plan covering:

- Home restructure
- Detail restructure
- Player restructure
- shared visual system and component refactor

It should not be split further before planning unless the implementation plan reveals the need for separate tracks.

## Acceptance Criteria

- The first-screen experience reads as a media center, not a source operations dashboard.
- Home prioritizes continue-watching and library browsing over subscription management.
- Detail guides users toward a recommended source instead of showing all lines as equal.
- Player communicates failures in player language and makes fallback behavior visible.
- Shared components reduce repeated one-off visual assembly across major surfaces.
- Source health remains accessible without overtaking content-first flow.
