# Folio

The application now starts with **Leptos CSR + a Rust JSON API**. The browser owns rendering and UI state. Cloudflare Worker execution is restricted to `/api` and `/api/*`; the browser WASM, JavaScript loader, CSS and SPA shell are served as Static Assets.

## Rust application bootstrap

- `crates/web`: Leptos CSR note list/editor, explicit save, loading/error/dirty states, draft-discard protection. No polling or per-keystroke requests.
- `crates/core`: shared JSON contracts, path-safe IDs, byte limits and derived metadata.
- `crates/worker`: workers-rs JSON API, canonical R2 Markdown and D1 metadata.
- `crates/local`: independent loopback Axum server, filesystem Markdown and SQLite metadata. No Cloudflare account required.
- `migrations`: schema shared by D1 and SQLite.

Requirements: Rust 1.91+, `wasm32-unknown-unknown`, Trunk 0.21+, Node.js 22+. `Cargo.lock` and `package-lock.json` pin dependencies.

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk --version 0.21.14
npm ci
npm run app:build
npm run app:local
```

Open `http://127.0.0.1:8788`. Notes persist in `.local/folio-app/vault/*.md`; the index is `.local/folio-app/index.sqlite`. Override `FOLIO_DATA_DIR`, `FOLIO_ASSETS_DIR` and `FOLIO_PORT` if needed. For live frontend development, keep the local API running and run `npm run app:dev` in a second terminal; Trunk proxies `/api` to port 8788.

For the Cloudflare runtime locally:

```sh
npm run worker:migrate:local
npm run worker:dev
```

Wrangler builds the server WASM and emulates D1/R2 locally. Build the frontend first. Wrangler and native local mode use separate local data stores.

```sh
cargo fmt --all --check
cargo test --locked
npm run worker:check
npm run app:build
```

With a local server running, `node scripts/smoke-api.mjs http://127.0.0.1:8788` exercises the static shell, save/read/list API, origin checks and size limits. Use port 8787 for Wrangler. It writes a test note, so use disposable local storage. `npx wrangler deploy --dry-run --outdir .local/worker-check` validates packaging without deploying.

Verified: Rust tests and Clippy, browser WASM release build, Worker packaging dry-run, native and Wrangler API smoke tests, and browser edit/save/reload. Authentication and a live Cloudflare deployment are not part of this bootstrap.

### Bootstrap boundary

This is an initial working notes slice, not the completed planner or a production deployment. The editor saves explicitly; autosave, task indexing, folders, pagination beyond the first 50 notes, authentication and multi-client conflict detection remain future work. Saves currently use last-write-wins semantics. Failed saves retain the current browser draft; closing the tab still requires the user's discard confirmation and there is no persistent browser draft recovery yet.

The cloud note API **fails closed by default** until authentication is implemented. `APP_ENV=development` is an explicit local Wrangler override, never a production setting. Before future deployment, configure real D1/R2 resources and implement session authentication/CSRF protection. `workers_dev=false` prevents an accidental public preview endpoint; no resources are created or deployed by the build commands.

Both backends write canonical Markdown before metadata. An index failure returns an error without deleting Markdown; retrying Save repairs the metadata. The list reads only the bounded SQL index, never scans the vault/R2. Cloudflare-specific types stay outside the shared core.

## Previous design preview

A server-rendered UI preview extracted from the user's Minimal Paper Planner Stitch project. See [project docs](docs/README.md) for the product and production architecture.

## Run

Requires Node.js 22 or newer. No packages to install.

```sh
npm run dev
```

Open http://127.0.0.1:4173. The preview binds only to localhost.

```sh
npm run check
npm test
npm run design:export
```

## Included

- Home, daily/weekly planner, task groups, notes browser/editor, settings, and `/design-system`.
- Reusable HTML components in `ui/components.mjs`; shared CSS tokens in `public/tokens.css`.
- Native forms, server-rendered pages, minimal JavaScript for editor autosave, shortcuts, and navigation drawer. No SPA framework or browser WASM.
- Local task creation/completion/date changes, goal creation/completion, quick notes, note search/rename/move/archive, Markdown preview, and notes export.
- Editor saves after three seconds idle, on blur, navigation, and Ctrl/Cmd+S. Failed saves retain dirty state and a browser draft backup.
- Light/Dark themes, system preference on first visit, persistent manual toggle, and Folders navigation.
- Private Stitch exports are retained locally in ignored `design/stitch/`; they are not included in the public repository.
- Editable SVG assets, token JSON, and native Penpot assembly scripts in `design/penpot/`. The private Penpot file contains native reusable components, desktop/mobile screens, and Light/Dark foundations.

## Preview boundary

`preview/` is a development-only Node HTTP adapter so the UI can be inspected without Cloudflare credentials. Its JSON data lives in ignored `.local/preview.json`. Do not deploy this adapter as the production service.

The former Node preview remains separate from the new Rust application. The current architecture is Leptos CSR with a Rust data API, canonical Markdown, and derived SQL indexes, as specified in `docs/ARCHITECTURE.md`. The preview's seeded source-note task links do not synchronize checkbox changes with Markdown.

The UI deliberately excludes tags, backlinks, graphs, collaboration, AI, calendar integrations, and analytics. Quick Note creates a normal note, per the MVP specification.
