# Architecture

## 1. Overview

Browser
  |
  | HTML / HTMX / minimal JS
  v
Cloudflare Worker
  |- SSR
  |- Auth
  |- Notes
  |- Todo
  |- Planner
  |- Goals
  |- Search
  |
  |- R2: canonical Markdown
  `- D1: derived metadata and indexes

## 2. Runtime

Server:
- Rust
- wasm32-unknown-unknown
- Cloudflare Workers

Client:
- HTML
- CSS
- HTMX
- minimal Vanilla JavaScript

Rust WASM runs server-side. Do not ship Rust WASM to the browser unless explicitly required later.

## 3. Storage Roles

### R2
Canonical note content.

Responsibilities:
- Markdown files
- future attachments
- folder/path hierarchy

### D1
Derived/query data.

Responsibilities:
- note metadata
- path
- title
- preview
- updated_at
- search text
- parsed tasks
- goals
- sessions

D1 should be rebuildable from R2 where practical.

## 4. Suggested Tables

### notes
- id
- path
- title
- preview
- search_text
- created_at
- updated_at
- size

### tasks
- id
- note_id?
- title
- completed
- due_date?
- due_time?
- created_at
- updated_at

### goals
- id
- title
- description?
- position
- status
- created_at
- updated_at

### sessions
- id
- token_hash
- expires_at
- created_at

### users
- id
- username
- password_hash
- created_at
- updated_at

## 5. Save Pipeline

Editor
→ debounce / explicit save
→ Worker
→ validate
→ R2 PUT Markdown
→ parse metadata/tasks/search text
→ D1 update

R2 is written first because Markdown is canonical.

If D1 update fails, note content must remain recoverable.

## 6. Read Pipelines

### Note List
Browser → Worker → D1 → SSR HTML

### Open Note
Browser → Worker → D1 metadata + R2 Markdown → SSR HTML

### Search
Browser → Worker → D1 → result list

Do not fetch all R2 objects for note list/search/todo/home.

## 7. Portability

Core logic must not directly depend on Cloudflare APIs.

Use abstractions:

```rust
trait NoteStorage {
    async fn get(&self, path: &str) -> Result<String, Error>;
    async fn put(&self, path: &str, content: &str) -> Result<(), Error>;
    async fn delete(&self, path: &str) -> Result<(), Error>;
}
```

And:

```rust
trait MetadataRepository {
    // notes, tasks, goals, search
}
```

Implementations:

Cloud:
- R2NoteStorage
- D1MetadataRepository

Local:
- FilesystemNoteStorage
- SQLiteMetadataRepository

## 8. SSR + HTMX

Use server-rendered HTML by default.

HTMX is appropriate for:
- task completion
- task creation
- goal state changes
- note list refresh
- small partial updates

Editor-specific save behavior may use minimal JS/fetch where cleaner.

## 9. Auth

Single-user username/password authentication.

Use server-side sessions with:
- HttpOnly
- Secure
- SameSite cookies

Never store plaintext passwords.

## 10. Failure Model

R2 write succeeds / D1 fails:
- keep Markdown
- surface/retry index update
- support future re-index

Network failure while editing:
- preserve dirty state
- show unsaved status
- never silently discard edits

D1 loss:
- rebuild from R2

## 11. Cost Rules

- no R2 scan for normal UI
- no R2 GET per search result
- no save per keypress
- bounded D1 queries
- avoid polling
