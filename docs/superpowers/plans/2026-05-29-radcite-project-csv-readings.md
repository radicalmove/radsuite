# RADcite Project Context And CSV Readings Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make local RADcite commands and UI use the selected course/project, and add CSV readings import preview for real course reading inventories.

**Architecture:** Add explicit local project commands and optional `project_id` fields to project-owned RADcite requests while preserving fallback CRJU150 behavior. Add a deterministic CSV reading candidate extractor in `radsuite-cite`, expose it through desktop/Tauri commands, then wire project state and CSV preview into the existing Svelte shell.

**Tech Stack:** Rust, SQLx/SQLite, serde, CSV parsing, Tauri commands, Svelte 5, TypeScript, Vitest.

---

## File Structure

- Modify: `Cargo.toml`
  - Add workspace `csv` dependency.
- Modify: `crates/radsuite-cite/Cargo.toml`
  - Consume workspace `csv`.
- Modify: `crates/radsuite-cite/src/lib.rs`
  - Export CSV reading import types/functions.
- Modify: `crates/radsuite-cite/src/readings_csv.rs`
  - New CSV extractor.
- Modify: `crates/radsuite-cite/tests/readings_csv_import.rs`
  - New extractor tests.
- Modify: `crates/radsuite-db/src/repositories.rs`
  - Add local project listing support.
- Modify: `crates/radsuite-db/tests/repository_roundtrip.rs`
  - Add project listing test.
- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add project commands, project-aware request fields, CSV preview command.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add project isolation and CSV import command tests.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose new Tauri commands and updated request types.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add project summary and CSV import types as needed.
- Create: `apps/desktop-ui/src/lib/projectCommands.ts`
  - Project list/create helpers.
- Modify: `apps/desktop-ui/src/lib/readingCommands.ts`
  - Add project IDs and CSV preview helper.
- Modify: `apps/desktop-ui/src/lib/referenceCommands.ts`
  - Add project IDs to list/add helpers.
- Modify: `apps/desktop-ui/src/lib/exportCommands.ts`
  - Project-aware course reference export.
- Modify: `apps/desktop-ui/src/lib/*test.ts`
  - Update/add command helper tests.
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
  - Data-backed projects and compact create form.
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
  - Pass selected project into DOCX analysis.
- Modify: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
  - Add DOCX/CSV import source selector.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Load projects, pass selected project ID through RADcite workflows.
- Modify: `apps/desktop-ui/src/styles.css`
  - Compact sidebar form and import-source styling.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require project commands and CSV import hooks.

## Task 1: CSV Reading Candidate Extractor

- [x] Add failing `radsuite-cite` tests for real-shaped `course_readings.csv` input.
- [x] Run `cargo test -p radsuite-cite readings_csv` and confirm missing API failure.
- [x] Add workspace `csv` dependency and `radsuite-cite` dependency.
- [x] Implement `CsvReadingExtractionRequest`, `CsvReadingImportError`, and `extract_csv_reading_candidates`.
- [x] Map `citation`, `week`, `section_title`, `section_seq`, and optional `reading_category` headers into `ReadingImportCandidate`.
- [x] Run focused tests and confirm pass.
- [x] Commit extractor work.

## Task 2: Local Project Commands And Project-Scoped Backend

- [x] Add failing desktop/db tests for local project listing, creating projects, project-scoped DOCX analysis, reference/module isolation, and project-code export filenames.
- [x] Run focused tests and confirm expected failures.
- [x] Add repository support for listing all local projects.
- [x] Add desktop request/summary types:
  - `RadciteProjectSummary`
  - `CreateRadciteProjectRequest`
  - `ListSavedReviewsRequest`
  - `ListCourseReferencesRequest`
  - `ListRadciteModulesRequest`
- [x] Add optional `project_id` to existing project-owned requests.
- [x] Add helpers to load supplied project or fallback CRJU150 project.
- [x] Update relevant commands to use explicit project context.
- [x] Run focused desktop/db tests and confirm pass.
- [x] Commit project-scoped backend work.

## Task 3: CSV Preview Desktop/Tauri Bridge

- [x] Add failing desktop test for `preview_module_readings_csv_import` and saving candidates into a selected module.
- [x] Add failing Vitest tests for project helper payloads and CSV preview payloads.
- [x] Expose `preview_module_readings_csv_import` in desktop and Tauri.
- [x] Add/update TypeScript helpers:
  - `listRadciteProjects`
  - `createRadciteProject`
  - `previewModuleReadingsCsvImport`
  - project-aware reading/reference/export calls.
- [x] Run focused Rust and Vitest tests and confirm pass.
- [x] Commit bridge work.

## Task 4: Svelte Project Context And CSV Import UI

- [x] Update style contract first to require project command hooks and CSV import text/hooks.
- [x] Run `npm run test:style` and confirm failure.
- [x] Make `ProjectSidebar.svelte` render loaded projects and compact create-project controls.
- [x] Make `App.svelte` load/create/select projects and clear project-specific state on project switch.
- [x] Pass selected `project_id` to document analysis, saved review listing, reference/module list/add/export commands.
- [x] Add DOCX/CSV source selector to `RadciteReadingsWorkspace.svelte` and call the right preview callback.
- [x] Add restrained CSS for the project form and import source selector.
- [x] Run `npm run test:style`, focused Vitest, and `npm run build`.
- [x] Commit UI work.

## Task 5: Functional Smoke And Publish

- [x] Run `cargo fmt --all`.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `npm run test:style`.
- [x] Run `npm test -- --run`.
- [x] Run `npm run build`.
- [x] Run a temporary functional smoke with:
  - create/select CRJU201 project;
  - analyse a real CRJU201/COMS432 DOCX;
  - preview `/Users/rcd58/course-output-system/Courses/CRJU201/Extracted/Inventories/course_readings.csv`;
  - save selected readings;
  - confirm exports use `crju201-*` filenames.
- [x] Commit verification docs/fixes if needed.
- [ ] Push branch and open PR.
- [ ] Wait for CI and merge if checks pass.

## Review Note

Spec/plan review subagents are skipped in this session because no explicit subagent request has been made; review is handled by direct verification and focused tests.
