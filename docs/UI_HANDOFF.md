# UI implementation and Stitch handoff

## Source

Source: the owner's private Minimal Paper Planner Stitch project. Private project identifiers and download links are intentionally excluded from the public repository.

Retrieved through Stitch MCP on 2026-09-14. Four light app screens, a dark Home screen, and the emblem were inspected. The private export is retained locally under ignored `design/stitch/`.

## Design decisions

Keep the Stitch theme's warm papers, Newsreader headings, Geist interface text, JetBrains Mono metadata, hairline borders, ruled task rows, sidebar, and focused editor column. Apply the product docs' muted green to selected navigation, primary actions, and completion. Keep terracotta as a small editorial accent. Normalize radii to 4/6/8 px.

The original Stitch export contains a broken emblem represented by URL text, decorative sync claims, tags, backlinks, benchmark widgets, and future daily-log append behavior. The implementation uses a simple book outline and real local save status, and follows the product specification for supported features.

## Component map

| Family | Components | Code |
| --- | --- | --- |
| Navigation | sidebar, navigation item, notebook row, search trigger, topbar, view switch | `sidebar`, `layout`, page markup |
| Actions | primary, secondary, text, disabled, icon | `button`, `icon` |
| Tasks | open, completed, source link, due date, inline add | `taskRow`, `taskForm` |
| Content | goal, note row, section heading, quick note | `goalCard`, `noteRow`, `sectionHeading`, Home |
| Editor | title, notebook select, body, preview, status, footer | Notes + `public/app.js` |
| Feedback | empty, failed save, recovered draft, local status | page markup + editor state |
| Overlay | search dialog, mobile drawer | `layout`, `sidebar` |

## Responsive behavior

- Wide desktop: 240 px navigation, centered Home writing column, full weekly spread.
- Below 850 px: collapsible navigation.
- Below 600 px: goals and notes stack; weekly columns become daily sections; the note list sits above the editor.
- Visible focus, keyboard shortcuts, reduced-motion support, labeled controls, live save feedback, and skip navigation.

## Penpot handoff

Generate with `npm run design:export`. The component SVGs use editable text, rectangles, lines, and groups. Install Newsreader, Geist, and JetBrains Mono in the target environment when exact font matching is required.

Created native Penpot pages: `00 — Foundations`, `01 — Components`, `02 — Screens`, `03 — Responsive`, and a private Stitch reference page. Desktop Home, Todo, Planner, and Notes have Light/Dark versions; mobile Home and Notes also have both themes. Screens use native editable text/shapes and registered component instances, not flattened screenshots. Foundations include 18 library colors, six typography styles, and 29 semantic/spacing/radius tokens.

The private Penpot endpoint, token, file identifiers, and connection diagnostics are intentionally excluded from this public handoff. See the local handoff status for native import progress.

## Latest reference update

Navigation uses Folders. Light and Dark share semantic tokens and component geometry. Dark colors follow the updated Stitch screen, with sage retained for interactive and completed states. The sidebar and Settings expose a persistent theme switch. Public previews contain synthetic sample content only.

Official references: [Penpot MCP](https://help.penpot.app/mcp/), [components](https://help.penpot.app/user-guide/design-systems/components/), [tokens](https://help.penpot.app/user-guide/design-systems/design-tokens/).
