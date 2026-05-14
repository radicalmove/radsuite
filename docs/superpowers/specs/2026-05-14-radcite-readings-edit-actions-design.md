# RADcite Readings Edit Actions Design

Approved via the standing Phase 2 instruction to keep moving through the next logical RADcite slice.

## Context

RADsuite now supports local RADcite modules, manually entered module readings, and HTML export for module readings. The current Readings workspace is still one-way: users can add modules and readings, but they cannot correct mistakes or remove items from the Local DB workflow.

The data model already includes `archived_at` on `course_modules` and `reference_entries`, and existing list/load queries filter archived records out. That makes soft-removal the least disruptive path.

## Goal

Add practical edit/remove actions for the Readings workspace:

- edit a module title/code/order/description
- remove a module from active lists
- edit a reading category, lesson code, APA text, original text, URL, notes, student notes, and estimated reading time
- remove a reading from active lists and exports

## Non-Goals

- Do not add permanent hard delete.
- Do not add undo or restore archived records.
- Do not add bulk selection.
- Do not move readings between modules in this slice.
- Do not add automated extraction or reference lookup.

## Backend Shape

Extend repositories:

- `CourseModuleRepository`
  - `update_course_module(&CourseModule)`
  - `archive_course_module(ModuleId)`
- `ReferenceEntryRepository`
  - `load_reference_entry(ReferenceEntryId)`
  - `update_reference_entry(&ReferenceEntry)`
  - `archive_reference_entry(ReferenceEntryId)`

`archive_course_module` should also archive its module readings so removed modules do not leave invisible active readings behind. Repository list/load methods already hide archived modules and references.

Add desktop commands:

- `update_radcite_module`
- `archive_radcite_module`
- `update_module_reading`
- `archive_module_reading`

Validation should match existing add commands:

- module title is required
- update/archive module id must exist and be active
- reading id must exist, be active, and be a reading
- reading category must be `compulsory` or `optional`
- reading must keep either APA/reference text or original citation text

## Frontend Shape

Extend `RadciteReadingsWorkspace.svelte` with inline editing instead of modals:

- module cards show `Edit` and `Remove`
- clicking `Edit` populates the existing module form and changes submit text to `Update module`
- clicking `Cancel edit` returns the module form to add mode
- reading rows show `Edit` and `Remove`
- clicking `Edit` populates the existing reading form and changes submit text to `Update reading`
- remove actions use a confirmation before calling the archive command

`App.svelte` should refresh module/readings state after each action. If the selected module is removed, the app should select the first remaining module or clear the selection.

## Export Interaction

Archived readings disappear from module reading lists and module reading exports because export loads from the same active repository list. Archived modules cannot be exported because active module load returns missing.

## Testing

Rust:

- repository tests verify update/archive for modules and readings
- desktop command tests verify update/archive behaviour and validation
- module export should naturally exclude archived readings through existing repository filtering

Frontend:

- command helper tests verify the four new Tauri payloads
- style contract requires visible edit/remove controls and Tauri command hooks
- build/type-check catches Svelte prop wiring

Manual/browser:

- open Readings
- confirm inline edit/remove controls render without overlap
- confirm edit forms switch between add and update modes

## Review Note

The normal brainstorming workflow asks for spec-review subagents and a user review gate. This Codex session only allows subagents when the user explicitly asks for them, and the user has given standing approval to proceed with logical Phase 2 slices, so those review steps are intentionally skipped.
