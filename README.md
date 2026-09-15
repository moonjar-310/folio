# Folio

A personal planner and Markdown notebook, built with **Rust, Leptos CSR and a shared JSON API**. The Folio Penpot screens and sidebar guide the interface.

## Included

- Home: today's date, goals, intentions, Quick Note, recent notes and Daily Note.
- Todo: Today (including overdue), Upcoming, Someday and Completed; create, edit, schedule, complete and delete standalone tasks.
- Planner: daily/weekly views, date navigation and task actions.
- Notes: create, rename, move, archive, delete, paginated SQL search and path-based file lists.
- Editor: Markdown preview, three-second autosave, save on blur/navigation and Ctrl/Cmd+S, browser draft recovery, visible failures and revision conflicts.
- Nested Folders, editable/reorderable goals, Settings, persistent Light/Dark themes and mobile navigation.
- Single-user setup/login/logout, hashed passwords, expiring server sessions, CSRF checks and rate-limited login.

## Runtime composition

| Crate | Responsibility |
| --- | --- |
| `folio-core` | Contracts, validation, Markdown parsing and storage port |
| `folio-application` | Shared authentication, API routing, notes, tasks and goals |
| `folio-adapter` | Selects an implementation using the entrypoint's Cargo feature |
| `folio-storage-local` | Native filesystem and SQLite |
| `folio-storage-cloudflare` | Cloudflare R2 and D1 |
| `folio-local` | Axum loopback HTTP server; adapter feature `local` |
| `folio-worker` | Workers fetch entrypoint; adapter feature `cloudflare` |
| `folio-web` | Leptos browser WASM, components and client state |

Runtime selection is compiled into each entrypoint. Browser code and the shared application do not import Cloudflare types.

## Run locally

Requires Rust 1.91+, the `wasm32-unknown-unknown` target, Trunk 0.21.14+, and Node.js 22+.

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk --version 0.21.14
npm ci
npm run app:build
npm run app:local
```

Open **http://127.0.0.1:8788** and create your workspace. Use at least 12 characters for the password. The server binds loopback only and automatically migrates SQLite.

Data defaults to `.local/folio-app/`. Set `FOLIO_DATA_DIR`, `FOLIO_ASSETS_DIR`, or `FOLIO_PORT` to override. For live frontend development, leave the API running and run `npm run app:dev` (Trunk on port 8080).

## Cloudflare

See [deployment instructions](docs/DEPLOYMENT.md) for D1/R2 resources, migrations, the initial setup secret, and CPU requirements for password hashing.

```sh
# Configure .dev.vars as described in docs/DEPLOYMENT.md first.
npm run worker:migrate:local
npm run worker:dev
```

The browser application is served through Static Assets; only `/api` and `/api/*` execute the Worker. Authentication is required in both runtimes. There is no development authentication bypass.

## Verify

```sh
cargo fmt --all --check
cargo test --locked
cargo test -p folio-web --locked
cargo clippy --all-targets --locked -- -D warnings
npm run worker:check
npm run app:check
npm run app:build
npm run worker:package
```

With a **disposable loopback data directory**, run:

```sh
node scripts/smoke-product.mjs http://127.0.0.1:8788
```

This creates synthetic fixtures and a test account if the store is empty. It checks authentication, CSRF, origin checks, notes, search, folders, task/Markdown synchronization, conflicts, goals, archive/delete and logout. Do not run it against your personal vault.

Implementation and verification details are in [development status](docs/DEVELOPMENT.md). Live Cloudflare deployment and production CPU/free-tier behavior are not asserted by emulator tests.

## Data and recovery

Markdown is canonical at its folder path: `vault/Projects/Folio/Design.md` locally and `notes/Projects/Folio/Design.md` in R2. The body is plain Markdown. Folders and files remain discoverable without note indexes or optional identity metadata.

`.folio/` stores standalone tasks/goals and auxiliary identity/recovery records. SQL caches search and planner data; credentials, sessions and mutation leases remain in SQL. Settings provides **Rebuild from files** and **Migrate existing files**. Migration retains verified original copies under `.folio/legacy/`. See [architecture](docs/ARCHITECTURE.md) and [operation/recovery](docs/DEPLOYMENT.md).


## Design references

[Product specification](docs/PRODUCT_SPEC.md) · [Design](docs/DESIGN.md) · [Folio implementation contract](design/folio/README.md) · [UI handoff](docs/UI_HANDOFF.md)

The earlier Node preview remains available using `npm run preview:dev`. Its data and behavior are separate from the Rust application and it is not the production backend.
