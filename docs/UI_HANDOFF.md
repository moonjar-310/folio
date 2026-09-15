# Folio UI implementation

## Current Rust application — 2026-09-15

The Folio Penpot file was read through MCP, including Home, Todo, Planner, Notes and the common sidebar. The implementation follows the product specification when source patterns contain additional design-only content.

- `crates/web/src/components.rs`: Sidebar, Folio emblem/SVG icons, TaskRow, TaskForm, GoalCard, NoteCard and section headings.
- `crates/web/src/folder_tree.rs`: nested folder/file explorer, disclosure state, context menus and create/rename dialogs. [Folder interaction contract](FOLDERS.md).
- `crates/web/src/pages.rs`: Home, Todo, daily/weekly Planner and Settings.
- `crates/web/src/editor.rs`: Notes browser, rename/move/archive/delete, Markdown editor and sanitized preview.
- `crates/web/src/overlays.rs`: authentication and modal SQL search.
- `crates/web/src/state.rs`: API calls, editor persistence, navigation, task/goal updates, dates and theme state.
- `crates/web/style.css`: shared Light/Dark tokens, 240 px sidebar, responsive layouts and Markdown typography.

The UI uses the provided Folio emblem and SVG icon assets. It uses semantic HTML, reusable components and actual data. Sample dates/counts, tags, analytics, artificial synchronization claims and unrelated source-pattern controls are not product features.

The desktop sidebar is persistent. Below 850 px it becomes a drawer; below 600 px the note browser/editor and weekly planner stack vertically. CSS variables switch themes without recreating application state. Font stacks use Newsreader, Geist and JetBrains Mono with readable system fallbacks.

See [Folio design contracts](../design/folio/README.md), [design tokens and direction](DESIGN.md), and [verification results](VERIFICATION.md).

## Historical reference

The earlier Node preview and editable Penpot assembly assets remain in `preview/`, `ui/`, `public/` and `design/penpot/`. Run the preview using `npm run preview:dev`. These are design references; the Rust Leptos application is the current implementation.

The connected Penpot file contains `01 — Design System`, `02 — Screens`, and `03 — Archive`. The hidden previous-drafts group is a backup. Core components and the five current screen instances guide implementation; source-pattern variants are not separate frontend components.

Private endpoint credentials and file identifiers remain outside public documentation.
