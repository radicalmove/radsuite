# RADcite Module Readings Export Design

Approved via the standing Phase 2 instruction to continue with the next logical RADcite slice after the readings foundation.

## Context

RADsuite now has local RADcite modules and manually entered readings. The old Python RADcite app exported module readings as Moodle/AKO-ready HTML with compulsory/optional sections, lesson-code prefixes, Generico wrappers, optional AKO cleanup, reading notes, and estimated reading time.

The Rust rebuild can already export course references, but the Exports workspace only handles course-wide references. Module readings can now be exported from real Local DB records instead of from placeholder state.

## Goal

Add module-readings export to the RADcite Exports area:

- select a module
- generate HTML from that module's stored readings
- preserve compulsory and optional headings
- include lesson codes, reading URLs, notes, and estimated reading time
- support normal Moodle output with Generico wrappers
- support AKO | LEARN output without Generico wrappers and with hanging-indent reading paragraphs

## Non-Goals

- Do not auto-extract readings from uploaded documents.
- Do not add editing/deleting readings.
- Do not add reference validation or lookup during export.
- Do not implement full OpenAthens stable-link generation without a stored stable URL field.
- Do not add filesystem save dialogs; keep browser-side copy/download for now.

## Backend Shape

Add a desktop command:

```rust
pub async fn export_module_readings(
    state: &DesktopState,
    request: ExportModuleReadingsRequest,
) -> Result<ModuleReadingsExport, ModuleReadingExportError>
```

The request includes:

- `module_id`
- `for_ako_learn`

The response includes:

- `filename`: safe `.html` filename using the project and module label
- `content_type`: `text/html; charset=utf-8`
- `html`: generated HTML snippet
- `module_id`
- `reading_count`

The command should load the module, then list `ReferenceEntryType::Reading` entries scoped to the module. Missing modules should return a user-facing error. The existing repository already sorts module readings by category, lesson code, display order, text, and id, so the export should preserve repository order.

## HTML Formatting

For normal Moodle output:

```html
<p>These are the readings located in the module content, provided here for your convenience/change text.</p>
<p>{GENERICO:type="references"}</p>
<h4>Compulsory readings</h4>
<p><span style="font-size: 0.9375rem;"><strong>1.1&nbsp;</strong>Escaped reading text <a href="...">...</a></span></p>
<p>{GENERICO:type="references_end"}</p>
```

Rules:

- empty modules should return the old empty-state sentence: `No readings were detected for this module.`
- compulsory readings render before optional readings
- only render a section heading when that category has readings
- lesson codes render as bold prefixes with a non-breaking space
- reading text prefers APA citation, then citation text, then title, then `Reference pending.`
- unsafe text must be HTML-escaped
- if a stored `url` exists and is not already visible in the reading text, append it as a new-tab link
- if reading notes or estimated reading time are present, close Generico before those paragraphs, render metadata with the old left margin, then reopen Generico only if more readings follow

For `for_ako_learn = true`:

- remove all Generico wrappers
- apply `style="margin-left: 64px; text-indent: -64px;"` to reading paragraphs
- keep headings, links, notes, and estimated time

## Frontend Shape

Extend the existing `RadciteExportsWorkspace.svelte` instead of creating a separate page.

The workspace should include two export modes:

- `Course references`
- `Module readings`

For module readings, show:

- module selector using the Local DB modules already loaded for the Readings workspace
- selected-module reading count summary
- `AKO | LEARN` checkbox shared with the export mode
- `Generate HTML`, `Copy HTML`, and `Download HTML`
- preview text area/panel using the same copy/download mechanics as course references

`App.svelte` should refresh both course references and RADcite modules when the Exports area opens. The module-readings export result should be stored separately from the course-reference result so switching modes does not confuse counts or filenames.

## Testing

Rust:

- module readings export includes Generico wrappers, compulsory/optional headings, lesson-code prefixes, escaped text, URL links, and metadata placement
- AKO mode strips Generico and applies hanging indent
- missing module id returns a missing-module error

Frontend:

- command helper invokes `export_module_readings` with `module_id` and `for_ako_learn`
- Exports workspace exposes module readings mode and module selector controls
- style contract requires the new command bridge and UI hooks

Manual/browser:

- open Exports in the Vite app
- verify the two export modes are visible
- verify Module readings mode has a module selector/empty state without layout overlap

## Review Note

The normal brainstorming and planning workflows ask for review subagents. This Codex session only allows subagents when the user explicitly asks for them, so those review steps are intentionally skipped.
