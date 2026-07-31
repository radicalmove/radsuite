# RADcite Course Reference Merge Design

## Goal

Close a remaining RADcite reference-management parity gap by allowing users to merge duplicate course references without losing paragraph citation links or useful metadata.

## Scope

This slice covers active project-level `reference` entries in the Course References workspace. It does not merge module readings, run network lookup, or alter the archive/recycle-bin model beyond archiving the duplicate entries after a successful merge.

The user flow is:

1. Select two or more course references.
2. Choose the selected entry that should remain as the primary reference.
3. Confirm the merge.
4. RADsuite keeps the primary entry, fills empty primary metadata from the duplicates, reassigns paragraph citation links, and archives the duplicates.

## Backend Boundary

Add a repository-level merge operation that performs the entire mutation in one SQLite transaction. It updates the primary reference fields, changes every `paragraph_citations.reference_entry_id` pointing at a duplicate to the primary ID, and archives the duplicate `reference_entries` with one timestamp. The command validates that the primary and all targets are active course references belonging to the same project and limits the operation to ten duplicate targets, matching the original RADcite behavior.

Metadata is merged conservatively: existing primary values win; missing primary values may be filled from the first duplicate that has a value. The merge never overwrites a user’s selected primary citation, never changes reference type, and never touches module-reading entries.

## Frontend Boundary

Extend the course reference command wrapper with a typed merge request. The references workspace adds selection checkboxes, a selected-count indicator, a primary-reference selector, a `Merge selected` action, and a clear-selection action. The action is disabled until at least two entries are selected and a primary is chosen. Confirmation text names the primary and explains that the other entries will be archived.

The parent app owns the async command, error message, and refresh. The component retains selection if the command fails and clears it after a successful merge. Selection is reset when the refreshed reference list no longer contains the selected IDs.

## Error Handling

Invalid IDs, fewer than two selected entries, a primary included as a target, archived/missing entries, cross-project entries, non-course-reference entries, and more than ten targets return a user-readable command error. The transaction rolls back on any database failure. The UI keeps the current list and selection visible when the operation fails.

## Verification

- Repository contract proves paragraph links move to the primary and duplicate entries become archived atomically.
- Desktop contract proves validation, metadata consolidation, and project scoping.
- TypeScript command test proves the exact Tauri payload.
- Frontend style contract proves selection and merge controls are present.
- Full Rust and frontend checks plus CI run before integration.
