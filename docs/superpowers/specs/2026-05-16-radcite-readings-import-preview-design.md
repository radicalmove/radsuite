# RADcite Readings Import Preview Design

Approved on 2026-05-16 as the next RADcite Phase 2 slice after manual module readings, edit/remove actions, and module readings export.

## Context

RADsuite now has a local RADcite Readings workspace with modules, manually entered readings, edit/remove actions, and Moodle/AKO HTML export. The remaining parity gap with the old Python RADcite app is that readings are still manual: users cannot point RADsuite at module content and import the reading list candidates.

The old Python app extracted readings from document paragraphs using conservative heuristics in `/Users/rcd58/citation-checker/app/api/routes.py`:

- detect headings such as `Compulsory reading`, `Required reading`, `Optional reading`, and `Recommended reading`
- identify APA-like reference paragraphs by year and author-style opening patterns
- infer lesson markers such as `2.3` or `Module 2 lesson 3`
- split leading lesson prefixes from reference text
- optionally split multi-module documents when headings such as `Module 1` or `Week 2` are present

The Rust rebuild already has DOCX ingestion in `radsuite-cite`, Tauri dialog support, and persistent module/readings repositories.

## Goal

Add a review-before-save import path for module readings:

1. User chooses a DOCX in the Readings workspace.
2. RADsuite extracts likely reading candidates without writing to the Local DB.
3. The UI shows candidates for review and selection.
4. User saves selected candidates into the existing module readings storage.

This slice should reduce manual entry while keeping the user in control.

## Non-Goals

- No AI/LLM extraction.
- No Crossref/OpenAlex/reference lookup.
- No automatic database writes during analysis.
- No PDF import.
- No duplicate-merging UI beyond simple extraction de-duplication.
- No bulk deletion or undo beyond existing soft-remove actions.

## Recommended Approach

Build **DOCX extraction with review-before-save**.

The extractor should be deterministic and conservative. A false negative is acceptable because users can still add readings manually. A false positive is more harmful, so candidates should require obvious APA-like signals and user confirmation before saving.

## Backend Design

Add a focused readings extraction module in `radsuite-cite`, separate from citation review:

```rust
pub struct ReadingImportCandidate {
    pub module_order: Option<i32>,
    pub module_title: Option<String>,
    pub reading_category: ReadingCategory,
    pub lesson_code: Option<String>,
    pub apa_citation: String,
    pub citation_text: Option<String>,
    pub url: Option<String>,
}

pub fn extract_docx_reading_candidates(
    request: DocxReadingExtractionRequest,
) -> Result<Vec<ReadingImportCandidate>, DocxIngestionError>
```

The extractor can reuse DOCX package parsing logic from `docx.rs`. If the current helpers are private, the implementation should either expose a small plain-paragraph extraction function or keep the readings extractor in the same module to avoid duplicating XML parsing.

Heuristics:

- Track current category, defaulting to compulsory.
- Change category when a paragraph contains compulsory/required/optional/recommended reading language.
- Detect module/week headings only when clearly expressed as `Module N` or `Week N`; use those as candidate grouping hints.
- Treat a paragraph as a candidate only when it has a year and an author-like opening.
- Remove leading bullets and lesson prefixes.
- Extract the first URL when present.
- De-duplicate by category, lesson code, and cleaned APA text.

## Desktop Command Design

Expose two desktop-level commands:

- `preview_module_readings_import`
  - input: DOCX path, optional original filename
  - output: candidate list grouped enough for UI display
  - side effects: none

- `save_module_readings_import`
  - input: selected candidates with explicit target `module_id`, plus editable reading fields
  - output: saved `ModuleReadingSummary[]`
  - side effects: creates readings only

For the first implementation, module creation should stay explicit in the Readings workspace. If extracted candidates include `Module 1` hints but no matching module exists, the UI should ask the user to choose an existing module rather than silently creating modules. This keeps the import safe and avoids surprising course structure changes.

## UI Design

Extend `RadciteReadingsWorkspace.svelte` with an import panel above the manual add-reading form:

- file path input and `Choose DOCX`
- `Preview readings` button
- candidate summary: total candidates, compulsory count, optional count
- candidate list with:
  - checkbox
  - target module selector, defaulting to the selected module
  - category selector
  - lesson code input
  - editable APA citation textarea
  - URL input
- `Save selected readings` button

The existing manual add/edit forms remain available. The preview panel should not feel like a modal because users may need to compare against the current module readings list.

Error handling:

- empty path: show import error
- non-DOCX or malformed DOCX: return existing DOCX ingestion errors
- no candidates: show a calm empty state, not a failure
- save with no selected candidates: no-op with inline guidance
- save candidate without target module: validation error

## Data Flow

1. User opens Readings.
2. App loads modules/readings as it does now.
3. User previews DOCX candidates.
4. Tauri invokes `preview_module_readings_import`.
5. Frontend stores candidates locally.
6. User edits/selects candidates.
7. Frontend invokes `save_module_readings_import`.
8. App refreshes modules/readings and clears stale module readings export state.

## Testing

Use TDD for each layer.

Rust:

- extractor identifies compulsory and optional readings from DOCX paragraphs
- extractor infers lesson prefixes and first URL
- extractor ignores reading headings and non-reference paragraphs
- desktop preview command has no persistence side effects
- desktop save command persists selected candidates into existing module readings
- validation covers empty path, missing module, empty selected list, and empty reading text

TypeScript/Svelte:

- command wrappers send the expected Tauri payloads
- style contract requires import controls and save hooks
- production build catches Svelte type issues

Manual/browser smoke:

- open Readings
- choose/enter a DOCX path
- preview candidates
- save one candidate into an existing module
- verify it appears in the readings list and module readings export count updates

## Review Note

The normal brainstorming workflow asks for a spec-review subagent. This session's active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
