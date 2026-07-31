# RADcite Recycle Bin Design

## Context

The original RADcite application exposes a project recycle bin. Users can remove course references, module readings, modules, and documents from active work without permanently deleting them, then restore those records later. RADsuite already stores `archived_at` on these local records and filters archived records from active lists, but it does not expose the archived records or provide restoration commands.

## Goal

Add a project-scoped RADcite Recycle bin that lists archived documents, modules, course references, and module readings and lets the user restore any item without changing the existing active-workspace behavior.

## Scope

Included:

- a new `Archive` area in the RADcite project navigation
- local database queries for archived documents, modules, references, and readings
- restore operations for each item type
- restoring a module also restores its archived module readings, matching the existing archive cascade
- refresh of active references, readings, documents, and archive state after a restore
- backward-compatible serde/API contracts and focused tests

Not included:

- permanent deletion
- cross-project restore or moving records between projects
- changing the current archive confirmation behavior
- restoring archived projects themselves

## User Experience

The project sidebar gains `Archive` beneath the existing RADcite areas. The workspace shows a compact list grouped by item type, with the item label, useful context, archive timestamp, and a `Restore` action. Empty state and errors use the existing workspace styles. Restoring an item removes it from the archive list and makes it visible again in its original workspace.

## Architecture

The existing repository traits remain the storage boundary. Each relevant repository gains explicit archived-list and restore methods rather than exposing SQL through the desktop command layer. The desktop layer maps records into a single serialisable `RadciteArchiveItem` contract with an item kind and stable identifier. A single `restore_radcite_archive_item` command dispatches to the correct repository and returns the refreshed archive listing.

Document archive state is stored in the existing `documents.archived_at` column; paragraph and citation rows remain intact so restoring the document restores the saved review. Module restoration updates the module and all archived reference entries belonging to that module in one transaction.

## Error Handling

- Archive listing is scoped to the selected project and returns a clear missing-project error through the existing command bridge.
- Restore requests reject unknown item kinds and IDs that are not archived in the selected project.
- A failed restore leaves the record archived and returns a user-facing error; no partial module restore is allowed.
- Existing active list commands continue to filter `archived_at IS NULL`.

## Testing

- repository round trips for archived and restored documents, modules, references, and readings
- desktop command tests for project scoping, archive listing, restore dispatch, and invalid requests
- Svelte type checking and command helper tests for the new contract
- full Rust workspace tests, strict clippy, frontend tests/build, and packaged desktop build

