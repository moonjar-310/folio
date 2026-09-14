# Development Plan

## Phase 0 — Bootstrap

- Rust Worker project
- routing
- Leptos CSR build and shared JSON contracts
- static assets
- D1 binding
- R2 binding
- local dev setup
- formatting/linting

Done when:
- Worker deploys
- static CSR shell loads without invoking the Worker
- health API returns JSON
- D1 and R2 bindings work

Bootstrap implemented: Cargo workspace, Trunk CSR note editor, JSON list/get/put routes, Worker D1/R2 adapter, standalone Axum/filesystem/SQLite adapter, input validation, explicit saves and failure states. This is not the full Notes/Editor phase. Deployment, authentication, autosave, task indexing and multi-client conflict detection remain pending. Production note routes fail closed until authentication exists.

## Phase 1 — Authentication

Implement:
- setup/login
- logout
- server-side sessions
- auth guard

Done when:
- unauthenticated users cannot access app
- login/logout work correctly

## Phase 2 — Notes Core

Implement:
- create
- open
- save
- rename
- move
- archive/delete
- folder tree
- note list

Rules:
- R2 stores Markdown
- D1 stores metadata

## Phase 3 — Editor

Implement:
- Markdown editing
- dirty state
- ~3 second debounce
- Ctrl/Cmd + S
- save status
- basic render/preview

Do not save per keypress.

## Phase 4 — Search

Implement:
- normalize searchable text
- D1 search
- title/preview/path results
- Ctrl/Cmd + K

No R2 scan.

## Phase 5 — Todo

Implement:
- Markdown checkbox parsing
- task index
- Today
- Upcoming
- Someday
- Completed
- source note link
- complete/reopen task

Be careful about task identity when re-parsing Markdown.

## Phase 6 — Home

Implement:
- current date
- What Matters / Goals
- today's tasks
- Quick Note
- Recently Edited
- Daily Note access

## Phase 7 — Planner

Implement:
- Daily
- Weekly
- date navigation
- task creation/completion
- due date changes

Do not build calendar integration.

## Phase 8 — Goals

Implement:
- create
- edit
- reorder
- complete
- archive

No Goal → Todo link required.

## Phase 9 — Responsive / Polish

- Folders terminology throughout navigation and editor controls
- Light/Dark semantic tokens and pre-paint theme selection
- persistent theme toggle in sidebar and Settings
- verify theme changes while editing do not discard drafts

- tablet sidebar behavior
- mobile drawer
- keyboard navigation
- focus states
- loading/error states
- accessibility
- visual cleanup

## Security

Required:
- no plaintext password storage
- HttpOnly session cookie
- Secure cookie in production
- SameSite
- CSRF protection for mutation routes
- input size limits
- path validation
- path traversal prevention
- escape HTML
- disable raw Markdown HTML initially unless safely sanitized

## Performance / Cost

- no R2 scan on normal render
- no R2 GET per search result
- no save per keypress
- bounded D1 queries
- paginate large lists
