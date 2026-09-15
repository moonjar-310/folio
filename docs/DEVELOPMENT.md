# Development and verification

## Implemented on 2026-09-15

The Rust application now implements the product's Home, Todo, daily/weekly Planner, Notes/editor, Folders, Goals, Settings and authentication flows.

### UI

- Folio Penpot Home/Todo/Planner/Notes and 240 px navigation sidebar inspected through MCP.
- Reusable Sidebar, Icon, TaskRow, TaskList, TaskForm, GoalCard, NoteCard and section components.
- Warm semantic tokens, Newsreader/Geist/JetBrains Mono font stacks, quiet green actions and editorial typography.
- Light/Dark appearance, responsive sidebar/drawer and stacked mobile editor/planner.
- SQL search dialog, keyboard shortcuts, note preview, dirty state, autosave, explicit retry and draft recovery.
- Native form controls, labels, focus styles, live feedback and a skip link.

### Backend

- Shared application services and explicit environment adapter.
- Local implementation: filesystem/SQLite; Cloudflare implementation: R2/D1.
- Single-user setup/login/logout, expiring sessions, CSRF and login throttling.
- Note CRUD, rename via Markdown title, canonical folder/file paths, nested folders, archive, SQL search/pagination.
- Markdown task indexing with source edits, standalone tasks and due date/time validation.
- Goals create/edit/order/complete/archive.
- Revision conflicts, R2 conditional writes, per-note leases and canonical-data preservation after SQL failure.
- Five shared SQL migrations, applied automatically in native mode and through Wrangler in cloud mode.

## Verification commands

```sh
cargo fmt --all --check
cargo test --locked
cargo test -p folio-web --locked
cargo clippy --all-targets --locked -- -D warnings
cargo clippy -p folio-web -p folio-worker --target wasm32-unknown-unknown --locked -- -D warnings
npm run app:build
npm run worker:package
```

The application tests cover unauthorized requests, setup, CSRF/origin failures, persistence, source-checkbox changes, stale revisions, failure preservation, dates, task identity after insertion/reordering, note pagination, date filtering, concurrent-save leases, cloud setup gating, expired sessions and login throttling. The editor test covers raw HTML, unsafe links and table rendering.

`scripts/smoke-product.mjs` runs the same HTTP workflow against disposable native and Wrangler environments. It creates synthetic test data and checks sessions, folders, notes, search, tasks, goals, conflicts, archive/delete and logout.

Browser checks cover login, Korean Markdown editing, autosave, preview, source-task completion, search, theme persistence and mobile layout. Detailed final results are maintained in [verification](VERIFICATION.md).

## Operation boundary

A successful build/emulator test is not a live deployment. Real Cloudflare resource configuration, remote migration, production CPU sizing and deployment are described in [DEPLOYMENT.md](DEPLOYMENT.md). Password hashing may exceed the Free CPU allowance. No cloud resources are provisioned automatically.

The historical Node preview remains a separate design utility, not an alternate production backend.

The expandable folder/file tree, context menus and resumable folder renames are described in [FOLDERS.md](FOLDERS.md).

Path storage supports optional identities, journaled moves, legacy conversion and cache rebuilding. See [architecture](ARCHITECTURE.md) for the recovery boundary.
