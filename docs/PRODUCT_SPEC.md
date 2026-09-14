# Product Specification

## 1. Goal

Build a browser-accessible personal planner and Markdown note app.

The product combines:
- Obsidian-style Markdown notes
- Todo management
- Daily / Weekly planning
- Long-term Goals

Primary user: one person.

Out of scope initially:
- collaboration
- teams
- comments
- graph view
- tags
- plugins
- Notion-style databases
- AI features
- external calendar integrations

## 2. Product Principles

### Simple
Only implement functions that materially improve daily use.

### Fast
Navigation, note opening, search, and task creation should feel immediate.

### Web-first
Accessible from desktop, tablet, and mobile browser.

### Self-hosted
Deployable to the user's own Cloudflare account.

### Zero-cost oriented
Normal personal usage should stay within Cloudflare Free Tier whenever possible.

### Portable
Markdown is the canonical data format.

## 3. Main Navigation

Home
Planner
Todo
Notes

Folders
- Daily
- Projects
- Personal
- Archive

Settings

## 4. Home

Default landing page.

Shows:
- Current date
- What Matters / Goals
- Today's Todo
- Quick Note
- Recently Edited Notes

Do not add:
- analytics
- streaks
- productivity score
- charts

## 5. Goals

Long-term direction, distinct from Todo.

Fields:
- id
- title
- description?
- order
- status
- created_at
- updated_at

Status:
- active
- completed
- archived

MVP does not require Goal → Todo linking.

## 6. Todo

Tasks can come from:
1. Markdown checkboxes in Notes
2. Explicit task creation

Groups:
- Today
- Upcoming
- Someday
- Completed

Fields:
- id
- title
- completed
- due_date?
- due_time?
- source_note_id?
- created_at
- updated_at

## 7. Planner

Task-oriented planner.

Views:
- Daily
- Weekly

Supports:
- date navigation
- task creation
- task completion
- due date changes

Does not support initially:
- external calendar sync
- meeting scheduling
- complex time blocking

## 8. Notes

Supports:
- create
- open
- edit
- rename
- move
- archive/delete
- sort by updated time
- search

Note list displays:
- title
- preview
- notebook/path
- updated time

## 9. Folders

Represents Markdown folder hierarchy.

Example:

Folders
- Daily
- Projects
  - Note App
  - MPC
- Personal
- Archive

Cloud:
notes/projects/mpc.md

Local:
vault/projects/mpc.md

## 10. Note Editor

Most important screen.

Supported Markdown:
- headings
- paragraphs
- lists
- checkboxes
- links
- code blocks
- blockquotes
- tables

Excluded initially:
- Graph View
- Tags
- Backlinks visualization
- Canvas
- Plugin system
- Block database

## 11. Autosave

Do not save per keystroke.

Default:
- debounce after about 3 seconds idle

Immediate flush:
- Ctrl/Cmd + S
- note navigation
- editor blur
- page navigation when safe

Save pipeline:
R2 PUT → D1 metadata/task/search update

## 12. Search

Search D1 first.

Flow:
query → D1 → result metadata → user opens result → R2 GET

Do not scan R2 for normal search.

## 13. Quick Note

Available on Home.

Initial behavior:
- create a normal note quickly

Future:
- append to Daily Note

## 14. Daily Note

Path example:
Daily/2026-09-14.md

Home should link directly to today's Daily Note.

## 15. Keyboard Shortcuts

- Ctrl/Cmd + K: Search / Command Palette
- Ctrl/Cmd + N: New Note
- Ctrl/Cmd + S: Save
- Ctrl/Cmd + Shift + N: Quick Note

## 16. Responsive

### Appearance

- Light and Dark themes across all screens.
- Follow system appearance initially; persist an explicit local override.
- Toggle from sidebar or Settings without losing editor state.
- User-facing folder navigation is named Folders. Existing internal `notebook` fields may remain during the UI phase to avoid unnecessary data migration.

Desktop:
- persistent sidebar

Tablet:
- collapsible sidebar

Mobile:
- drawer navigation
- prioritize Home, Todo, Notes, Quick Note
