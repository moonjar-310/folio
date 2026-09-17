# Folder explorer

Implemented on 2026-09-15 after adding and exporting the corresponding native, editable Penpot boards in `02 — Screens`:

- **Folio / Folders / Tree and context menus / Light**
- **Folio / Folders / Dialogs and collapsed state / Dark**

## Interaction

- Click a folder row or its chevron to expand/collapse its children. Nested folders precede that folder's Markdown files.
- Click a file to open it and switch the middle note list to its containing folder. An open document stays open when its parent is collapsed.
- Every folder has a visible **⋮** button. It opens **Rename folder**, **New folder**, and **New file**.
- Right-click a folder row for the same menu. Right-clicking a file targets its containing folder.
- The **Folders** heading menu and the section's empty area offer **New folder** and **New file**. A root-level new file goes into Personal.
- New folder/file names are single names, not paths. Nesting comes from the selected parent. `.md` is optional when naming a new file.
- Folder buttons support Enter/Space, Left/Right arrows and Shift+F10. Menus support Up/Down, Home/End and Escape; dialogs use normal keyboard focus containment.
- Expanded paths persist in browser storage. Collapsing a parent preserves its descendants' expansion choices for the next time it is opened.
- The same controls are available in the mobile navigation drawer. Deep paths can scroll inside the explorer, and full paths are available in row tooltips.

## Implementation

`crates/web/src/folder_tree.rs` owns the explorer, context menu, name dialog and file-list cache. Folder paths are physical vault paths / R2 key prefixes. Optional `.folio/notes` records preserve stable IDs across application moves.

The tree queries `GET /api/notes?folder=...&exact=true` only for visible, expanded folders, with 50-file pagination and a **More files…** action. Listings enumerate storage keys and use available SQL metadata; they do not read Markdown bodies. Saves update cached metadata; normal expand/collapse does not poll storage.

`PUT /api/folders` accepts `{ "path": "Projects/Folio", "name": "My project" }`. It renames the folder and its descendants. Existing destination folders and invalid/overlong descendant paths are rejected before writes.

A rename moves each affected Markdown file to its new path using revision-checked, journaled copies. Markdown bodies and task identities are preserved. An internal SQL lease serializes folder mutations and note writes so a save cannot add notes to a subtree midway through a rename.

The current pending operation is persisted in `.folio/folder-rename.json`; the SQL table from `0004_folder_renames.sql` remains a legacy fallback. Each PUT moves up to two notes and returns `done`; the UI sends subsequent batches until complete. Writes affecting that hierarchy are rejected while it is pending. After moving notes, the operation recreates destination folder markers, removes source markers, updates folder SQL rows in a batch and removes the canonical journal. Marker cleanup is not limited to two entries per request.

If a request fails or the browser closes, **Resume folder rename…** appears in the Folders section after reopening. Retrying uses canonical content already written and repairs its index. This is a resumable operation across two stores, not a cross-store transaction or an all-or-nothing rollback.

Local SQLite migrates automatically on server startup. Cloudflare deployments must apply all pending migrations from `migrations/` (currently 0001–0007), including the path-storage and refresh-token updates. See [deployment](DEPLOYMENT.md).

## Verification

- Shared application test: five notes, nested and empty descendants, Unicode paths, duplicate/invalid names, an injected index failure, blocked concurrent hierarchy writes, resumable batches, canonical paths/content, stable task IDs, and stale revisions.
- Tree test: collapsed descendants/files are absent, correct nesting depth and hidden-ancestor detection.
- The same HTTP smoke workflow passed on native filesystem/SQLite and local Worker R2/D1, including a multi-request rename.
- Browser review: nested creation, ⋮ menu, right-click file creation, rename with an edited note, keyboard context menu, and collapse behavior. See [verification results](VERIFICATION.md).

## Draft recovery during a pending rename

A pending rename can resume even when the editor contains an unsaved draft. The client keeps that draft's body and original conflict revision, updates its folder path when the operation finishes, and retains the browser backup. It does not replace the draft with the canonical version or silently accept a newer revision. If Save reports a conflict, **Save draft as a new note** preserves the draft separately.

Note-list requests carry a client generation number so an older folder response cannot replace a newer selection.
