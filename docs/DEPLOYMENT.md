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
| FOLIO_JWT_SECRET | Persisted `jwt-secret` in the data directory; optional override of at least 64 characters |

Use a separate data directory for verification. The built-in server is intentionally loopback-only. Keep your vault directory private to your OS account.

## Cloudflare local emulation

1. Install the toolchain listed in README and run `npm ci`. Wrangler builds both the frontend and Worker automatically.
2. Create an ignored `.dev.vars` file with **both** `FOLIO_SETUP_TOKEN` (at least 24 random characters) and `FOLIO_JWT_SECRET` (at least 64 characters; generate 32 random bytes as hex). The Worker requires the signing key even before account setup.
3. Run `npm run worker:migrate:local`, then `npm run worker:dev`.
4. Open the local Wrangler URL. On first setup, enter that token with your username/password.

For a new checkout without `.dev.vars`, this command generates both values without printing them. It refuses to overwrite an existing file; if the file exists, preserve its values and add only any missing setting. Read the setup token locally when the first-run form asks for it.

```sh
node -e "const fs = require('node:fs'); const crypto = require('node:crypto'); fs.writeFileSync('.dev.vars', 'FOLIO_SETUP_TOKEN=' + crypto.randomBytes(24).toString('hex') + '\nFOLIO_JWT_SECRET=' + crypto.randomBytes(32).toString('hex') + '\n', {flag: 'wx', mode: 0o600});"
```

Native and Wrangler modes use separate data stores. The adapter selects filesystem/SQLite for the native binary and R2/D1 for the Worker binary; neither runtime depends on the other being available.

## Cloud deployment with Access

운영 인증은 **Cloudflare Access 로그인 + Folio Access JWT(5분) / Refresh Token(1주일)**이다. 로컬 비밀번호는 Argon2id를 사용한다. 2026-09-16에 Worker, D1, R2와 Custom Domain 배포를 완료했다. Access는 허용 이메일 목록으로 전체 호스트를 보호한다. 계정별 실제 값은 Git에서 제외한 `.env.production`에 보관한다.

### 준비할 값과 설정

| 항목 | 필요한 내용 |
| --- | --- |
| CLI 인증 | `npx wrangler login`, `npx wrangler whoami` 성공 확인 |
| 계정/주소 | Account ID, 실제 서비스 호스트명, 같은 계정의 활성 Cloudflare DNS zone |
| D1 | `folio` DB와 UUID, `migrations/`의 7개 마이그레이션(0001–0007) 적용 |
| R2 | R2 구독 활성화와 비공개 `folio-notes` 버킷 |
| Access | Zero Trust 팀 도메인, Access application AUD, 허용할 이메일 목록 |
| 정책 | Folio 호스트 전체를 Access로 보호하고 지정한 이메일만 Allow |
| 로그인 | Access의 One-time PIN 로그인 방법 활성화 |

[Custom Domains](https://developers.cloudflare.com/workers/configuration/routing/custom-domains/)은 활성 zone이 필요하다. 호스트의 기존 DNS/서비스 충돌을 확인한다. [R2](https://developers.cloudflare.com/r2/get-started/)는 구독 활성화 과정에서 결제 설정을 요구할 수 있다. 버킷 public access나 별도 S3 키는 필요하지 않다.

### Access 설정

1. Zero Trust 조직을 설정하고 팀 도메인 `your-team.cloudflareaccess.com`을 확인한다.
2. **Integrations > Identity providers**에서 **One-time PIN**을 추가한다. 새 조직에는 자동으로 추가되지 않을 수 있다.
3. **Access > Applications**에서 Self-hosted 웹 앱을 추가하고 Folio의 실제 호스트 전체를 지정한다. `/api`나 로그인 경로만 보호하지 않는다.
4. Allow 정책의 Emails에 허용할 이메일을 지정한다. Everyone/Bypass 정책을 추가하지 않는다. 앱의 인증 방법에 One-time PIN을 선택한다.
5. 생성된 앱의 **Application Audience (AUD)** 값을 복사한다. Worker account ID나 API 토큰과 다른 값이다.

[OTP 안내](https://developers.cloudflare.com/cloudflare-one/integrations/identity-providers/one-time-pin/) · [Access 웹 앱](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/) · [JWT 검증](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/validating-json/)

### 계정 리소스 준비

```powershell
npx wrangler login
npx wrangler whoami
npx wrangler d1 list
npx wrangler r2 bucket list
# 해당 리소스가 없을 때만 생성. 여러 계정이면 먼저 CLOUDFLARE_ACCOUNT_ID 지정.
npx wrangler d1 create folio --location apac --update-config=false
npx wrangler r2 bucket create folio-notes
```

기존 동명 리소스가 다른 서비스 소유라면 공유 설정의 이름을 바꾼 후 진행한다. 기존 데이터는 자동 이전되지 않는다. 원격 마이그레이션 전에는 D1 및 R2를 백업한다.

### 운영 설정과 배포 없는 검사

```powershell
Copy-Item .env.production.example .env.production
# Account ID, D1 UUID, 호스트, Access 팀 도메인, AUD, 이메일을 입력
npm run worker:configure
npm run worker:preflight
```

`FOLIO_ACCESS_EMAIL`은 쉼표로 구분한 정확한 이메일 목록이다. 허용된 주소들은 같은 노트 공간을 공유하며 사용자별 저장소 분리는 없다. Access 정책에도 같은 목록을 지정한다.

생성된 `wrangler.production.toml`과 `.env.production`은 Git에서 제외된다. 생성기는 공통 `wrangler.toml`에서 빌드/스토리지 구성을 가져오고 인증 방식을 `cloudflare_access`로 고정한다. 입력값이 빠지면 생성 단계에서 실패한다. 유료 CPU 한도를 설정하지 않으며 비밀번호 fallback도 없다. 초기 `FOLIO_SETUP_TOKEN`이나 앱 비밀번호는 운영 Access 모드에서 필요하지 않다.

`worker:preflight`는 **dry-run**만 수행하며 권한/실제 리소스/정책/운영 CPU까지 확인하지 않는다. `worker:package`는 운영 값이 없는 상태에서 두 release 번들을 빌드하는 로컬 검사다. Rust 1.91+, wasm32 타깃, Trunk 0.21.14+, Node.js 22.9+ 및 `npm ci`가 필요하다. worker-build 0.8.5는 필요할 때 설치한다.

### 실제 게시

```powershell
npm run worker:migrate:remote
# 최초 수동 배포 또는 서명키 교체 시: 생성된 운영 설정으로 시크릿 등록
npx wrangler secret put FOLIO_JWT_SECRET --config wrangler.production.toml
npm run worker:deploy
```

Access 정책을 먼저 구성한 뒤 게시한다. 운영 배포에는 반드시 `worker:deploy`를 사용한다. 기본 `wrangler.toml`은 비밀번호 기반 로컬 에뮬레이션용이다. `workers.dev`와 preview URL은 비활성화되어 있다. API는 Access assertion이 없거나 검증에 실패하면 거부한다. 운영 URL에서 이메일 PIN으로 로그인하면 Folio 토큰 쌍을 발급한다. 일반 요청은 Access JWT만 검증하고, 갱신할 때 DB의 Refresh Token 해시를 검증·교체한다. 비밀번호 설정 화면은 나타나지 않는다.

게시 후 본인/다른 이메일 허용·거부, 로그인/로그아웃, Secure 쿠키, 노트 저장·재조회, Todo 동기화, Access 만료 후 재로그인을 확인한다. `/api/auth/logout`은 Folio 세션을 삭제하고 브라우저는 `/cdn-cgi/access/logout`으로 이동한다. 외부 IdP 세션까지 종료된다는 보장은 없다. 세션 만료 중 미저장 초안이 있으면 재로그인 버튼이 저장소 백업을 확인한 뒤 페이지를 다시 연다. 브라우저 저장이 실패하면 이동을 막으므로 먼저 글을 복사한다.

### 테스트와 CPU

```sh
npm run worker:config:test
npm run worker:smoke
cargo test --workspace --locked
```

`worker:smoke`는 각각 새 `.local/worker-smoke-*` D1/R2에서 Argon2id 비밀번호 모드와 Access 모드를 검사한다. Access 테스트는 매 실행 생성한 RSA 키/서명 토큰을 전용 임시 entrypoint에 주입한다. 실제 JWT 검증과 Rust 업무 API는 실행하지만 원격 Access 로그인 페이지·이메일 전달·정책 집행을 대신 검증하지는 않는다. 생산 entrypoint는 팀 도메인의 JWKS만 사용한다. 제품 흐름 확인 후 같은 임시 저장소에 복구 샘플을 만들고, 로컬 D1의 notes/folders/tasks/goals 인덱스만 삭제한 뒤 원본 파일 탐색·편집·재색인·폴더 이동을 검증한다. R2 원본과 인증 테이블은 유지한다. 이 삭제는 자동 생성한 임시 로컬 저장소에서만 수행하며 원격 DB에는 실행하지 않는다. 테스트 서버는 종료하며 디버깅용 파일은 남긴다.

일반 비밀번호는 Argon2id v19, 메모리 19 MiB, 반복 2회, 병렬도 1이다. 기존 PBKDF2-SHA256 600,000회 해시는 올바른 비밀번호로 로그인한 경우에만 Argon2id로 전환하며 실패한 로그인에서는 변경하지 않는다. 로컬 데이터에 자동 선행 변환은 수행하지 않는다.

운영 Access 모드는 비밀번호 해싱을 실행하지 않지만 JWT 서명 검증과 D1/R2 업무 처리는 CPU를 사용한다. 무료 요금제 동작은 배포 후 실제 CPU 지표로 확인해야 한다. 유료 요금제를 필수로 가정하지 않는다. [CPU 한도](https://developers.cloudflare.com/workers/platform/limits/)와 [Access 세션](https://developers.cloudflare.com/cloudflare-one/access-controls/access-settings/session-management/)을 참고한다.

## Backups and recovery

- Native: stop the local server before copying the entire data directory (SQLite plus vault), or use SQLite's backup API and snapshot the vault.
- Cloud: export D1 and back up the entire R2 `notes/` prefix, including `.folio/` and folder markers. D1 contains authentication and leases as well as derived indexes.
- Rebuild through Settings or the CLI rather than deleting SQL: account/session data is still SQL-owned. After whole-DB loss, recreate the schema; in password mode also recover/reconfigure the account. Access mode can issue a new session after verified owner sign-in.
- An index failure leaves Markdown intact. Keep the draft and retry Save.
- A conflict preserves the browser draft. Reopen the server version or choose **Save draft as a new note**.
- A process interrupted during a save may leave a note lease for up to two minutes; retry after expiration.
- Browser drafts are recovery aids, not backups. Clearing browser storage removes them.
- Older ID-based Markdown remains readable. Run **Migrate existing files** to convert it to paths and preserve original copies.

### Path storage upgrade

Back up the full data directory/bucket and database first. Apply all pending migrations before running the current Worker, including `0005_vault_paths.sql` for path storage and `0007_refresh_tokens.sql` for current authentication. Native mode applies them automatically.

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


## GitHub Actions deployment

`.github/workflows/cloudflare.yml` checks pull requests and pushes to `main`. Main pushes and manual runs on main deploy only after Rust checks/tests, JWT/config tests and both disposable Worker workflows pass. Deployments are serialized. Actions are pinned to immutable commits; PR checks do not receive production secrets.

Repository secrets:

- `FOLIO_PRODUCTION_ENV`: contents of the ignored `.env.production` file. Includes account, database, domain, Access settings and optional placement hint.
- `CLOUDFLARE_API_TOKEN`: dedicated deployment API token; do not copy Wrangler's temporary OAuth or refresh token into CI.
- `FOLIO_JWT_SECRET`: token signing key, at least 64 characters; see [signing-key setup](#folio-token-signing-key).

The deployment step creates its ignored config, validates a production dry run, applies pending D1 migrations, installs the JWT signing secret and deploys. It removes generated configuration on exit. It never changes Access policies or creates a public R2 bucket. Account permission scope must be reviewed when issuing the deployment token.

`FOLIO_PLACEMENT_REGION=aws:ap-east-1` places execution near Hong Kong for the currently observed HKG D1 primary. Omit the optional value to use Smart Placement. Review measurements before changing the hint when moving storage. Browser static assets remain served at the edge.


### Folio token signing key

`FOLIO_JWT_SECRET` is required as an encrypted GitHub repository secret. Generate at least 32 random bytes encoded as 64 hex characters. The deployment workflow installs it as the Worker secret before deployment. Never add it to TOML vars or committed env files. For a manual deployment, configure the same secret with `npx wrangler secret put FOLIO_JWT_SECRET --config wrangler.production.toml` before publishing. Local Worker emulation also needs a development signing key in `.dev.vars`; disposable smoke tests supply their own key automatically.

Migration 0007 clears obsolete opaque sessions/audit rows and adds the refresh username field. Existing users authenticate again after this migration; notes, tasks and account passwords are unaffected. Refresh expiry is an absolute week, not a sliding extension; business JWTs last five minutes. The Cloudflare Access session policy is independent. See [auth details](AUTH_OPTIONS.md).
