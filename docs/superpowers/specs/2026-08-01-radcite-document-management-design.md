# RADcite Document Management Design

## Goal

Bring the most useful saved-document metadata controls from the original RADcite app into the Rust desktop app without changing the underlying imported file or adding a second document model.

## Scope

The Documents workspace will allow a user to edit an active saved document's:

- display name, stored in the existing document notes field;
- document number, as a positive integer or blank;
- document type: Content, RISE, or Other;
- reference-output setting: include or exclude this document from reference output.

The saved-review list will show the effective display name while retaining the original filename as secondary information. Archive and restore remain the lifecycle controls.

Module assignment is intentionally out of scope for this slice. The Rust schema currently models documents at project level, while module readings are separate records. Adding a document-to-module relationship would change import and export semantics and should be designed as its own feature.

## User Flow

1. A user opens Documents and sees the saved review list.
2. They select Edit on a saved document.
3. An inline editor opens with the current metadata.
4. Save validates the document number, persists the metadata locally, refreshes the row, and closes the editor.
5. Cancel restores the unchanged row.
6. Persistence or validation failures remain in the Documents workspace and leave the editor open so the user can correct the issue.

The original filename and source path are read-only. Editing a display name changes how the document is presented in RADcite, not the source file or generated export text on disk.

## Architecture

`CitationDocumentRepository` will gain an `update_document_metadata` operation. The repository update will write only `notes`, `doc_variant`, `doc_number`, `exclude_from_references`, and `updated_at`; identity, source, file, project, and lifecycle columns remain immutable. The desktop command `update_radcite_document` accepts `UpdateRadciteDocumentRequest` with `project_id: ProjectId | null`, `document_id: DocumentId`, `display_name: string`, `doc_number: number | null`, `doc_variant: content | rise | other`, and `exclude_from_references: boolean`, and returns the refreshed `SavedRadciteReviewSummary`. It will enforce the selected-project boundary and active-document requirement, normalize the display name, validate the document number, apply the requested enum and exclusion values, then persist through the repository. The Tauri bridge registers the same command.

Document summaries and loaded review responses will include the effective display name and editable metadata required by the UI. The Svelte workspace will use a small inline editor within the saved-review row; command invocation stays in a focused `documentCommands.ts` helper. After save, the list and active review header use the effective display name.

Reference entries linked through `ReferenceEntry.document_id` to an excluded document are omitted from course-reference lists, RADcite review matching, course-reference exports, and module-reading exports. Unlinked manually-created entries remain visible. This matches the original app's document-level exclusion behavior. Display-name edits do not alter generated export text in this slice.

## Error Handling

- Empty display names fall back to the original filename by clearing the optional display name.
- Document numbers must be blank or greater than zero.
- Invalid enum values are rejected by typed deserialization.
- Invalid document IDs, archived documents, and project mismatches return the existing command error surface.
- The exclusion setting is explicit and does not remove or archive the document.

## Testing

- Repository round-trip test proves metadata changes persist, are returned in summaries, and leave identity/source/lifecycle columns unchanged.
- Desktop contract tests cover project scoping, active-document validation, positive-number validation, empty-name normalization, enum/exclusion updates, and the full response shape.
- Frontend helper tests cover the Tauri payload.
- Frontend workspace tests cover an extracted `documentEditorState` helper with explicit draft, cancel, save-success, and save-failure contracts; no new component-test environment is introduced for this slice.
- Desktop tests prove excluded linked references are omitted from course-reference lists, review matching, course-reference exports, and module-reading exports, while unlinked entries remain.
- Full Rust and frontend checks remain required before integration.
