# Folio documentation

Folio combines a personal planner, long-term goals, tasks and Markdown notes.

## Read first

1. [Product specification](PRODUCT_SPEC.md)
2. [Architecture and API](ARCHITECTURE.md)
3. [Design system](DESIGN.md)
4. [Development status](DEVELOPMENT.md)

## Build and operate

- [Repository quick start](../README.md)
- [Local and Cloudflare operation](DEPLOYMENT.md)
- [Verification results](VERIFICATION.md)
- [UI implementation handoff](UI_HANDOFF.md)
- [Folio Penpot implementation contracts](../design/folio/README.md)
- [Contributor constraints](AGENTS.md)

The browser is Leptos CSR. The shared Rust application selects native filesystem/SQLite or Cloudflare R2/D1 through an adapter crate. Markdown is canonical, and routine listings/search/planner queries use SQL indexes.

Home, Planner, Todo, Notes, Folders and Settings are the primary navigation. The product remains a quiet personal planner; graph views, tags, collaboration, AI and calendar integrations are outside this scope.
