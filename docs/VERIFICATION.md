# Verification results

## Home intentions date filtering — 2026-09-18

- Home now shows only tasks due on the current local day; Todo retains overdue tasks.
- The application regression test verifies workspace bootstrap with more than 200 past tasks, today's paginated results (including completed tasks), exclusion of future tasks, preserved Todo overdue results, and rejection of an empty current date.
- Playwright with synthetic September 16–19 fixtures verified Home after reload, Todo and return navigation, and a simulated September 18–19 rollover. Only each day's intentions appeared on Home; September 16–17 remained available in Todo.
- Application tests (11), release browser build, formatting and diff checks passed.

## Local date rollover — 2026-09-18

- Reproduced the previous stale Home date by advancing Playwright's clock from September 17, 23:59 to September 18, 00:01 in Asia/Seoul and returning focus to the tab.
- After the fix, Playwright verified Home date/task refresh, Todo visibility-resume refresh, updated default due dates, preserved Quick Note/task drafts and manually selected dates, planner Today highlighting, Sunday-to-Monday navigation, timer-only midnight rollover, and the new day's Daily Note title.
- Repeated focus/visibility/pageshow on the same date made zero task-list requests. Tests used only an isolated native store with synthetic tasks.
- Web unit tests (5), WASM compilation, release frontend build, formatting and WASM Clippy passed.

## Privacy remediation — 2026-09-17

- Production deploy registers masks for individual configuration values and allowed emails before calling Wrangler, and suppresses informational binding summaries. Twelve Node authentication/configuration/masking tests passed; actionlint accepted the workflow.
- Explicit sign-out offers Save / Discard / Cancel for Quick Note drafts. Playwright checks on an isolated synthetic native store passed cancellation, saving and readback after login, discard cleanup, logout API failure recovery, browser-storage cleanup failure, and retry without duplicate notes. The 390 px dialog was visually checked.
- Web unit tests (5), formatting, WASM compilation, release frontend build and WASM Clippy passed. Default personal data and remote application data were not used for these checks.
- Previously exposed public deployment logs were deleted and their download endpoints confirmed unavailable. Git history cleanup is handled separately from ordinary source edits; copies already downloaded by third parties cannot be revoked.

## Current review — 2026-09-17

- `cargo test --workspace --locked`: 19 tests passed (application 11, core 3, browser 5), plus workspace targets and doc tests.
- `npm run worker:config:test`: all 9 authentication/configuration tests passed.
- All 15 Markdown documents were read against source/configuration; 66 local links, image references and heading anchors resolved, and every documented npm script exists. The 18 public external documentation links opened successfully; the private design URL was replaced with its board location.
- The documented `.dev.vars` generation command passed in an isolated temporary directory: expected key lengths, no secret output and refusal to overwrite an existing file.
- The repository contains 7 migrations (0001–0007). Current auth uses Argon2id locally and Cloudflare Access in production, with Folio JWT/refresh tokens in both runtimes.
- README screenshots were captured with Playwright on 2026-09-17 using a separate native demo store and synthetic data, after a successful release frontend build. These images cover Home, Planner, note preview, dark appearance and mobile Todo.
- Follow-up: the complete `npm run worker:smoke` passed for password and signed Access modes, including canonical recovery after deleting only derived D1 rows. All 7 migrations applied to each fresh local store. Native filesystem/SQLite recovery passed separately after stopping the test server, clearing derived rows and restarting it.
- GitHub Actions run [35196579296](https://github.com/moonjar-310/folio/actions/runs/35196579296) for `a65c5a9` completed successfully, including `Deploy main`. This confirms that deployment job; live email login, expiry/logout and sustained production CPU were not re-tested.

The sections below are dated implementation checkpoints, not one current test report. Their counts, auth models, Paid-plan assumptions and deployment status apply only to that checkpoint. Follow [current operation instructions](DEPLOYMENT.md) and [current authentication](AUTH_OPTIONS.md) when running the app.

`scripts/smoke-storage.mjs` now uses Bearer JWTs, same-origin requests, refresh rotation and optional signed Access assertions. `scripts/smoke-worker.mjs` runs its prepare/verify-loss phases against the newly created local D1/R2 store in both modes. Between phases the harness clears only notes/folders/tasks/goals SQL rows; canonical files and authentication remain intact. The helper first checks that search/task/goal indexes are empty, then verifies file discovery/open/edit, rebuilding, search, source checkboxes, standalone tasks/goals, and folder moves with empty descendants. Invalid phases are rejected before any HTTP calls.

## Initial verification — 2026-09-15

All data used for HTTP/browser verification below was synthetic and isolated from the default personal vault.

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

At this path-storage checkpoint, the workspace had 15 passing tests (application 9, core 3, browser 3). Added tests cover canonical plain Markdown at real paths, lost note/folder/task/goal index rows, missing optional note identity metadata, arbitrary external Markdown files, empty directories, legacy migration and duplicate-title suffixes, filename case collisions, traversal/Windows path rejection, interrupted copy/delete moves, source edits during a pending move, malformed auxiliary records, and stale search-result cleanup.

Both `scripts/smoke-product.mjs` and `scripts/smoke-storage.mjs` passed against fresh native `.local/path-native` (8790) and local Wrangler `.local/path-worker` (8791). The storage workflow explicitly deleted all notes/folders/tasks/goals SQL rows, retained authentication, and then verified raw discovery/open/edit before rebuilding; search, Markdown checkboxes, standalone tasks and goals were restored. Folder rename and empty descendant preservation then passed. The resulting R2 object was downloaded with Wrangler and compared byte-for-byte with the native Markdown; they matched and contained no folder comment.

The default local store was backed up under `.local/backups/before-path-storage-20260915-172854`, migrated and rebuilt using the operator CLI. It contained no Markdown files before migration. The local server was restarted at 8788.

The latest Chrome login screen rendered, but password-manager interference / detached browser input prevented completion of a new authenticated Settings-button walkthrough. The earlier folder/editor browser results above remain historical; the new maintenance controls are build-checked and their authenticated endpoints are covered by the HTTP recovery workflows. No production Cloudflare deployment was performed.

## Cloudflare pre-deployment preparation — 2026-09-16

- Wrangler now builds both the Leptos frontend and Worker through `scripts/build-cloudflare.mjs`, using locked release builds and worker-build 0.8.5. A separate frontend build is no longer required before Wrangler packaging.
- `npm run worker:package` passed: 4 static assets, Worker upload 777.18 KiB / gzip 294.89 KiB, with DB, NOTES and ASSETS bindings. This was a dry run, not an upload.
- `node scripts/smoke-worker.mjs` passed against a fresh `.local/worker-smoke-azOx5q` store after packaging. All 5 local D1 migrations applied. Static HTML, actual browser WASM bytes/MIME, SPA deep links, API navigation bypass, no-store headers and rejection of an incorrect setup secret passed. The full authenticated D1/R2 product HTTP workflow also passed. The test runtime stopped afterward.
- `npm run check` passed (Rust formatting and browser/Worker WASM checks).
- `cargo test --workspace --locked` passed all 15 tests and doc tests.
- `npm run worker:config:test` passed 4 tests, including actual Wrangler parsing of generated production TOML, invalid account/database/hostname rejection, shared bindings and CPU/custom-domain settings.
- `npm run worker:configure` without production inputs failed as expected before producing an operational configuration.
- `git diff --check` passed. No new authenticated browser interaction review was performed in this preparation.

Production configuration is generated from the shared config and three ignored environment values. Worker preview URLs and workers.dev are disabled; production adds Custom Domain routing and a 30,000 ms CPU limit requiring Workers Paid. `worker:preflight` packages without deployment; `worker:migrate:remote` and `worker:deploy` are explicit operator actions.

`npx wrangler whoami` reported an expired token that could not be refreshed non-interactively. Therefore actual account permissions, plan, D1/R2 availability, active DNS zone and hostname ownership/conflicts remain unverified. No cloud resources, remote schema, secrets or DNS were changed, and no Worker was deployed. The production-specific preflight awaits actual Account ID, D1 UUID and hostname; its generated configuration format was checked using synthetic test values. See [deployment requirements and commands](DEPLOYMENT.md).


## Access and Argon2id implementation — 2026-09-16

This supersedes the earlier PBKDF2/Paid deployment assumptions above. Production configuration now requires Cloudflare Access and has no Paid CPU override. Local password authentication uses Argon2id; legacy PBKDF2 verification exists only for successful-login migration.

- Rust workspace tests: 17 passed (application 11, core 3, browser 3), plus doc tests. New tests verify encoded Argon2id parameters, unsuccessful legacy login leaving credentials unchanged, successful migration, Access identity/session binding, expiry, Origin/CSRF and logout.
- `npm run worker:config:test`: 7 tests passed. Real RSA/JWT verification rejects wrong signatures, HS256, issuer, AUD, expiry, not-before, user and token type. Tests cover private identity handoff, generated WorkerEntrypoint class invocation, fail-closed configuration/outage behavior, remote JWKS cache and rotation, and production configuration parsing.
- Native and browser/Worker WASM Clippy with `-D warnings` passed. Rust formatting and release browser/Worker compilation passed.
- Fresh local Workers/D1/R2 product workflows passed for password mode (`.local/worker-smoke-yBbIQ0`) and signed Access mode (`.local/worker-smoke-qSHRGB`). Each applied 6 migrations and ran static HTML/WASM, SPA/API routing, authentication, notes, search, folders, source-checkbox synchronization, conflicts, goals, tasks and logout. Access mode also rejected missing and forged assertion headers. Both test runtimes stopped.
- Browser verification used a disposable native store `.local/auth-ui-20260916` at port 8794. Actual native Argon2id setup, logout and login passed. Access UI behavior was exercised with Playwright response interception over that store: one automatic session exchange, no password fields, HTML returned to a save request preserving the editor, browser-storage failure blocking navigation, exact Korean draft restored after reauthentication/reload, and full-page Access logout navigation. The native beforeunload confirmation was accepted only after the recovery backup check. Test routes were removed afterward.
- The final Worker wrapper retains the generated Rust WorkerEntrypoint class and its panic-recovery proxy; it does not call a nonexistent static fetch method.

No actual Access organization/policy, cloud resources, remote migration or deployment was changed. The test-only RSA public key is injected through an ignored temporary entrypoint, not through a production bypass. Email OTP delivery, actual Access redirects/cookies/policy revocation and production CPU remain live-deployment checks. Existing personal local credentials were not modified by these tests.


## 2026-09-16 production deployment

- Created dedicated Folio D1 (APAC) and private R2 bucket; applied all six migrations remotely.
- Created self-hosted Access application covering the entire custom hostname, with an Allow policy for the two user-specified exact emails and One-time PIN authentication. Read back the saved policy in the dashboard.
- Extended Worker email verification to a normalized comma-separated allowlist. Eight Node tests pass, including signed JWTs for either allowed identity, outsider rejection and malformed-list rejection.
- Production dry run and actual Worker deployment succeeded: 868.64 KiB upload, 326.08 KiB gzip, reported startup 7 ms. D1, R2 and assets bindings present. workers.dev and previews disabled.
- HTTPS browser navigation reached the Folio Access sign-in form. Anonymous root, auth status and notes requests returned 302 to the configured Access team with a browser User-Agent. Python's default User-Agent was rejected by Cloudflare with 403/1010 before that redirect.
- Owner completed real email PIN login. Folio displayed the authenticated email and created a cloudflare_access session (confirmed in remote D1). Saved a Korean deployment record from the live UI; page reload retained its title and content, and remote D1 confirmed its Personal Markdown path. Local emulator verification is recorded separately above. Second-email login, live logout/expiry flows and sustained CPU usage were not exercised.


## 2026-09-16 JWT authorization and latency

- Before: live tail measured workspace 1,318 ms, tasks 384/390 ms, folder-rename status 511 ms. Request ingress was SJC while remote D1 reported HKG.
- Removed D1 session reads from production authorization. Access JWT validation remains per request; D1 stores only a token fingerprint audit asynchronously at auth startup. Local Argon2id/password sessions are unchanged.
- Reused the workspace key listing; SQL lease acquisition uses atomic RETURNING. Added request-scoped aggregate Server-Timing and content-free diagnostic timing logs.
- Smart Placement did not immediately relocate this small workload. Applied explicit production hint aws:ap-east-1 based on the observed HKG D1 primary.
- Warm live tail after placement: workspace 226 ms (application 197 ms, four D1 queries totaling 35 ms, R2 list 84 ms, R2 format-marker read 78 ms); tasks 12/20 ms (D1 8/12 ms); folder-rename status 97 ms. These are small-sample Worker processing measurements, not browser end-to-end latency. First calls can be slower because of remote key discovery and storage cold paths.
- Warm auth-status Server-Timing reported no application storage wait; tail wall-time also includes background audit completion and must not be treated as response time for that route.
- Rust workspace 17 tests pass; native and WASM Clippy pass; 10 Node authentication/config tests pass. Both password and signed-JWT disposable Worker product smoke flows pass. actionlint 1.7.12 accepted the GitHub Actions workflow.
- GitHub Actions workflow checks PRs and main pushes, then performs production preflight, remote migration and deploy on main. Encrypted FOLIO_PRODUCTION_ENV repository secret configured; the deployment API token was created with the approved scope and stored as `CLOUDFLARE_API_TOKEN` in GitHub Actions.


## 2026-09-16 Folio token-pair update

The direct-Access/audit-only implementation and timings above describe an earlier deployment. The current change uses a Folio Access JWT (300 seconds) and hash-only, atomically rotated Refresh Token (604800 seconds, absolute expiry), shared by native and Worker runtimes. The signing key is stored in encrypted GitHub Actions secrets. Rust workspace tests (17), native/WASM Clippy, nine Node authentication/config tests and both disposable Worker product flows passed. Refresh smoke checks confirm rotation and old-token rejection. GitHub Actions workflow passes actionlint. Live deployment is verified separately.
