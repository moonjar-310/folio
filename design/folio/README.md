# Folio 프론트엔드 구현 계약

> Penpot 디자인 정리 및 검수를 마쳤습니다. 검증 범위와 보관 구조는 [STATUS.md](./STATUS.md)를 확인하세요.

Penpot은 Stitch의 시각적 원본을 유지하면서 웹에서 재사용할 단위를 정의한다. 화면의 좌표나 줄바꿈을 그대로 HTML 구조로 옮기지 않는다.

## 컴포넌트 경계

| 계층 | 단위 | 책임 |
| --- | --- | --- |
| 토큰 | 색상 역할, 서체, 간격, 테두리, 모서리 | 테마와 공통 규칙. 화면별 하드코딩을 피한다. |
| 기본 요소 | Button, IconButton, Checkbox, TaskStatus, Badge, Keycap, SearchField | 하나의 입력·행동·상태를 표현한다. |
| 조합 요소 | TaskRow, GoalCard, NoteCard, NavigationItem, PlannerEvent | 데이터와 기본 요소를 조합하며 내부 배치만 관리한다. |
| 영역 | Sidebar, Topbar, PlannerDay, EditorBlock | 목록·슬롯·컨트롤을 배치한다. |
| 화면 | Home, Todo, Planner, Notes | 데이터 조회, 라우팅, 상태 소유, 반응형 영역 배치를 담당한다. |

## 반드시 같은 컴포넌트로 구현할 것

- 제목의 길이, 설명의 줄 수, 날짜, 항목 수는 props/children 차이다. 이를 이유로 GoalCard2, TuesdayColumn처럼 별도 컴포넌트를 만들지 않는다.
- PlannerDay는 `events[]`를 반복해서 PlannerEvent로 렌더링한다. 일곱 요일은 동일한 컴포넌트이며 today/weekend는 상태다.
- 기본·완료·우선순위 작업은 TaskRow의 `status`다. 시각 상태는 데이터 상태와 별개이며 hover/focus/disabled를 페이지 이름으로 분리하지 않는다.
- 테마는 앱 루트의 CSS 변수 범위에서 전환한다. LightGoalCard/DarkGoalCard 같은 별도 UI 컴포넌트는 만들지 않는다.
- 아이콘과 텍스트를 한 문자열로 넣지 않는다. 아이콘은 슬롯/prop, 본문은 실제 텍스트 요소로 전달한다.

## 레이아웃 계약

- 공통 컨트롤과 카드 내부는 Flex, 카드 목록과 주간 보드는 Grid로 구현한다. 절대 좌표는 아이콘 장식·현재 시간 표시처럼 실제 겹침이 필요한 부분에만 쓴다.
- 텍스트는 제목·설명·메타데이터 단위다. 디자인을 맞추기 위한 수동 `<br>`나 줄별 DOM 요소를 만들지 않는다.
- 읽기 영역은 최대 폭을 갖고, 카드/행은 컨테이너 폭을 따른다. 텍스트가 있는 flex 자식에는 `min-width: 0`을 지정한다.
- GoalCard 설명은 입력한 줄바꿈을 유지하며 최대 두 줄을 표시한다. 세 줄 이상은 말줄임표로 생략하고, Edit goal 모달에서 전체 설명을 확인·수정한다. NoteCard 미리보기는 한 줄 clamp, 전체 텍스트는 노트 본문에 보존한다.
- 화면의 날짜·온도·카운트·최근 수정 시각은 디자인 샘플이며 컴포넌트 내부 상수로 넣지 않는다.
- 긴 제목, 비어 있는 목록, 여러 줄 한국어, 좁은 폭에서도 검사한다. 화면별 조건은 화면/영역 컴포넌트에서 처리한다.

## 상태·접근성

- 버튼은 `<button>`, 이동은 `<a>`, 체크는 네이티브 checkbox를 우선한다.
- 아이콘 전용 버튼은 aria-label을 필수로 받는다. 라우트 선택은 `aria-current`, 토글은 `aria-pressed` 또는 checked 상태로 전달한다.
- hover만으로 행동을 숨기지 않는다. 키보드 focus-visible에서도 같은 행동을 이용할 수 있어야 한다.
- 상태 변화와 데이터 저장은 상위 상태 소유자에게 callback으로 전달한다. TaskRow가 직접 API를 호출하거나 전역 저장소를 선택하지 않는다.
- 디자인의 14–18px 아이콘은 glyph 크기다. 포인터·터치 타깃은 컨트롤 패딩으로 별도로 확보한다.

## Penpot 읽는 방법

페이지는 `01 — Design System`, `02 — Screens`, `03 — Archive` 세 개다. Design System에서 토큰·공통 요소·사용 규칙·템플릿을 함께 확인하고, Screens에서 실제 화면 5개를 비교한다. Archive는 원본 참고 이미지와 이전 라이브러리·반응형 초안을 보관한다. Screens의 숨겨진 `Previous drafts — retained backup` 그룹은 이전 화면 8개의 보존용이며 구현 기준이 아니다.

공통 라이브러리의 Core 컴포넌트를 새 구현의 기준으로 사용한다. Stitch에서 옮긴 화면과 Source patterns는 원본의 내용·배치와 화면 구성 예시를 확인하는 자료다. 데이터 내용이나 항목 수 때문에 나뉜 source 패턴은 그대로 컴포넌트 종류로 옮기지 않는다.

`component-contracts.ts`는 프레임워크와 무관한 props 계약이다. 디자인 시안이 모든 제품 동작과 반응형 화면을 구현했다는 뜻은 아니다. 필요한 API 연동과 실제 상호작용 테스트는 프론트 구현 단계에서 수행한다.

## 아이콘

`icons/`에 Material Symbols SVG 52종과 라이선스를 포함했다. `fill="currentColor"`로 색상 토큰을 상속하며, `icon-names.ts`가 허용되는 이름을 정의한다. 아이콘 폰트의 ligature 문자열을 UI 텍스트로 넣지 않는다. Penpot에서도 동일한 SVG 벡터를 공통 아이콘 에셋으로 사용한다.

## Folder explorer update — 2026-09-15

`02 — Screens`에 `Folio / Folders / Tree and context menus / Light`와
`Folio / Folders / Dialogs and collapsed state / Dark` 보드를 추가했다.
폴더/파일 계층, 펼침·접힘, ⋮/우클릭 메뉴, 이름 변경·하위 폴더·파일 생성 UI를
네이티브 편집 가능한 레이어로 정의했다. [구현 계약](../../docs/FOLDERS.md)을 따른다.

## Goal editor update — 2026-09-17

Penpot MCP로 기존 GoalCard와 Home의 Light/Dark 시안을 수정하고,
`02 — Screens`에 편집 모달의 Light, Dark, Mobile 보드를 추가했다.

- 카드 설명: 줄바꿈 유지, 두 줄까지 표시, 세 줄 이상은 `…`로 생략. 원문은 저장 데이터에 유지.
- Edit goal: 네이티브 모달, 600 px 너비, 모바일 좌우 여백 16 px, 화면 높이를 넘으면 스크롤.
- Description: 전체 원문을 편집하는 넓은 textarea. 줄바꿈·빈 줄 유지, 세로 크기 조절 및 스크롤 지원.
- Cancel/닫기/Escape: 저장 없이 닫고 호출 버튼으로 포커스 복귀. 다시 열면 저장된 내용을 표시.
- Save: 해당 목표만 저장 중 상태로 전환. 실패하면 모달에 오류와 입력 내용을 유지하고 재시도 허용.

[Penpot 편집 모달 시안](../../docs/UI_HANDOFF.md)

검수: 브라우저 WASM 빌드, 로컬 API 응답을 사용하는 실제 웹에서 여러 줄 저장·재열기,
실패 후 입력 보존·취소, 데스크톱 및 390 px 모바일 배치 확인.
Penpot 파일 유효성 및 모달 시안 경계 검사 통과.
