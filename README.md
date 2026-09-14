# Paper & Folio UI

A server-rendered UI preview extracted from the user's Minimal Paper Planner Stitch project. See [project docs](docs/README.md) for the product and production architecture.

## Run

Requires Node.js 22 or newer. No packages to install.

```sh
npm run dev
```

Open http://127.0.0.1:4173. The preview binds only to localhost.

```sh
npm run check
npm test
npm run design:export
```

## Included

- Home, daily/weekly planner, task groups, notes browser/editor, settings, and `/design-system`.
- Reusable HTML components in `ui/components.mjs`; shared CSS tokens in `public/tokens.css`.
- Native forms, server-rendered pages, minimal JavaScript for editor autosave, shortcuts, and navigation drawer. No SPA framework or browser WASM.
- Local task creation/completion/date changes, goal creation/completion, quick notes, note search/rename/move/archive, Markdown preview, and notes export.
- Editor saves after three seconds idle, on blur, navigation, and Ctrl/Cmd+S. Failed saves retain dirty state and a browser draft backup.
- Light/Dark themes, system preference on first visit, persistent manual toggle, and Folders navigation.
- Private Stitch exports are retained locally in ignored `design/stitch/`; they are not included in the public repository.
- Editable SVG assets, token JSON, and native Penpot assembly scripts in `design/penpot/`. The private Penpot file contains native reusable components, desktop/mobile screens, and Light/Dark foundations.

## Preview boundary

`preview/` is a development-only Node HTTP adapter so the UI can be inspected without Cloudflare credentials. Its JSON data lives in ignored `.local/preview.json`. Do not deploy this adapter as the production service.

The production plan remains Rust Worker SSR, canonical R2 Markdown, and derived D1 data, as specified in `docs/ARCHITECTURE.md`. Cloud authentication, R2/D1 bindings, Markdown task indexing, and failure recovery across R2/D1 are not implemented by this UI task. The preview's seeded source-note task links do not synchronize checkbox changes with Markdown.

The UI deliberately excludes tags, backlinks, graphs, collaboration, AI, calendar integrations, and analytics. Quick Note creates a normal note, per the MVP specification.
