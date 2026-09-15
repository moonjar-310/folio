# Verification results

Date: 2026-09-15. All data used for HTTP/browser verification was synthetic and isolated from the default personal vault.

## Automated checks

| Check | Result |
| --- | --- |
| `cargo fmt --all --check` | Passed |
| `cargo test --workspace --locked` | Passed: 15 tests across application, core and browser Markdown renderer; all workspace targets/doc tests passed |
| `cargo clippy --all-targets --locked -- -D warnings` | Passed for default native workspace members |
| Web and Worker `cargo check` for `wasm32-unknown-unknown` | Passed |
| Web and Worker WASM Clippy with `-D warnings` | Passed |
| `npm run app:build` | Release browser WASM and assets built successfully |
| Wrangler local D1 migrations | All five migrations applied |
| Native HTTP workflow | Passed against filesystem/SQLite at loopback port 8790 |
| Cloudflare HTTP workflow | Passed against local Wrangler D1/R2 at loopback port 8791 |
| Worker deployment dry run | Passed; WASM, static assets and bindings packaged without deployment |

The shared application tests exercise account setup/authentication, CSRF and origin rejection, login throttling, expired sessions, production setup gating, note/task/goal/folder operations, SQL pagination, dates, task identity after Markdown edits, stale revisions, note leases and preservation of Markdown after an index failure. Core tests cover validation, fenced-code task exclusion and title changes preserving the body/line endings. The renderer test covers unsafe HTML/links and table rendering.

The same `scripts/smoke-product.mjs` workflow passed on both storage implementations. It checks real HTTP status codes, cookies, session authorization, request validation, note read/save/search, source-checkbox synchronization, conflicts, folder operations, archive/restore/delete, standalone tasks, goals and logout.

## Penpot and browser review

The connected Folio Penpot file was inspected through MCP. Home, Todo, Planner, Notes and the shared 240 px sidebar were read and exported for visual review. The implementation uses the repository's Folio tokens, emblem and SVG icon assets.

Chrome review of the native Rust server confirmed:

- Account login and authenticated navigation.
- Korean Markdown editing and idle autosave, followed by successful readback.
- Markdown headings, paragraphs, tables, blockquotes and task-list preview.
- Completing a source task updates its original Markdown checkbox.
- Search returns a Korean body-text match and opens its source note.
- Ctrl+K opens the search dialog.
- Quick Note saves through the API, clears its input and appears in recently edited notes.
- Dark appearance and the selected note URL survive reload.
- Desktop navigation and mobile drawer navigation.
- Notes/editor and weekly planner stack at a 390 × 844 viewport.
- The browser viewport override was reset after mobile review.

HTTP fixtures do not ship as product seed data. Path storage rejects duplicate filenames; use fresh disposable stores for repeated smoke runs.

## What these results do not establish

- No production Cloudflare account resources were created, remote migrations run, or Worker deployed.
- Local Wrangler validates D1/R2 API behavior; it does not establish production CPU, billing, latency or concurrent-load characteristics.
- PBKDF2 password derivation executes in Worker WASM. Production setup/login CPU must be sized before deployment; do not assume Workers Free is sufficient.
- Browser review was performed in Chrome, not a complete cross-browser or assistive-technology certification.
- Canonical Markdown and SQL are separate stores. The implementation preserves the canonical write on an index failure and supports retry, but does not claim a transaction across R2 and D1. Back up both primary data stores.

See [deployment and recovery](DEPLOYMENT.md) and [architecture](ARCHITECTURE.md) for operational instructions.

## Folder explorer follow-up

The folder feature was designed in Penpot before implementation. Two editable boards now document nested expansion, shared ellipsis/right-click menus, and create/rename dialogs. Native and WASM Clippy passed; the workspace now has nine tests. Both native and local Cloudflare HTTP workflows also passed the new exact-folder listing and resumable hierarchy rename checks.

Browser review confirmed nested folder creation, right-click Markdown creation, parent-folder rename while preserving the edited Korean note, unchanged child relationships, collapsing all descendants, and Shift+F10 context menus. See [folder implementation](FOLDERS.md) for persistence and recovery behavior.

Final browser checks also confirmed a 390 × 844 mobile drawer/context menu and name dialog in Light/Dark appearance. After resetting the viewport, reload preserved a collapsed parent (`aria-expanded=false`) while keeping the selected Korean document open. The default local server was restarted on port 8788 and its health endpoint returned OK.

## Folder follow-up review

Additional checks passed after fixing cross-folder file selection and pending-rename draft recovery: all 10 workspace tests, Web/Worker WASM Clippy, and the release browser build. A browser test opened an indexed note in a pending rename, edited a Korean unsaved draft, resumed the rename, and saved the preserved draft as a separate note. The original canonical body was independently read through the API and remained unchanged; the pending operation was cleared.

## Path storage follow-up

The latest workspace has 15 passing tests (application 9, core 3, browser 3). Added tests cover canonical plain Markdown at real paths, lost note/folder/task/goal index rows, missing optional note identity metadata, arbitrary external Markdown files, empty directories, legacy migration and duplicate-title suffixes, filename case collisions, traversal/Windows path rejection, interrupted copy/delete moves, source edits during a pending move, malformed auxiliary records, and stale search-result cleanup.

Both `scripts/smoke-product.mjs` and `scripts/smoke-storage.mjs` passed against fresh native `.local/path-native` (8790) and local Wrangler `.local/path-worker` (8791). The storage workflow explicitly deleted all notes/folders/tasks/goals SQL rows, retained authentication, and then verified raw discovery/open/edit before rebuilding; search, Markdown checkboxes, standalone tasks and goals were restored. Folder rename and empty descendant preservation then passed. The resulting R2 object was downloaded with Wrangler and compared byte-for-byte with the native Markdown; they matched and contained no folder comment.

The default local store was backed up under `.local/backups/before-path-storage-20260915-172854`, migrated and rebuilt using the operator CLI. It contained no Markdown files before migration. The local server was restarted at 8788.

The latest Chrome login screen rendered, but password-manager interference / detached browser input prevented completion of a new authenticated Settings-button walkthrough. The earlier folder/editor browser results above remain historical; the new maintenance controls are build-checked and their authenticated endpoints are covered by the HTTP recovery workflows. No production Cloudflare deployment was performed.
