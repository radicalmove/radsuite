# RADcite Usability Actions Design

## Goal

Make the merged RADcite desktop shell feel usable for day-to-day review by adding native DOCX selection and local citation review actions.

## Scope

This slice adds:

- A native `Choose DOCX` control beside the existing document path field.
- Local review actions in the right Citation Actions panel:
  - `Mark as resolved`
  - `Add citation manually`
  - `Verify citation`
- Session-level updates to paragraph state and summary counts after those actions.

This slice does not add:

- Source searching.
- Reference matching against readings.
- Writing citations back into the DOCX.
- Exporting a modified Word document.
- Cross-session persistence of manual review actions.

## Design

Use the Tauri v2 dialog plugin for the file picker. The frontend imports `open` from `@tauri-apps/plugin-dialog`, the Tauri app registers `tauri_plugin_dialog::init()`, and the app grants the dialog default permission through a capability file. The picker is limited to single `.docx` files and fills the existing path input; manual path entry remains available as a fallback.

Keep citation review actions local to the current analysis result in Svelte state. `App.svelte` owns the active `AnalyseDocxReviewResponse`; action callbacks update the selected paragraph and then recompute the summary from the updated paragraphs. This keeps the behavior fast and testable while avoiding premature database or DOCX writeback design.

The action semantics are:

- `Mark as resolved`: sets `needs_citation` to `false` for the selected paragraph.
- `Add citation manually`: asks for citation text in an inline form, appends it to the selected paragraph citations, marks it verified, and clears `needs_citation`.
- `Verify citation`: marks all currently detected citations on the selected paragraph as verified.

The UI should clearly show that these are review-session changes, not exported DOCX edits. Buttons should only enable when the selected paragraph supports the action.

## Testing

- Extend the style/markup contract to require the picker and action form elements.
- Add pure TypeScript tests for the review-state reducers so action behavior is independent of Svelte rendering.
- Run the desktop UI build and Rust workspace verification.

## Notes

The Tauri dialog plugin design follows the official Tauri v2 Dialog documentation. The docs state that JavaScript usage imports `open` from `@tauri-apps/plugin-dialog`, the Rust side initializes `tauri_plugin_dialog::init()`, and file dialogs return filesystem paths on Linux, Windows, and macOS.

The normal brainstorming workflow asks for a spec-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
