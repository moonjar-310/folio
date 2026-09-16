# 개인용 데스크톱·모바일 앱 검토

검토일: 2026-09-16. 상태: **방향 합의 및 타당성 검토 문서. 클라이언트 구현 전.**

## 결정한 방향

Folio를 개인 서비스로 유지하면서 웹과 별도 설치 앱에서 같은 노트·할 일·플래너 데이터를 사용한다. 서버는 기존 Cloudflare Workers + D1 + R2를 유지한다. 이번 목적 때문에 VPS로 이전할 필요는 없다.

- 데스크톱: `desktop-tauri`에 Tauri 기반 별도 클라이언트를 둔다. 대상은 Windows·macOS다.
- 모바일: `mobile-flutter`에 Flutter 기반 별도 클라이언트를 둔다. 대상은 Android·iOS다.
- 같은 Git 저장소에서 관리하되, 각 클라이언트의 소스·의존성·빌드·배포를 분리한다.
- 개인 기기에 직접 설치하는 방식을 우선한다. 공개 스토어 출시는 보류한다.
- 광고, 유료화, 결제, 공개 회원가입은 이번 범위에 포함하지 않는다.

이 문서는 기존 웹을 리팩터링하거나 앱 프로젝트를 생성하라는 구현 지시가 아니다. 앱 구현, 인증 변경, 배포 설정 변경은 후속 작업으로 구분한다.

## 제안하는 모노레포 구조

아래의 `desktop-tauri`와 `mobile-flutter`는 향후 추가할 디렉터리다. 현재 존재하는 것으로 해석하지 않는다.

```text
folio/
├── crates/
│   ├── web/                  # 기존 Leptos 웹 클라이언트
│   ├── worker/               # 기존 Cloudflare HTTP 진입점
│   ├── application/          # 기존 서버 업무·인증 처리
│   ├── core/                 # 기존 Rust 모델·검증·저장소 계약
│   └── ...                   # 기존 로컬 서버·저장소 어댑터
├── desktop-tauri/            # 별도 데스크톱 UI 및 Tauri 호스트
│   └── src-tauri/
├── mobile-flutter/           # 별도 Flutter UI 및 API 클라이언트
│   ├── lib/
│   ├── android/
│   └── ios/
├── docs/
└── .github/workflows/
```

기존 `crates/web`를 이동하거나 앱 전용 코드로 바꾸지 않는다. Tauri의 UI 기술은 후속 설계에서 정한다. Leptos 연동은 가능한 선택지지만, 기존 웹 소스를 그대로 묶는 것으로 확정하지 않는다. [Tauri의 Leptos 연동 안내](https://v2.tauri.app/start/frontend/leptos/)

Flutter는 화면과 데이터 접근을 분리하고, HTTP 통신·토큰 갱신을 화면 코드 밖에 둔다. [Flutter 아키텍처 안내](https://docs.flutter.dev/app-architecture/guide)

초기 공유 경계는 **HTTP API 계약**이다. Flutter가 Rust 서버 크레이트를 직접 호출하도록 만들거나, 클라이언트마다 서버 업무 규칙을 복제하지 않는다. 향후 요청·응답·오류 형식을 문서화하고 계약 검증을 추가할 수 있으나, OpenAPI 생성이나 공통 SDK는 아직 구현하지 않았다. Tauri의 Cargo workspace 편입 여부도 기존 Worker CI에 플랫폼 의존성이 유입되지 않도록 별도로 결정한다.

## 서버와 데이터 재사용의 타당성

웹과 두 앱 모두 같은 Cloudflare JSON API를 호출하는 구성이 가능하다. Markdown 원본과 파일 경로는 R2에, 검색·플래너 인덱스 및 인증 데이터는 D1에 유지한다. 앱에는 Cloudflare 관리 토큰, D1·R2 접근 키 또는 JWT 서명키를 넣지 않는다.

노트 목록·검색·열기·저장, 할 일, 플래너, 목표, 폴더 API는 재사용 대상이다. 기존 응답의 페이지 구분, revision 기반 충돌 감지, 저장 실패 후 초안 보존 규칙을 클라이언트에서도 지켜야 한다. 상세 계약은 [현재 아키텍처와 API](ARCHITECTURE.md)를 기준으로 검토한다.

같은 서버를 사용한다는 것이 실시간 동기화나 오프라인 편집을 의미하지는 않는다. 초기에는 온라인 조회·편집을 제안한다. 다른 기기의 변경은 서버에서 다시 조회할 때 반영하고, 충돌 시 사용자 초안을 덮어쓰지 않는다. 오프라인 편집에는 로컬 저장소, 변경 대기열, 충돌 처리, 로그아웃 시 데이터 처리까지 별도 설계가 필요하다. 기존 로컬 서버의 filesystem/SQLite 저장소는 클라우드 동기화 클라이언트가 아니다.

## 핵심 선행 과제: 앱 로그인

현재 인증은 [인증 구현 문서](AUTH_OPTIONS.md)에 설명된 브라우저 흐름이다.

- Cloudflare Access가 운영 호스트와 API를 보호하며, Worker는 요청마다 Access 신원을 검증한다.
- Folio Access JWT는 5분, Refresh Token은 최초 로그인부터 1주일의 절대 만료를 사용한다.
- 일반 API는 Folio JWT를 검증하며 인증을 위해 세션 DB를 조회하지 않는다.
- Refresh Token은 HttpOnly 쿠키로 전달되고, DB에는 해시만 저장한다. 갱신은 원자적으로 교체한다.
- 쓰기 요청은 CSRF 검증을 사용하며, refresh는 명시적인 동일 출처 Origin도 요구한다.

별도 앱이 Folio JWT만 보내면 현재 API를 바로 사용할 수 있다고 가정하면 안 된다. 시스템 브라우저, 앱 WebView, 네이티브 HTTP 클라이언트가 쿠키를 자동으로 공유하지 않으며, Cloudflare Access 인증과 Folio 인증을 모두 만족해야 한다. Cloudflare Access의 쿠키 동작은 [공식 설명](https://developers.cloudflare.com/cloudflare-one/access-controls/applications/http-apps/authorization-cookie/)을 참고한다.

구현에 앞서 다음을 작은 로그인 검증 프로젝트에서 확인해야 한다.

1. 시스템 브라우저의 Cloudflare 로그인을 앱으로 안전하게 연결할 수 있는지 확인한다. 반환 경로와 일회용 교환 코드, 요청을 시작한 앱과의 연결 검증은 설계 후보이며 아직 구현되지 않았다.
2. 이후 앱 API 요청에도 필요한 Cloudflare Access 자격 증명과 그 갱신·만료 처리를 확인한다. 로그인 콜백만 만든다고 이 문제가 해결되지는 않는다.
3. 앱 Refresh Token의 전송 방식과 OS 보안 저장소 사용을 정한다. 기존 웹 쿠키·CSRF 보호를 유지하면서 앱용 인증 경로가 필요한지 판단한다. Origin 헤더를 임의로 붙이는 것으로 보안 설계를 대체하지 않는다.
4. 앱별로 독립적인 Refresh 세션을 발급하고, 앱 내부의 동시 갱신을 하나로 합치는 방식, 로그아웃, 장기 미사용 후 재로그인, 초안 복구를 검증한다.

이 흐름을 지원하려면 서버 인증 진입점이나 Cloudflare 라우팅에 제한적인 변경이 필요할 수 있다. 구체적인 변경은 검증 결과를 보고 결정한다. 앱 안에 공용 서비스 토큰을 배포하거나 기존 Access 보호를 일괄 해제하는 방식은 채택하지 않는다. 기존 5분/1주일 토큰 수명과 서버의 해시 저장 원칙은 유지한다.

## 개인용 설치와 스토어 출시

앱 제작과 공개 스토어 출시는 별개다. 현재는 다음 방식을 우선 검토한다.

| 대상 | 개인용 설치 방향 | 준비·유지 과제 |
| --- | --- | --- |
| Windows | Tauri 설치 파일 직접 설치 | Windows 빌드 환경, 버전 관리, 배포 서명 여부 검토 |
| macOS | Tauri 앱 번들/설치 파일 직접 설치 | macOS 빌드 환경, 서명·공증 및 Gatekeeper 처리 검토 |
| Android | Flutter APK를 본인 기기에 직접 설치 | Android SDK, APK 서명키 보관, 동일 키로 업데이트 |
| iOS | 개발 기기 설치 또는 TestFlight 검토 | macOS/Xcode, 개발자 계정·프로비저닝, 빌드 만료에 따른 재배포 |

데스크톱 패키징과 Android 직접 설치는 각각 [Tauri 배포 안내](https://v2.tauri.app/distribute/)와 [Flutter Android 배포 안내](https://docs.flutter.dev/deployment/android)를 기준으로 준비한다. 직접 설치는 스토어 심사를 줄이지만 플랫폼의 서명·설치 보안 요구를 없애지는 않는다.

2026-09-16에 확인한 스토어·개발 배포 조건은 다음과 같다. 실제 출시 시점에 다시 확인한다.

- **Google Play:** 2023년 11월 13일 이후 생성한 개인 개발자 계정은 프로덕션 접근 신청 전에 최소 12명의 테스터가 연속 14일 참여하는 비공개 테스트를 요구한다. 조건 충족 후 프로덕션 접근을 신청하며, 자동 출시 승인을 의미하지 않는다. 본인 기기에 APK를 직접 설치하는 데 필요한 테스터 수 조건은 아니다. [Google 공식 요건](https://support.google.com/googleplay/android-developer/answer/14151465?hl=en-GB)
- **Apple App Store:** TestFlight는 선택 사항이다. Google Play와 같은 사전 테스터 모집 조건으로 혼동하지 않는다. 공개 출시는 별도의 App Review를 거친다. [Apple TestFlight 설명](https://developer.apple.com/help/glossary/testflight-beta-testing/)
- **무료 Apple Personal Team:** 기기 설치용 프로비저닝 프로파일은 발급 후 7일 만료이므로 장기 개인 사용에도 갱신 부담이 있다. [개발 계정 안내](https://developer.apple.com/help/account/basics/about-your-developer-account)
- **TestFlight:** Apple Developer Program 및 App Store Connect 구성을 전제로 검토한다. 빌드는 90일 동안 테스트할 수 있어 새 빌드 업로드를 계속 관리해야 한다. 영구 개인 배포 수단으로 보지 않는다. [TestFlight 안내](https://developer.apple.com/testflight/) · [내부 테스트 안내](https://developer.apple.com/help/app-store-connect/test-a-beta-version/add-internal-testers)

iOS를 포함한 모든 플랫폼에 같은 비용·설치 편의성을 약속하지 않는다. 초기 실기기 검증 순서는 사용 기기와 개발 환경을 확인한 뒤 정하되, 현재 Windows 환경에서는 Windows·Android를 먼저 검토하고 macOS·iOS 빌드는 Mac 또는 macOS CI 준비 후 진행하는 방안을 제안한다. [Tauri 개발 환경 요구사항](https://v2.tauri.app/start/prerequisites/)

## 향후 CI/CD 경계

기존 웹/Worker 자동 배포와 앱 패키징을 별도 워크플로로 관리하는 방향을 제안한다. 앱 코드 변경만으로 서버가 재배포되지 않도록 변경 경로를 구분하고, 서버/API 계약 변경에는 필요한 클라이언트 호환성 검사를 연결한다. 실제 앱 배포는 별도의 릴리스 태그 또는 수동 실행을 검토한다.

현재 Worker 워크플로는 `main` 푸시마다 실행된다. 위 경로 필터나 앱 워크플로는 아직 추가하지 않았다. 이번 문서 변경도 기존 자동 워크플로를 트리거할 수 있다.

Android 서명키, Apple 인증서·프로파일, 배포용 비밀값은 소스에 넣지 않는다. GitHub 저장소가 공개되어 있으므로 GitHub Releases에 올린 설치 파일도 개인 전용 보관소가 아니다. 설치 파일의 공개 범위와 별개로 앱은 항상 사용자 인증을 요구하며 서버 관리 자격 증명을 포함하지 않아야 한다.

## 후속 구현을 시작할 때의 검증 순서

1. 실제 대상 기기, Mac 사용 가능 여부, iOS 배포 방식, 온라인 우선 범위를 확인한다.
2. 별도 앱에서 로그인 → 인증 API 조회 → 5분 만료 후 갱신 → 재실행 → 로그아웃을 검증한다. 인증 연결이 성립하기 전 전체 UI 구현을 시작하지 않는다.
3. 독립 클라이언트 프로젝트를 추가하고 노트 목록·열기·저장부터 연결한다. 기존 웹 소스 재구성은 전제하지 않는다.
4. 두 기기의 같은 노트 수정 충돌, 네트워크 실패, 백그라운드 전환과 초안 보존을 검증한 뒤 할 일·플래너 등으로 확장한다.
5. 개인 기기에 설치하고 동일 서명으로 업데이트되는지 확인한다. 충분히 사용한 뒤 공개 스토어 출시 필요성을 다시 판단한다.

이번 변경의 산출물은 이 검토 문서와 문서 색인 링크뿐이다. 앱 소스, 서버 인증 변경, 빌드 파일, 스토어 등록은 포함하지 않는다.
