# AGENTS.md

Read before coding:

1. PRODUCT_SPEC.md
2. ARCHITECTURE.md
3. DESIGN.md
4. DEVELOPMENT.md

## Product Constraints

Do not add unrequested features.

Explicitly out of scope unless requested:
- tags
- graph view
- backlinks UI
- plugins
- collaboration
- comments
- AI features
- additional OAuth / SSO providers beyond the implemented Cloudflare Access login
- external calendar integrations
- analytics
- streaks
- Notion-style databases

## Technical Constraints

Primary target:
- Rust
- Cloudflare Workers
- WASM
- Leptos CSR (browser WASM)
- JSON API on Rust Workers
- Static Assets for the browser application
- R2
- D1

The user explicitly selected Leptos CSR on 2026-09-14. Browser WASM owns rendering and UI state; Worker WASM owns data reads/writes. Do not restore SSR/HTMX without a new architecture decision.
Keep UI-only changes local; avoid duplicate reads, polling and per-keystroke saves.

## Storage Rules

Markdown at its vault path is canonical in both local filesystem and R2 storage. `.folio/` also contains primary standalone tasks/goals and recovery journals.

D1/SQLite contains derived note/search/planner indexes **and SQL-owned authentication, refresh sessions, rate limits and mutation leases**. Do not treat the entire database as disposable. See [storage and recovery](ARCHITECTURE.md#canonical-storage).

Do not scan Markdown bodies for routine:
- note list
- search
- todo
- recently edited
- home

Folder/file browsing lists canonical storage keys with cached SQL metadata so files remain discoverable after index loss. Search and planner use bounded SQL queries.

## Portability Rules

Keep Cloudflare-specific APIs at infrastructure boundaries.

Preserve the already implemented runtime adapters:
- Cloudflare: R2 + D1
- Native: filesystem + SQLite

## UI Rules

The product is a personal planner, not a dashboard.

Use:
- calm spacing
- warm neutral background
- restrained green accent
- planner-like task rows
- strong note typography

Avoid:
- gradients
- glassmorphism
- heavy cards
- analytics widgets
- colorful category systems

## Implementation Rules

Before implementing a feature:
1. Verify it exists in PRODUCT_SPEC.md.
2. Respect architecture boundaries.
3. Choose the simplest implementation.
4. Avoid speculative abstractions.

For note saves:
- canonical Markdown must not be lost
- account for R2 success / D1 failure
- do not save per keystroke

For editor:
- maintain dirty state
- debounce saves
- expose failed save

## Code Quality

- small functions
- explicit errors
- testable domain logic
- minimal dependencies
- no giant utils modules
- update docs when behavior changes
