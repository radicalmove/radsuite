# RADcite Readings Foundation Design

Approved via the standing Phase 2 instruction to continue with the next logical RADcite slice after course-reference export.

## Context

RADsuite can now ingest DOCX files, review citations, manage course references, suggest citation/reference matches, and export course references. The left navigation still shows `Readings` as unavailable.

The old Python RADcite app models readings as module-scoped reference entries. A course has modules or weeks, and each module can have compulsory and optional readings with lesson codes, APA/reference text, URLs, notes, and estimated reading time. The old app also auto-extracts readings from uploaded documents and exports Moodle-ready module readings HTML, but those workflows depend on having module/readings storage first.

The Rust rebuild already has `ReferenceEntryType::Reading`, but it does not yet have module/week records or a way to attach readings to modules.

## Goal

Add the first real local `Readings` workflow:

- persist module/week records for the local RADcite project
- persist manually entered readings against a selected module/week
- enable the `Readings` navigation item
- show a functional Readings workspace for creating modules and adding/listing readings

This should make the app structurally ready for the next slice: module-readings export.

## Non-Goals

- Automatic extraction of readings from DOCX/PDF content.
- Crossref/OpenAlex lookup or auto-fill.
- Editing, merging, deleting, or archiving readings.
- Document-to-module assignment for analysed DOCX files.
- Module-readings HTML export.
- Full project/course CRUD.

## Recommended Approach

Use a small real foundation instead of a placeholder or a full parity port.

The alternatives considered were:

- Placeholder UI only: fast, but it does not unlock exports or functional testing.
- Full old-RADcite parity: useful eventually, but too much surface area for one slice.
- Bounded foundation: enough data model and UI to create module readings manually, without taking on extraction/export/search complexity yet.

The bounded foundation is the right next step because it creates durable data the later export and extraction slices can use.

## Data Model

Add a `CourseModule` domain model:

- `id`
- `project_id`
- `code`
- `title`
- `order_index`
- `description`
- `archived_at`
- timestamps

Add a `ModuleId` UUID wrapper.

Extend `ReferenceEntry` with optional reading/module metadata:

- `module_id`
- `lesson_code`
- `reading_category`: `compulsory` or `optional`
- `reading_notes`
- `estimated_reading_time`

Keep `ReferenceEntryType::Reading` as the broad type. The category is separate because the old app distinguishes compulsory and optional readings within the reading workflow.

SQLite gets a new migration rather than changing the existing foundation migration:

- `course_modules` table
- project/order index
- nullable reading metadata columns on `reference_entries`

## Rust Repositories

Add a `CourseModuleRepository` beside the existing project/document/reference repositories:

- insert module
- list modules for project
- load module by id

Extend `ReferenceEntryRepository`:

- continue listing by project and reference type for course references
- add module-scoped reading listing
- persist and hydrate the new reading metadata fields

Sorting should be stable and simple:

- modules by `order_index`, title, id
- readings by category, lesson code, display order, APA/citation text, id

## Desktop Commands

Add Tauri-facing command logic in `radsuite-desktop`:

- `list_radcite_modules`
- `add_radcite_module`
- `list_module_readings`
- `add_module_reading`

The commands should use the same local RADcite project helper as Documents, References, and Exports.

Validation rules:

- module title is required
- reading module id must exist
- reading category must be `compulsory` or `optional`
- reading must include either APA/reference text or original citation text

The Readings workspace can seed a default `Module 1` only if we need it for first-run usability, but it should not hide the module concept. The UI should still show module creation clearly.

## Svelte UI

Enable `Readings` in `ProjectSidebar.svelte`.

Add `RadciteReadingsWorkspace.svelte`:

- workspace header with selected module and reading counts
- compact module selector/list
- add module form
- add reading form
- grouped reading list for compulsory and optional readings
- refresh action and error notices

Add a small `readingCommands.ts` wrapper for the Tauri calls and matching Vitest coverage.

Keep the visual style aligned with the current RADcite shell:

- compact cards and panels
- red primary action
- green/neutral status chips where useful
- no large marketing-style layout

## Error Handling

Backend commands should return plain user-facing errors for empty titles, missing modules, invalid categories, and empty reading text.

The UI should keep existing readings visible if a refresh or add action fails, and show the error in the workspace notice pattern already used by References and Exports.

## Testing

Rust:

- domain serialization for `CourseModule` and reading metadata
- repository roundtrip for modules and module readings
- desktop command tests for adding/listing modules and readings
- validation tests for empty module title, invalid category, and missing reading text

Frontend:

- command wrapper tests for list/add module and list/add reading
- style contract includes active Readings route, workspace component, and enabled sidebar item
- build/test verification after implementation

Manual/browser:

- open the Svelte app
- select Readings
- add a module
- add one compulsory and one optional reading
- verify they appear under the selected module

## Follow-On Slice

After this lands, the next logical slice is module-readings export:

- select a module in Exports
- generate Moodle/AKO-ready readings HTML
- preserve compulsory/optional headings
- apply OpenAthens/DOI link handling
