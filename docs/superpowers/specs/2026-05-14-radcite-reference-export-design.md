# RADcite Reference Export Design

## Status

Approved via the standing Phase 2 instruction to continue with the next logical RADcite slice after the merged review queue.

## Context

RADsuite can now ingest DOCX files, store course references, link detected citations to references, suggest likely matches, and show review queue status. The left navigation still has an inactive `Exports` area, while the previous Python RADcite app had an `Export HTML` workflow for course references and module readings.

The Rust rebuild does not yet have modules or module readings. It does have local course references for the seeded RADcite project, so the smallest useful export is a course-reference HTML export.

## Goal

Make the RADcite `Exports` area produce a Moodle/AKO-ready HTML snippet from the current local course references.

## Non-Goals

- Do not add module readings or module/week selection yet.
- Do not write citations back into DOCX.
- Do not introduce server-side sync/export.
- Do not add APA validation gates before export.
- Do not add filesystem save dialogs in this slice.

## Backend Shape

Add a desktop command:

```rust
pub async fn export_course_references(
    state: &DesktopState,
    request: ExportCourseReferencesRequest,
) -> Result<CourseReferencesExport, CourseReferenceExportError>
```

The response should include:

- `filename`: a safe `.html` filename, for example `crju150-course-references.html`
- `content_type`: `text/html; charset=utf-8`
- `html`: the generated HTML snippet
- `reference_count`: number of exported references

The command should load the same local RADcite project used by the references workspace, list active `Reference` entries, and format each non-empty APA/citation/title value as an escaped HTML paragraph. The default export should wrap the paragraphs in the old RADcite Generico markers:

```html
<p>{GENERICO:type="references"}</p>
...
<p>{GENERICO:type="references_end"}</p>
```

If `for_ako_learn` is true, omit those Generico wrapper paragraphs.

## Frontend Shape

Add an active `RadciteExportsWorkspace.svelte` for the existing `exports` nav item.

The workspace should show:

- reference count available for export
- an `AKO | LEARN` checkbox that removes Generico tags
- a `Generate HTML` button
- a preview panel showing the generated HTML as text
- `Copy HTML` and `Download HTML` actions once export content exists

The download can be browser-side using a `Blob` and the filename returned by Rust. Copy can use `navigator.clipboard.writeText`, with an error notice if clipboard access fails.

## UX Direction

Keep this as a production utility surface, not a landing page. It should be visually aligned with the existing References and Documents workspaces: compact controls, restrained panels, and clear empty states. The right citation actions panel can remain visible for now.

## Testing

Rust:

- generated export includes escaped references and Generico wrappers by default
- AKO | LEARN mode omits Generico wrappers

Frontend:

- command helper sends the `for_ako_learn` flag to the Tauri command
- style contract requires the Exports workspace, export controls, and active exports route
- Svelte build/type-check

## Review Note

The normal brainstorming workflow asks for spec-review subagents. This Codex session only allows subagents when the user explicitly asks for them, so that review step is intentionally skipped.
