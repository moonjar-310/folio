# Architecture

## Decision — 2026-09-14

Leptos CSR runs in browser WASM. A Rust Worker serves JSON data and stores notes; it does not render HTML. This supersedes the earlier SSR/HTMX plan, following the user's explicit request to keep UI work in the browser and minimize Worker calls.

## Runtime and deployment

Browser → Static Assets: HTML shell, WASM, JavaScript loader, CSS.
Browser → /api/* → Rust Worker → D1 / R2.

One Workers project hosts both, but `run_worker_first` matches only `/api` and `/api/*`. Static files and SPA navigation fallback do not invoke the application Worker. Initial note listing uses one API call; opening a note fetches its Markdown; saving returns metadata so the list updates without another read. UI-only state changes do not call the API.

- `crates/web`: Leptos CSR, Trunk build, client state and fetch calls.
- `crates/core`: serializable contracts, validation and metadata extraction; no runtime-specific dependencies.
- `crates/worker`: Cloudflare fetch entrypoint and storage bindings, `wasm32-unknown-unknown`.
- `crates/local`: native Axum/Tokio HTTP adapter and independent local storage.

Cloudflare and local use the same contracts and validation. They have different runtime entrypoints and storage implementations. The Worker WASM binary is not a standalone local server.

## Storage

Cloud: R2 `notes/{id}.md` is canonical, D1 stores derived metadata.
Local: filesystem `vault/{id}.md` is canonical, SQLite stores the same index schema.

IDs contain only ASCII letters, digits and hyphens, up to 64 bytes. No user-controlled filesystem paths. Titles/previews are derived from Markdown. The bootstrap intentionally uses a flat ID-based vault; folder and rename support comes later.

Read paths:
- List → SQL index, newest 50, no object scans.
- Open → one canonical Markdown object/file.
- Save → validate → canonical Markdown write → metadata upsert → summary JSON.

## API

| Method | Path | Result |
| --- | --- | --- |
| GET | /api/health | Runtime health, no storage reads |
| GET | /api/notes | At most 50 NoteSummary entries |
| GET | /api/notes/{id} | Note containing id and markdown |
| PUT | /api/notes/{id} | SaveNote body; returns NoteSummary |

Markdown is bounded to 128 KiB; JSON request bodies are separately bounded including escaped characters. Saves are explicit in this bootstrap. No autosave per keystroke or polling. A successful save updates the browser list from its response. In-flight operations disable editor writes/navigation to prevent a stale response from overwriting newer input.

## Failures and concurrency

A failed request keeps the active draft and dirty state. Switching notes prompts before discarding edits. Browser unload also prompts when dirty. Persistent browser draft recovery and retry UX beyond explicit Save remain future work.

Canonical data is written before indexing. An index failure must not remove canonical Markdown. Retry Save repairs metadata. Local writes use a temporary file and atomic replacement; local storage operations are serialized under a mutex and run outside the async executor. Cloud writes currently use last-write-wins semantics. ETags/version conflicts and recovery/re-index tooling are required before multi-client use.

## Security boundary

The standalone server binds only to 127.0.0.1. The Worker note API is disabled unless APP_ENV=development, intended only for local Wrangler emulation. Production defaults fail closed. There is no implemented login/session system yet.

Future production authentication is single-user username/password with hashed passwords, HttpOnly/Secure/SameSite session cookies and CSRF protection. Do not treat the development gate as authentication. Do not expose the development override publicly.

## Remaining product phases

Authentication, autosave/shortcuts, full editor preview, folder operations, SQL pagination/search, Markdown task indexing, Home, Todo, Planner and Goals must be implemented against these shared data contracts. The Node preview and Penpot templates are design references, not the Rust application's completion status.

## Resource rules

- Serve public static assets without executing the Worker.
- Keep UI-only state in the browser.
- Avoid duplicate initial queries and refetch-after-save.
- Bound lists and request bodies.
- Do not scan R2 for list/search/home.
- Batch user edits into saves; do not save per keystroke.
- Measure Worker requests, CPU, SQL reads/writes and R2 operations separately.
