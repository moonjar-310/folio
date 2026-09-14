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
- OAuth / SSO
- external calendar integrations
- analytics
- streaks
- Notion-style databases

## Technical Constraints

Primary target:
- Rust
- Cloudflare Workers
- WASM
- SSR HTML
- HTMX
- minimal JS
- R2
- D1

Avoid SPA frameworks.

Do not ship Rust WASM to browser unless explicitly justified.

## Storage Rules

R2 Markdown is canonical.

D1 is derived/query data.

Do not fetch all R2 documents for:
- note list
- search
- todo
- recently edited
- home

## Portability Rules

Keep Cloudflare-specific APIs at infrastructure boundaries.

Preserve future replacements:
- R2 → filesystem
- D1 → SQLite

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
