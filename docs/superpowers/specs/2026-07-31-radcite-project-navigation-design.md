# RADcite Project Navigation Design

**Date:** 2026-07-31
**Status:** Proposed

## Goal

Make a multi-course RADsuite workspace easier to scan and maintain by separating active and archived projects, allowing each project's tool list to collapse, and replacing implementation-focused status labels with plain-language local/offline wording.

## Context

The current Svelte sidebar renders every project and every tool at full height. It has no project-level archive state, so completed or inactive courses continue to compete with current work. The database already uses soft archive timestamps for documents, modules, references, and readings; projects should use the same preservation model.

The current application header exposes `Local DB ready` and `Sync off`. These describe internal implementation state rather than user outcomes. RADsuite is currently local-first, so the primary message should be that work is saved on this Mac. Cloud synchronisation should be described only as a connection state.

## Design

### Active project navigation

- The main sidebar section is labelled **Active projects**.
- Active projects are listed in the existing deterministic course-code/title order.
- Every project keeps its code and title visible in a compact project header.
- The project header remains the selection target. Selecting a project also expands its tools.
- A separate chevron control expands or collapses that project's tools without changing the selected project.
- The selected project is expanded automatically. The expanded/collapsed state is stored locally in the browser shell so it survives a reload, while selecting a project always makes that project visible.
- Selection takes precedence over the saved collapsed state: selecting an active project expands its card and selecting an archived project expands both the Archived projects section and that project card. A project that is merely present on reload does not override a saved collapsed state.
- Existing RADcite, Audio cleanup, and Voice generation destinations remain under the same project; this change does not alter tool availability or routes.

### Archived projects

- Projects gain a nullable `archived_at` timestamp in the domain and SQLite schema.
- The sidebar filters archived projects out of **Active projects** and shows them in a collapsed **Archived projects** section with a count.
- Archiving is a soft archive. It does not remove or modify project documents, modules, readings, references, audio, or review history.
- Each active project exposes an **Archive** action. Each archived project exposes a **Restore** action.
- An archived project can still be selected from the Archived section so its data can be inspected or restored. Its tool list is collapsed by default.
- If the selected project is archived and another active project exists, the app selects the first active project after the archive operation and expands that active project. If no active project remains, the archived project stays selected and the Archived projects section opens so the workspace does not become unusable.
- If an unselected project is archived, selection does not change. Restoring a project selects and expands the restored project so the result of the action is visible.
- Restoring a project returns it to Active projects without changing its contents.
- Archive and restore operations are idempotent at the repository boundary: repeating the same operation leaves the project in the requested state and does not create duplicate records.

### Plain-language status

The top-level status strip uses the following labels:

| Internal state | User-facing label |
| --- | --- |
| Local database ready | **Saved on this Mac** |
| Local database unavailable | **Local saving unavailable** |
| Cloud sync configured | **Cloud sync on** |
| Cloud sync not configured | **Cloud sync not connected** |

The status labels retain accessible titles/details for users who need more context. Other in-workspace copy that currently says `Local DB` will be changed to `Saved locally` when it describes where an action was stored, rather than exposing the database term.

## Architecture

### Core and database

- Add `archived_at: Option<DateTime<Utc>>` to `radsuite_core::Project`, initialized to `None` by `Project::new`.
- Add `archived_at` to the projects table through a forward-only migration.
- The migration must upgrade an existing version-2 database without rewriting child rows; existing projects receive `archived_at = NULL`. Archive and restore update both `archived_at` and `updated_at`, matching the timestamp semantics used by the existing module archive operations.
- Extend `ProjectRepository` with `archive_project` and `restore_project` methods.
- Include the timestamp in project list/load queries and row mapping. Existing list callers continue to receive both active and archived projects so the desktop shell can present both sections.
- Extend the desktop project summary with an optional RFC 3339 `archived_at` value.
- Extend the shared `ApiProjectSummary` with the same optional field so the domain and API serialization remain structurally consistent. This slice does not add server archive/restore routes or cloud-sync behavior; the desktop commands are the only new mutation surface.

### Desktop command bridge

- Add `archive_radcite_project` and `restore_radcite_project` commands using the existing project lookup/error conventions. Each accepts a request object of `{ project_id: ProjectId }` and returns the refreshed `RadciteProjectSummary`; a missing ID returns `RADCITE_PROJECT_NOT_FOUND` through the existing string error path rather than silently updating zero rows.
- Register both commands in the Tauri command list.
- Keep project-scoped content commands unchanged; archive controls project visibility, not data access.

### Svelte shell

- Extend `ProjectNavItem` and the project command wrapper with archive state and archive/restore operations.
- Keep one project collection in `App.svelte`; derive active and archived subsets in the sidebar. This avoids separate loading paths and ensures a refresh cannot show stale archive state.
- After archive/restore, refresh the project list and choose a valid selected project using the rules above.
- Keep expansion state in `ProjectSidebar.svelte` and persist only the project IDs and their open/closed state in `localStorage` under `radciteProjectNavState`, using a versioned object `{ version: 1, expandedProjectIds: string[] }`. Malformed, unavailable, or quota-blocked storage falls back to the selected-project default without affecting project data.
- Add a small safe-storage helper used by both `App.svelte` theme persistence and `ProjectSidebar.svelte`. It catches `SecurityError`, quota errors, and malformed JSON so storage failures never prevent the app from loading or the project list from rendering.

## Error handling

- Database failures are returned through the existing command error-to-string path and shown in the sidebar notice.
- A missing project produces the existing missing-project error rather than silently creating a project.
- A failed archive or restore leaves the current project selection and expansion state unchanged.
- If local storage cannot be read or written, navigation remains functional; persistence of expansion state is best effort only.

## Testing

### Rust repository and command tests

- Verify new projects load with `archived_at = None`.
- Verify repository archive and restore transitions preserve the project and its contents.
- Verify the forward migration upgrades a pre-archive schema with `archived_at = NULL` and leaves child rows/audio references intact.
- Verify archive and restore update `updated_at` and remain idempotent when repeated.
- Verify listing and desktop summaries expose archive state.
- Verify archive and restore commands accept `{ project_id }`, reject missing project IDs with the project-not-found error, and return the refreshed summary for valid projects.
- Verify the existing project-scoped RADcite and RADcast contract tests still pass with archived projects present.

### Frontend tests and contract checks

- Verify project command wrappers invoke the expected archive and restore command names and payloads.
- Verify pure navigation helpers cover active/archived partitioning, selection transitions after archive/restore, and the last-active-project case.
- Verify the safe-storage helper covers missing storage, malformed state, and read/write exceptions.
- Extend the style/contract test to verify the sidebar exposes Active projects and Archived projects, archive/restore actions, accessible expand/collapse controls, and the status copy does not render `Local DB ready` or `Sync off`.
- Extend the style/contract test to cover all user-facing `Local DB` strings that are changed to `Saved locally`.
- Run the existing style and frontend test scripts.

### Manual verification

- Create two projects, collapse one, reload the app, and confirm expansion state is retained.
- Archive the selected project and confirm the app selects another active project.
- Restore the archived project and confirm it returns to Active projects with its content intact.
- Archive the only active project and confirm the project remains usable from Archived projects.
- Check light and dark modes at desktop and narrow widths for clipping or overlapping controls.

## Non-goals

- This slice does not add cloud synchronisation.
- This slice does not delete projects or their data.
- This slice does not introduce project search, drag-and-drop ordering, or a separate project-management screen.
- This slice does not change the underlying RADcite, RADcast, or RADTTS workflows.
