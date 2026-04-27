# RADcite Functional DOCX Design

## Status

Approved in conversation on 2026-04-28 as the next step after merging the Svelte desktop shell.

## Context

RADsuite now has Rust RADcite domain models, SQLite persistence, citation analysis, DOCX ingestion, and a Svelte/Tauri desktop shell. The missing piece for first functional testing is a user-visible path that runs the real Rust DOCX pipeline from the desktop app.

The current desktop shell has one Tauri command, `get_app_status`, and no project or RADcite workflow UI. The next slice should create the smallest useful end-to-end path without committing to the full final RADcite UX.

## Goal

Add a minimal functional DOCX workflow in the desktop app:

- enter a local `.docx` path
- run Rust DOCX ingestion
- persist the analysed result in local SQLite
- display summary counts in the Svelte UI

## Non-Goals

- Full polished file picker UX.
- PDF ingestion.
- Reference-list parsing, APA validation, Crossref/OpenAlex lookup, or manual citation fixing.
- Multi-project management.
- Rich document detail/review screen.

## Design

Add local SQLite state to `DesktopState`. The app should create the local data directory, connect to `radsuite.sqlite3`, run migrations, and expose the database pool to desktop commands. Tests can keep using an in-memory SQLite pool.

Add a desktop command function in `radsuite-desktop`:

```rust
pub async fn analyse_docx_path(state: &DesktopState, request: AnalyseDocxRequest) -> Result<AnalyseDocxResponse, AnalyseDocxError>
```

The request should include a local file path and optional original filename. The command should:

1. Ensure a local demo project exists.
2. Run `radsuite_cite::ingest_docx`.
3. Persist the returned document, paragraphs, and citations through `SqliteCitationDocumentRepository`.
4. Return counts and identifiers suitable for UI feedback.

The Tauri wrapper should expose this as `analyse_docx_path` and convert errors to strings for the JavaScript bridge.

Update the Svelte shell with a compact RADcite panel:

- one text input for a `.docx` path
- an Analyse button
- loading, error, and result states
- result counts: paragraphs, citations, missing citation flags

## Error Handling

Return user-readable errors for:

- empty path
- non-DOCX path
- unreadable or malformed DOCX
- database/migration/persistence failures

The UI should show the error in the panel without crashing the rest of the shell.

## Testing

Add Rust desktop tests with generated minimal DOCX fixtures to verify:

- analysing a DOCX persists the document and returns summary counts
- empty path is rejected before ingestion

Verify:

- `cargo test -p radsuite-desktop`
- `npm run build` from `apps/desktop-ui`
- `cargo test --workspace`

Plan/spec reviewer subagent steps were not run because this Codex session only permits subagents when the user explicitly asks for delegation.
