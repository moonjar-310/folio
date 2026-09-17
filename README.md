# Folio

**하루의 계획과 생각을 한곳에 모으는 개인용 플래너 · Markdown 노트 앱**

Folio는 노트, 할 일, 일간·주간 계획, 목표를 하나의 작업 공간에서 관리하는 웹 애플리케이션입니다. Rust와 Leptos 기반으로 동작하며, 내 컴퓨터에서는 파일과 SQLite에, Cloudflare에서는 R2와 D1에 데이터를 저장합니다. 노트 본문은 일반 Markdown 파일로 보관합니다.

## 주요 기능

| 화면 / 기능 | 설명 |
| --- | --- |
| Home | 오늘의 날짜, 목표와 의도, Quick Note, 최근 노트, Daily Note |
| Todo | 오늘과 기한이 지난 할 일, 예정된 일, 언젠가 할 일, 완료 항목 관리 |
| Planner | 일간·주간 보기, 날짜 이동, 할 일 일정 및 완료 상태 변경 |
| Notes | 노트 생성·이름 변경·이동·보관·삭제, 검색 및 페이지 단위 조회 |
| Markdown 편집기 | 미리보기, 3초 자동 저장, 포커스 이동·화면 이동 시 저장, Ctrl/Cmd+S, 브라우저 초안 복구 |
| 노트와 할 일 연동 | Markdown 체크박스를 할 일로 표시하고 완료 상태를 원문에 반영 |
| 폴더와 목표 | 중첩 폴더 트리, 폴더 이름 변경, 목표 편집·순서 변경·완료·보관 |
| 사용 환경 | 라이트·다크 테마, 모바일 내비게이션, 저장 실패 및 수정 충돌 안내 |

개인 작업 공간을 전제로 합니다. Cloudflare Access에 여러 이메일을 허용해도 같은 공간을 공유하며, 사용자별 저장소 분리는 제공하지 않습니다. 로컬 저장소와 Cloudflare 저장소 사이의 자동 동기화는 없습니다.

## 기술 구성

| 영역 | 기술 |
| --- | --- |
| 프런트엔드 | Rust, Leptos CSR, WebAssembly, Trunk |
| 공통 백엔드 | Rust 애플리케이션 서비스와 JSON API |
| 로컬 실행 | Axum HTTP 서버, 파일시스템, SQLite |
| 클라우드 실행 | Cloudflare Workers, Static Assets, R2, D1 |
| 인증 | 로컬 Argon2id 비밀번호 / 운영 Cloudflare Access 이메일 로그인 |
| 개발 도구 | Cargo, Node.js, npm, Wrangler, GitHub Actions |

로컬 서버와 Worker는 같은 애플리케이션 로직을 사용하고, 각 진입점의 Cargo feature로 저장소 구현을 선택합니다. 브라우저와 공통 애플리케이션 계층은 Cloudflare 타입에 의존하지 않습니다.

## 로컬에서 시작하기

### 준비 사항

- Rust 1.91 이상과 Cargo — CI에서는 Rust 1.91.1 사용
- Rust `wasm32-unknown-unknown` 타깃
- Trunk 0.21.14
- Node.js 22.9 이상과 npm — CI에서는 Node.js 24 사용

```sh
git clone https://github.com/neongseoman/folio.git
cd folio
rustup target add wasm32-unknown-unknown
cargo install --locked trunk --version 0.21.14
npm ci
npm run app:build
npm run app:local
```

브라우저에서 **http://127.0.0.1:8788**을 열고 첫 계정을 생성합니다. 비밀번호는 12자 이상이어야 합니다. 로컬 서버는 루프백 주소에만 바인딩하며 SQLite 마이그레이션을 자동 적용합니다.

### 프런트엔드 개발

`npm run app:local`로 API 서버를 실행한 상태에서 별도 터미널을 엽니다.

```sh
npm run app:dev
```

**http://127.0.0.1:8080**에서 개발 화면을 확인합니다. Trunk가 프런트엔드 변경을 반영하고 `/api/` 요청을 기본 로컬 서버 포트 `8788`로 전달합니다. API 포트를 변경하면 [Trunk 설정](crates/web/Trunk.toml)의 프록시 주소도 맞춰야 합니다.

### 로컬 환경 변수

| 변수 | 기본값 | 용도 |
| --- | --- | --- |
| `FOLIO_DATA_DIR` | `.local/folio-app` | 데이터 저장 디렉터리 |
| `FOLIO_ASSETS_DIR` | `dist/web` | 빌드한 프런트엔드 경로 |
| `FOLIO_PORT` | `8788` | 로컬 API 및 정적 파일 서버 포트 |

## Cloudflare 실행과 배포

### 로컬 에뮬레이션

루트에 `.dev.vars`를 만들고 24자 이상의 임의 값으로 `FOLIO_SETUP_TOKEN`을 설정합니다. 첫 계정 생성 시 이 토큰을 입력합니다.

```sh
npm run worker:migrate:local
npm run worker:dev
```

Wrangler가 브라우저와 Worker 번들을 빌드합니다. `worker-build` 0.8.5도 필요 시 빌드 스크립트가 설치합니다. 터미널에 표시된 로컬 URL로 접속하면 됩니다. 에뮬레이터 데이터는 네이티브 로컬 앱 데이터와 별도로 보관됩니다.

### 운영 환경

운영은 Cloudflare Access로 호스트 전체를 보호하고 허용된 이메일로 로그인합니다. Folio는 로그인 후 5분 Access JWT와 1주일 회전형 Refresh Token을 사용합니다. 두 실행 환경 모두 인증이 필요하며 개발용 인증 우회는 없습니다.

1. D1, 비공개 R2 버킷, Custom Domain, Cloudflare Access 정책을 준비합니다.
2. `.env.production.example`을 `.env.production`으로 복사하고 계정·DB·호스트·Access 설정을 입력합니다.
3. `npm run worker:preflight`로 운영 설정 생성과 배포 dry-run을 수행합니다.
4. [운영 가이드](docs/DEPLOYMENT.md)에 따라 JWT 시크릿을 설정하고 원격 마이그레이션 및 배포를 진행합니다.

`.env.production`과 생성된 `wrangler.production.toml`은 Git에서 제외됩니다. `worker:preflight`는 실제 배포나 운영 권한·정책 검증을 수행하지 않습니다.

[GitHub Actions](.github/workflows/cloudflare.yml)는 PR과 `main` push를 검증합니다. **`main` push 또는 `main`에서 수동 실행하면 검사 통과 후 Cloudflare 배포를 수행합니다.** 필요한 저장소 시크릿과 운영 절차는 [배포 문서](docs/DEPLOYMENT.md#github-actions-deployment)를 참고하세요.

## 프로젝트 구조

```text
crates/
  core/                공통 계약, 검증, Markdown 파싱, 저장소 인터페이스
  application/         인증, API 라우팅, 노트·할 일·목표 서비스
  adapter/             실행 환경별 저장소 구현 선택
  storage-local/       파일시스템 및 SQLite
  storage-cloudflare/  R2 및 D1
  local/               Axum 로컬 서버 진입점
  worker/              Cloudflare Worker 진입점
  web/                 Leptos 화면, 컴포넌트, 클라이언트 상태
migrations/            SQL 마이그레이션
scripts/               빌드, 배포 설정, 스모크 테스트 도구
tests/                 Node 기반 테스트
docs/                  제품, 아키텍처, 운영 및 검증 문서
design/folio/          디자인 구현 기준
```

`preview/`, `ui/`, `public/`에는 초기 Node 기반 디자인 프리뷰가 남아 있습니다. `npm run preview:dev`로 실행하며 Rust 앱과 데이터 및 동작이 분리된 디자인 확인 도구입니다.

## 데이터 보관과 복구

노트 본문은 폴더 경로를 유지하는 Markdown 파일이 원본입니다.

- 로컬: 데이터 디렉터리 아래 `vault/Projects/Folio/Design.md`
- Cloudflare R2: `notes/Projects/Folio/Design.md`
- `.folio/`: 독립 할 일·목표, 식별 정보와 복구 보조 기록
- SQLite / D1: 검색·플래너용 인덱스와 인증·세션·수정 잠금 정보

Settings의 **Rebuild from files**로 파일에서 인덱스를 재구성하고, **Migrate existing files**로 기존 저장 형식을 경로 기반으로 변환할 수 있습니다. 변환 시 검증된 원본 사본을 `.folio/legacy/`에 보관합니다.

전체 복구를 위해 로컬에서는 서버를 중지하고 데이터 디렉터리 전체를 백업하고, 클라우드에서는 D1과 `.folio/`를 포함한 R2 `notes/` 전체를 백업합니다. Markdown만 복사하면 노트 본문은 보존되지만 독립 할 일·목표와 인증 데이터는 포함되지 않습니다. 외부에서 파일을 편집한 뒤 검색과 체크박스 목록을 갱신하려면 인덱스를 재구성합니다.

자세한 내용은 [아키텍처](docs/ARCHITECTURE.md)와 [백업 및 복구](docs/DEPLOYMENT.md#backups-and-recovery)를 참고하세요.

## 개발 및 검증 명령어

```sh
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --all-targets --locked -- -D warnings
cargo clippy -p folio-web -p folio-worker --target wasm32-unknown-unknown --locked -- -D warnings
npm run app:build
npm run worker:config:test
npm run worker:smoke
```

| 명령어 | 용도 |
| --- | --- |
| `npm run app:check` | 웹 WASM 컴파일 검사 |
| `npm run worker:check` | Worker WASM 컴파일 검사 |
| `npm run worker:package` | 프런트엔드·Worker 빌드 및 배포 dry-run |
| `npm run worker:smoke` | 별도 임시 D1/R2에서 비밀번호·Access 모드 검증 |
| `npm run worker:preflight` | 운영 설정 생성 및 배포 dry-run |

로컬 API 스모크 테스트는 **별도의 폐기 가능한 데이터 디렉터리**로 서버를 실행한 뒤 수행합니다. 빈 저장소에 테스트 계정과 샘플 데이터를 생성하므로 개인 데이터 저장소에 실행하지 마세요.

```sh
node scripts/smoke-product.mjs http://127.0.0.1:8788
```

구현 및 검증 기록은 [개발 현황](docs/DEVELOPMENT.md)과 [검증 결과](docs/VERIFICATION.md)에 있습니다. 에뮬레이터 테스트는 실제 Access 이메일 전달·운영 정책이나 운영 CPU 사용량을 검증하지 않습니다.

## 관련 문서

- [문서 안내](docs/README.md)
- [제품 명세](docs/PRODUCT_SPEC.md)
- [아키텍처](docs/ARCHITECTURE.md)
- [실행·배포·복구](docs/DEPLOYMENT.md)
- [폴더 동작](docs/FOLDERS.md)
- [디자인](docs/DESIGN.md) · [디자인 구현 기준](design/folio/README.md) · [UI 인계](docs/UI_HANDOFF.md)
- [개발 현황](docs/DEVELOPMENT.md) · [검증 결과](docs/VERIFICATION.md)
- [데스크톱·모바일 클라이언트 계획](docs/CLIENT_APPS.md)
