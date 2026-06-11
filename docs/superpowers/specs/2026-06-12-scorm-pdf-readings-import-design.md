# SCORM PDF Readings Import Design

Approved in chat after clarifying that the workflow is for listing module readings, not RADcite citation-review documents.

## Context

RADsuite already supports module readings stored against local RADcite course modules. Readings can be added manually, imported from DOCX, imported from CSV, reviewed before saving, deduplicated on save, and exported as module readings HTML.

The current importer does not handle SCORM PDF outputs. The user has two expected source patterns:

- Microlearning SCORMs: multiple PDFs per module, usually one PDF per lesson or microlearning.
- Course SCORMs: one or a few larger PDFs that may contain multiple modules, lessons, or sections.

The importer must support both without forcing the user to manually create every module or assign every detected reading before previewing.

## Goal

Add a batch SCORM PDF readings import workflow that extracts reading candidates from multiple PDFs, previews them in the existing Module readings review UI, and saves selected readings through the existing Local DB path.

## Non-Goals

- Do not use this workflow for RADcite citation-review paragraph analysis.
- Do not change the stored module readings schema.
- Do not silently save extracted readings without a preview step.
- Do not require perfect PDF layout reconstruction.
- Do not build a separate readings-list database or export format.

## User Workflow

1. User opens `RADcite > Readings`.
2. User selects `PDF` as an import source.
3. User chooses multiple PDFs or a folder containing SCORM PDF outputs.
4. RADsuite extracts text from each PDF and detects reading candidates.
5. RADsuite previews all candidates in one editable list.
6. RADsuite shows source PDF, inferred module, inferred lesson code, category, APA text, URL, and import selection.
7. User edits uncertain rows, deselects unwanted rows, and saves once.
8. Existing module readings refresh and export as usual.

## Detection Rules

The PDF importer should reuse the existing DOCX candidate model where possible:

- Category labels map `required`, `compulsory`, and similar wording to the stored compulsory category.
- UI displays this category as `Required`.
- Optional and recommended labels map to optional.
- If a duplicate reading appears as both optional and required, required wins.
- APA-like references with author/year patterns become candidates.
- Standalone URLs inside a reading section become candidates when no fuller reference is available.
- Bibliography/reference-list sections are ignored unless they are clearly part of a readings section.

## Module And Lesson Inference

Microlearning PDFs should be inferred primarily from filenames and parent folders:

- `Module 6 Microlearning 3.pdf` should infer module `Module 6` and lesson `Microlearning 3`.
- `Week 4 Lesson 2.pdf` should infer module `Week 4` and lesson `Lesson 2`.

Course SCORM PDFs should also scan PDF headings:

- `Module N`, `Week N`, and similar headings update the current module context.
- Lesson or microlearning headings update the current lesson context.
- If module inference is uncertain, candidates remain unassigned in the preview rather than being saved to a guessed module.

## Architecture

Add PDF extraction inside `radsuite-cite`, alongside existing DOCX and CSV reading import code. The first implementation should expose a side-effect-free batch extraction API returning `ReadingImportCandidate` values plus source metadata where needed by the UI.

Expose a desktop command that accepts multiple PDF paths and returns preview candidates. The command should not write to SQLite. Saving should continue to use the existing `save_module_readings_import` command so deduping and module-reading persistence stay centralized.

Extend the Svelte Readings workspace by adding `PDF` to the existing DOCX/CSV import selector, a multi-file/folder picker, and source-aware preview status text.

## Error Handling

- Empty PDF selection returns a clear "choose PDF files" error.
- Unsupported file extensions are skipped or rejected before extraction.
- Files that cannot be parsed should produce a per-file failure in the preview/status instead of aborting the entire batch where possible.
- If no readings are detected, show a normal empty preview message.
- Unassigned candidates must be editable before save.

## Testing

Use focused tests before implementation:

- `radsuite-cite` tests for extracting candidates from generated or fixture PDF text.
- Tests for filename inference across microlearning and course-style names.
- Desktop command tests for multiple PDF paths and empty path validation.
- TypeScript command tests for trimming and invoking the new preview command.
- Svelte or workflow tests for PDF source selection and save-through behaviour.

## First Slice

Build preview-only batch PDF import into the existing Module readings import flow:

- Add a PDF extraction API.
- Add a desktop preview command.
- Add UI command wiring and import-source selector support.
- Reuse the existing save command and preview table.

This slice is enough for the user to load SCORM PDFs, review readings, save them, and export module readings using the current export workflow.
