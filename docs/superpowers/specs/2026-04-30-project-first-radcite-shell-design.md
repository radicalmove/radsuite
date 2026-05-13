# Project-First RADcite Shell Design

## Status

Approved in conversation on 2026-04-30 as the next UI/UX direction after merging the first functional DOCX workflow.

## Context

RADsuite now has a working Svelte/Tauri desktop app that can send a DOCX path into the Rust RADcite pipeline, persist the analysis locally, and return summary counts. That slice proved the technical path, but the UI is intentionally preliminary.

The old Python RADcite app at `/Users/rcd58/citation-checker` provides the workflow model:

- choose or create a course/project
- upload or select a document
- review paragraphs and citation status
- filter by citation state
- use a contextual citation action panel
- later manage course references, module readings, exports, and related tools

The old UI also carries complexity that should not be copied directly: many top-bar controls, deep modal flows, and mixed project/document/reference controls competing for attention.

## Product Model

RADsuite should be project-first.

A project is the top-level workspace. In current teaching use, a project usually maps to a course, but the term stays broader so RADsuite can later support training packages, client jobs, programmes, or resource collections without changing the app model.

Projects can contain:

- modules or weeks
- source documents
- RADcite references and readings
- RADcite exports
- future RADcast and RADTTS inputs/outputs

Course-specific language remains appropriate inside RADcite where it clarifies intent, for example "course references" and "module readings".

## Goal

Replace the placeholder desktop shell with the first real RADsuite working shell:

- persistent project sidebar
- project-first navigation grouped by tool
- RADcite Documents workspace as the first active tool area
- document import/review surface using the existing Rust DOCX command
- summary chips for document citation status
- paragraph review list
- contextual right-side citation actions placeholder

This slice should establish the durable navigation and layout pattern for later RADcite, RADcast, and RADTTS work.

## Non-Goals

- Full course/reference/module CRUD parity with the Python app.
- Crossref/OpenAlex search.
- Manual citation linking.
- Course reference management.
- Module readings management.
- Export formatting.
- Sync, sharing, auth, or admin workflows.
- Deep sidebar navigation for individual modules, documents, paragraphs, or citations.

## UX Direction

Use a three-zone desktop layout:

1. Left sidebar: project and tool navigation.
2. Main workspace: active tool surface.
3. Right panel: contextual actions for the current selection.

The sidebar should use two useful levels only:

```text
Project > Tool > Area
```

Avoid pushing deeper entities into the sidebar. Modules, documents, paragraphs, citations, readings, and references belong in the main workspace for the active area.

Recommended sidebar structure:

```text
Projects

v CRJU150 - Introduction to Criminal Justice
  RADcite
    Documents
    References
    Readings
    Exports
  RADcast
  RADTTS

v EDUC203 - Learning Design
  RADcite
    Documents
    References
    Readings
```

For this slice, only RADcite Documents needs to be active. References, Readings, Exports, RADcast, and RADTTS can appear as inactive or "coming later" destinations if that helps orient the user.

## RADcite Documents Workspace

The main RADcite Documents workspace should replace the raw path-based placeholder with an operational review desk.

Initial state:

- selected demo/local project visible in the sidebar
- document import area in the main workspace
- brief empty state inviting the user to import a DOCX
- no modal required for the first slice

After analysis:

- document title and original filename
- summary chips:
  - total paragraphs
  - total citations
  - paragraphs with citations
  - paragraphs needing citations
- filter controls using the summary chips
- paragraph list ordered by document order
- each paragraph row shows:
  - page/table indicator when available
  - text preview
  - citation badges
  - needs-citation status
  - selected state

Right panel:

- default state: "Select a paragraph"
- selected paragraph state:
  - full paragraph text
  - detected citations
  - whether RADcite thinks it needs citations
  - placeholder action slots for future search, verify, ignore, and manual citation actions

## Data And Command Shape

The existing `analyse_docx_path` command returns only summary counts. The new workspace needs document detail.

Add a Rust desktop command that returns the persisted analysed document with paragraph and citation details, for example:

```rust
pub async fn analyse_docx_for_review(
    state: &DesktopState,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxReviewResponse, AnalyseDocxError>
```

The response should include:

- project id/title
- document id/original filename
- summary counts
- ordered paragraphs
- citations per paragraph

This command can reuse the existing ingestion and persistence logic. The old summary command may remain for compatibility during the transition, but the Svelte UI should use the richer review response.

## State Model

For this slice, Svelte can use local component state for:

- selected project
- selected tool area
- selected document
- analysis result
- active filter
- selected paragraph
- loading/error states

Do not introduce a global state library yet. Svelte stores can be added later when cross-tool project state becomes shared by RADcite, RADcast, and RADTTS.

Seed the UI with one local/demo project if full project CRUD is not ready. The important thing is that the layout and data shape already match the intended project-first model.

## Error Handling

Show command errors in the RADcite Documents workspace without breaking the sidebar or global shell.

Expected errors:

- no file selected or empty path
- unsupported file type
- unreadable file
- malformed DOCX
- database or migration failure

The right panel should remain stable during errors and should not show stale selected paragraph details from a failed analysis.

## Testing

Rust:

- test the new review command returns ordered paragraphs and citations
- test summary counts still match paragraph/citation detail
- test empty path handling

Frontend:

- `svelte-check` should pass with no diagnostics
- build should pass
- UI state should render empty, loading, error, and analysed-result states

Manual functional test:

- run `cargo tauri dev`
- import a real DOCX
- confirm the sidebar remains stable
- confirm summary chips match visible paragraph status
- select paragraphs and confirm right panel updates

## Implementation Notes

Keep the first implementation scoped. The left sidebar can be real UI before the full project database workflow is complete.

Do not recreate the old Python app's top bar density. Global controls should stay minimal: app name, sync/status, help/settings if needed. Most work should happen inside the selected project/tool area.

Plan/spec reviewer subagent steps were not run because this Codex session only permits subagents when the user explicitly asks for delegation.
