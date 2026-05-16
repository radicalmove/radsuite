# RADcite Readings Import Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a review-before-save DOCX import workflow for RADcite module readings.

**Architecture:** Keep candidate extraction deterministic and side-effect free in `radsuite-cite`. Expose preview/save commands through `radsuite-desktop` and Tauri, then extend the existing Svelte Readings workspace with a DOCX preview panel that lets users select/edit candidates before saving to existing module-reading storage.

**Tech Stack:** Rust, quick-xml/zip DOCX parsing, SQLx/SQLite repositories, Tauri commands, Svelte 5, TypeScript, Vitest, CSS.

---

## File Structure

- Modify: `crates/radsuite-cite/src/docx.rs`
  - Add reading-candidate extraction types and heuristics.
- Modify: `crates/radsuite-cite/tests/docx_ingestion.rs`
  - Add extractor tests using minimal DOCX fixtures.
- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add preview/save request/response types and commands.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add command tests for preview no-side-effect behavior and save persistence.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose preview/save Tauri commands.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add reading import candidate/result types.
- Modify: `apps/desktop-ui/src/lib/readingCommands.ts`
  - Add `previewModuleReadingsImport` and `saveModuleReadingsImport`.
- Modify: `apps/desktop-ui/src/lib/readingCommands.test.ts`
  - Verify command payloads and trimming.
- Modify: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
  - Add DOCX import preview panel, editable candidates, and save selected action.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Wire import handlers, refresh readings, and clear export state on save.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add compact import-preview and candidate-row styling.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require visible import controls and command hooks.

## Task 1: Extract Reading Candidates From DOCX

- [ ] Add a failing test `docx_reading_import_extracts_compulsory_and_optional_candidates` in `crates/radsuite-cite/tests/docx_ingestion.rs`.
- [ ] Fixture should include paragraphs:
  - `Compulsory readings`
  - `1.2 Smith, J. (2024). Worked examples. https://example.com/worked`
  - `Optional readings`
  - `Taylor, R. (2023). Optional primer.`
  - a non-reference paragraph that should be ignored.
- [ ] Run `cargo test -p radsuite-cite docx_reading_import_extracts_compulsory_and_optional_candidates` and confirm it fails because the extractor API is missing.
- [ ] Add public structs in `crates/radsuite-cite/src/docx.rs`:
  - `DocxReadingExtractionRequest`
  - `ReadingImportCandidate`
- [ ] Add `extract_docx_reading_candidates(request)` that reuses existing DOCX paragraph extraction.
- [ ] Implement category, lesson-prefix, URL, APA-like reference, and de-duplication heuristics.
- [ ] Run the focused test and confirm it passes.
- [ ] Add and pass a second focused test for module/week heading hints if needed by implementation.
- [ ] Commit extractor work.

## Task 2: Desktop Preview And Save Commands

- [ ] Add failing desktop tests:
  - `module_readings_import_preview_extracts_candidates_without_persisting`
  - `module_readings_import_save_persists_selected_candidates`
  - `module_readings_import_save_validates_missing_module`
- [ ] Run `cargo test -p radsuite-desktop module_readings_import` and confirm command/types are missing.
- [ ] In `crates/radsuite-desktop/src/commands.rs`, add serializable types:
  - `PreviewModuleReadingsImportRequest`
  - `ModuleReadingImportCandidateSummary`
  - `SaveModuleReadingsImportRequest`
  - `SaveModuleReadingsImportCandidate`
- [ ] Add `ModuleReadingImportError` variants for empty path, DOCX errors, missing module, empty reading text, and invalid category.
- [ ] Implement `preview_module_readings_import` with no database writes.
- [ ] Implement `save_module_readings_import` by validating each selected candidate and inserting `ReferenceEntryType::Reading` rows through the existing repository.
- [ ] Return `Vec<ModuleReadingSummary>` from save.
- [ ] Run focused desktop tests and confirm they pass.
- [ ] Commit desktop command work.

## Task 3: Tauri And TypeScript Bridge

- [ ] Add failing Vitest cases for:
  - `previewModuleReadingsImport({ path, original_filename })`
  - `saveModuleReadingsImport({ candidates })`
- [ ] Run `npm test -- --run src/lib/readingCommands.test.ts` and confirm helpers are missing.
- [ ] Add TypeScript types in `apps/desktop-ui/src/types.ts` for `ModuleReadingImportCandidate`.
- [ ] Add helper functions in `apps/desktop-ui/src/lib/readingCommands.ts`, trimming editable fields and preserving `module_id`.
- [ ] Expose Tauri wrappers in `apps/desktop-ui/src-tauri/src/main.rs`.
- [ ] Run focused Vitest and confirm it passes.
- [ ] Commit bridge work.

## Task 4: Readings Workspace Import UI

- [ ] Update `apps/desktop-ui/scripts/verify-style-contract.mjs` first to require:
  - `Preview readings`
  - `Save selected readings`
  - `reading-import-panel`
  - `reading-import-candidate`
  - `previewModuleReadingsImport`
  - `saveModuleReadingsImport`
- [ ] Run `npm run test:style` and confirm it fails.
- [ ] Extend `RadciteReadingsWorkspace.svelte` props for preview/save callbacks.
- [ ] Add import state:
  - path
  - loading/error
  - editable candidate array
  - selected checkboxes
- [ ] Add DOCX `Choose DOCX` support using the existing dialog plugin pattern from `RadciteDocumentsWorkspace.svelte`.
- [ ] Render each candidate with checkbox, module selector, category selector, lesson code input, APA textarea, and URL input.
- [ ] Add `Save selected readings` action, disabled when no candidates are selected or modules are unavailable.
- [ ] Wire handlers in `App.svelte`; on save, clear module readings export and refresh selected module readings.
- [ ] Add compact CSS without card nesting.
- [ ] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.
- [ ] Commit UI work.

## Task 5: Full Verification And Publish

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `npm run test:style`.
- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build`.
- [ ] Browser/Tauri smoke-test:
  - open Readings
  - create/select a module
  - preview a DOCX with reading candidates
  - save one selected candidate
  - confirm it appears in module readings and export count updates
- [ ] Commit verification fixes if needed.
- [ ] Push branch.
- [ ] Open PR, wait for CI, merge if checks pass.

## Review Note

The normal planning workflow asks for a plan-review subagent. This session's active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
