# RADcite Managed Document Sources Design

## Status

Approved under the standing RADsuite migration instruction to continue with the next logical RADcite parity slice.

## Context

RADcite currently persists the extracted paragraphs, citations, and review actions for an imported DOCX or PDF, but it does not persist the source file itself. The path selected in the Documents workspace exists only in the Svelte session. After restarting RADsuite, a saved review can be reopened, but the same document cannot be reused automatically by Module Readings without choosing the source again.

## Goal

Make an imported document project-scoped and reusable across RADcite workspaces by copying the source into RADsuite's local application data, retaining its managed path with the saved review, and restoring that source when the review is reopened.

## Recommended Design

### Managed source storage

- When DOCX or PDF analysis begins, resolve the selected project and create a document-scoped destination below the app data directory:
  `documents/<project-id>/<document-id>-<safe-original-filename>`.
- Copy the source file before persisting the analysis. The managed copy becomes the source used by subsequent readings extraction.
- Store the managed path as an optional `source_path` on `Document`. Existing records from before this migration remain valid with `NULL` and continue to load as saved reviews.
- Keep the original filename separately for display and export names. Do not expose internal database or asset terminology in the UI.
- Do not delete managed source files when a document is archived; archive and restore must preserve the source for future reuse.

### Persistence and command contracts

- Add a forward-only SQLite migration adding nullable `documents.source_path`.
- Extend document repository insertion, loading, and summaries to carry the path.
- Add `source_path` and `source_file_type` to the shared saved-review response and summary contracts. The file type is needed to decide whether the source can feed the DOCX readings importer.
- Keep the existing analysis command names and request shapes unchanged.
- If copying fails, return a clear source-storage error and do not insert a partial document review.

### User flow

- After a new analysis, the Documents workspace keeps the user-selected path in the input while the application remembers the managed path internally for readings extraction.
- Opening a saved review restores its managed source. A DOCX saved review gets a `Use for readings` action that opens Module Readings and previews the document automatically using the managed copy.
- PDF saved reviews remain reusable in Documents but do not show the DOCX readings shortcut.
- Legacy saved reviews without a managed source remain openable. They simply do not offer automatic readings reuse until the user chooses the source again.

## Error Handling

- Empty, unsupported, or unreadable source files continue to use the existing ingestion errors.
- A copy failure identifies that RADsuite could not keep a local copy and leaves the database unchanged.
- A missing managed file is reported only when the user requests a follow-on extraction; opening the saved review itself remains available.
- Existing database files upgrade with `source_path = NULL` and retain all child paragraphs, citations, readings, and references.

## Testing

- Unit tests cover safe managed filenames, project/document destination layout, successful copy, and copy failure.
- Migration tests verify an existing version-3 schema gains the nullable column without altering saved rows.
- Repository tests verify source paths round-trip and legacy `NULL` values remain valid.
- Desktop contract tests verify DOCX/PDF analysis persists a managed source, saved-review responses expose its path and type, and a failed copy does not create a review.
- Frontend command/types and style contracts verify the saved-review readings action and legacy empty-source behavior.
- Full Rust tests, strict clippy, frontend tests/type-check/build, browser verification, and a packaged desktop build remain required.

## Scope Boundary

This slice does not implement permanent deletion, cloud upload, cross-device asset sync, hashing/deduplication, or a new document-management screen. Those can build on the managed source boundary later without changing the Documents or Readings user flow.
