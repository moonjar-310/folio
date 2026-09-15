# Operating Folio

## Native local mode

```sh
npm ci
npm run app:build
npm run app:local
```

Open http://127.0.0.1:8788. First visit creates the single-user account. SQLite migrations apply automatically and transactionally.

| Variable | Default |
| --- | --- |
| FOLIO_DATA_DIR | .local/folio-app |
| FOLIO_ASSETS_DIR | dist/web |
| FOLIO_PORT | 8788 |

Use a separate data directory for verification. The built-in server is intentionally loopback-only. Keep your vault directory private to your OS account.

## Cloudflare local emulation

1. Build the frontend.
2. Create an ignored `.dev.vars` file containing `FOLIO_SETUP_TOKEN=<random value of at least 24 characters>`.
3. Run `npm run worker:migrate:local`, then `npm run worker:dev`.
4. Open the local Wrangler URL. On first setup, enter that token with your username/password.

Native and Wrangler modes use separate data stores. The adapter selects filesystem/SQLite for the native binary and R2/D1 for the Worker binary; neither runtime depends on the other being available.

## Cloud deployment

Deployment has not been performed by this implementation task. Supply resources in your own Cloudflare account:

1. Create a D1 database and an R2 bucket; replace the placeholder database ID and verify resource names in `wrangler.toml`.
2. Set the initial secret using `npx wrangler secret put FOLIO_SETUP_TOKEN`.
3. Apply migrations with `npx wrangler d1 migrations apply DB --remote`.
4. Build the browser assets with `npm run app:build`.
5. Validate packaging with `npm run worker:package`.
6. Configure a custom route/domain and deploy with `npx wrangler deploy`. `workers_dev=false` remains the default.
7. Complete first setup over HTTPS; verify login/logout, session cookies, note save/read, and task synchronization.
8. Remove the setup secret after first setup if desired; normal login does not require it.

These commands that create resources, run remote migrations or deploy are operator steps, not part of local verification.

### CPU and free-tier sizing

Authentication hashes passwords using PBKDF2-HMAC-SHA256 with 600,000 iterations. This protects stored credentials but is CPU-intensive. The current implementation executes that derivation inside Worker WASM. Do not assume setup/login fit Workers Free's 10 ms CPU allowance; test against the actual deployment, or configure sufficient CPU on Workers Paid. Do not weaken password hashing to fit a budget.

Worker Static Assets and query-based storage keep routine usage small, but no zero-cost guarantee is made. The Cloudflare emulator validates behavior, not production CPU billing or limits. [Cloudflare CPU limits](https://developers.cloudflare.com/workers/platform/limits/) and [CPU profiling](https://developers.cloudflare.com/workers/observability/dev-tools/cpu-usage/) describe the production checks.

## Backups and recovery

- Native: stop the local server before copying the entire data directory (SQLite plus vault), or use SQLite's backup API and snapshot the vault.
- Cloud: export D1 and back up the entire R2 `notes/` prefix, including `.folio/` and folder markers. D1 contains authentication and leases as well as derived indexes.
- Rebuild through Settings or the CLI rather than deleting SQL: account/session data is still SQL-owned. After whole-DB loss, recreate the schema and recover/reconfigure the account first.
- An index failure leaves Markdown intact. Keep the draft and retry Save.
- A conflict preserves the browser draft. Reopen the server version or choose **Save draft as a new note**.
- A process interrupted during a save may leave a note lease for up to two minutes; retry after expiration.
- Browser drafts are recovery aids, not backups. Clearing browser storage removes them.
- Older ID-based Markdown remains readable. Run **Migrate existing files** to convert it to paths and preserve original copies.

### Path storage upgrade

Back up the full data directory/bucket and database first. Apply migration `0005_vault_paths.sql` before running the new Worker. Native mode applies it automatically.

With the local server stopped:

```sh
cargo run -p folio-local --locked -- --migrate-storage
cargo run -p folio-local --locked -- --rebuild-index
npm run app:local
```

Both commands respect `FOLIO_DATA_DIR`. The equivalent Settings actions work in either runtime. Keep passing the returned state to the maintenance endpoints until `done=true`; each request is bounded and the sequence can be restarted. Skipped invalid records are reported, not silently discarded.

After migration, copy the complete vault (including `.folio/`) to retain standalone tasks/goals and stable identities. Copying only Markdown still preserves note text and folder paths. Losing `.folio/` does not prevent opening or editing Markdown, but cannot recover independent planner records that existed only there. Browser theme/expansion preferences remain browser-local.

Folio does not automatically synchronize local and R2 stores. External file changes are visible on refreshed file listings; use **Rebuild from files** for refreshed full-text search and checkbox lists. Avoid simultaneous direct filesystem/S3 edits during application-controlled moves: external tools do not honor Folio's leases.

No secrets, real cloud resource identifiers, or real user passwords belong in the repository.
