# RADcite Project Navigation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add persistent project archiving, compact collapsible project navigation, and plain-language local/offline status copy to the RADsuite desktop app.

**Architecture:** Keep project archive state in the shared `Project` model and local SQLite database, expose desktop-only archive/restore commands, and let the Svelte shell derive Active and Archived sections from one project list. Keep navigation persistence separate from project data through a small safe local-storage helper and pure navigation-selection helpers.

**Tech Stack:** Rust, SQLite/sqlx migrations, Tauri command bridge, Svelte 5 runes, TypeScript, Vitest, existing style-contract checks.

**Design reference:** `docs/superpowers/specs/2026-07-31-radcite-project-navigation-design.md`

---

### Task 1: Add project archive state to the domain and database

**Files:**
- Modify: `crates/radsuite-core/src/domain.rs`
- Modify: `crates/radsuite-core/src/api.rs`
- Create: `crates/radsuite-db/migrations/0003_project_archive.sql`
- Modify: `crates/radsuite-db/src/repositories.rs`
- Test: `crates/radsuite-core/tests/domain_contracts.rs`
- Test: `crates/radsuite-db/tests/repository_roundtrip.rs`
- Test: `crates/radsuite-db/tests/migration_upgrade.rs`

- [ ] **Step 1: Write failing domain and repository tests**

Add a dedicated archive-state assertion for a new project and API summary, plus repository assertions that archiving/restoring a project changes only archive metadata while preserving child records. Add a legacy-schema migration fixture that creates a pre-0003 SQLite schema, inserts a project and child rows, applies the new migration, and asserts `archived_at` is null and child rows remain intact.

- [ ] **Step 2: Run the focused tests to verify the new behavior fails**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts project_owner_can_be_returned_as_api_summary
cargo test -p radsuite-db --test repository_roundtrip project_can_be_archived_and_restored
cargo test -p radsuite-db --test migration_upgrade
```

Expected: compilation/test failure because the project model, migration, and repository methods do not yet expose archive state.

- [ ] **Step 3: Add the forward migration and model fields**

Create `0003_project_archive.sql` with `ALTER TABLE projects ADD COLUMN archived_at TEXT;`. Add `archived_at: Option<DateTime<Utc>>` to `Project` and `ApiProjectSummary`, initialize it to `None`, and include it in `ApiProjectSummary::from_project`. The migration-upgrade test must use a pre-0003 schema rather than only testing a fresh current migration.

- [ ] **Step 4: Extend repository queries and archive transitions**

Update project insert/list/load queries and `project_from_row` to include `archived_at`. Add `archive_project` and `restore_project` to `ProjectRepository`. Each operation must update `archived_at` and `updated_at`, return a missing-project error when no ID matches, and be idempotent when the project is already in the requested state.

- [ ] **Step 5: Run the focused tests to verify they pass**

Run:

```bash
cargo test -p radsuite-core --test domain_contracts
cargo test -p radsuite-db --test repository_roundtrip
cargo test -p radsuite-db --test migration_upgrade
```

Expected: all focused core and database tests pass, including migration upgrade coverage and preservation of child rows.

- [ ] **Step 6: Commit the database slice**

```bash
git add crates/radsuite-core crates/radsuite-db
git commit -m "feat: persist RADcite project archive state"
```

### Task 2: Add desktop archive and restore command contracts

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing desktop command tests**

Add tests for archiving/restoring a project with `{ project_id }`, returning a refreshed summary, rejecting a missing project ID, and exposing `archived_at` through `list_radcite_projects`.

- [ ] **Step 2: Run the focused desktop tests to verify they fail**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts project_archive
```

Expected: compilation failure because the request type and command functions do not exist.

- [ ] **Step 3: Implement the request types, error, and command functions**

Add `ArchiveRadciteProjectRequest` and `RestoreRadciteProjectRequest` containing `project_id: ProjectId`. Extend `RadciteProjectSummary` with `archived_at`. Add a missing-project variant to `RadciteProjectError`, implement the command functions through `SqliteProjectRepository`, and return the refreshed summary.

- [ ] **Step 4: Register the commands with Tauri**

Import the request types and add `archive_radcite_project` and `restore_radcite_project` to the generated command handler in `apps/desktop-ui/src-tauri/src/main.rs`.

- [ ] **Step 5: Run the focused desktop tests**

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts
```

Expected: all desktop contract tests pass, including existing project-scoped RADcite and RADcast tests.

- [ ] **Step 6: Commit the command slice**

```bash
git add crates/radsuite-desktop apps/desktop-ui/src-tauri/src/main.rs
git commit -m "feat: add RADcite project archive commands"
```

### Task 3: Add tested frontend navigation and storage helpers

**Files:**
- Modify: `apps/desktop-ui/src/types.ts`
- Modify: `apps/desktop-ui/src/lib/projectCommands.ts`
- Modify: `apps/desktop-ui/src/lib/projectCommands.test.ts`
- Create: `apps/desktop-ui/src/lib/projectNavigation.ts`
- Create: `apps/desktop-ui/src/lib/projectNavigation.test.ts`
- Create: `apps/desktop-ui/src/lib/storage.ts`
- Create: `apps/desktop-ui/src/lib/storage.test.ts`

- [ ] **Step 1: Write failing helper and command-wrapper tests**

Cover archive-state partitioning, selection after archiving a selected project, selection when no active project remains, selection after restore, malformed/missing storage, storage read/write exceptions, and exact Tauri payloads for archive/restore.

- [ ] **Step 2: Run the focused frontend tests to verify they fail**

Run from `apps/desktop-ui`:

```bash
npm test -- --run src/lib/projectNavigation.test.ts src/lib/storage.test.ts src/lib/projectCommands.test.ts
```

Expected: failure because the helpers, archive fields, and command wrappers do not yet exist.

- [ ] **Step 3: Implement the type and command contracts**

Add `archived_at: string | null` to `RadciteProjectSummary` and `ProjectNavItem`. Add `archiveRadciteProject` and `restoreRadciteProject` wrappers that trim/validate the project ID and invoke the exact Tauri command names with `{ request: { project_id } }`.

- [ ] **Step 4: Implement pure navigation helpers**

Create helpers for active/archived partitioning and deterministic selection transitions. Keep the last-active-project rule explicit: if no active project remains, preserve the archived selection and request an open Archived section.

- [ ] **Step 5: Implement safe storage helpers**

Create versioned JSON read/write helpers for `radciteProjectNavState` and string helpers for `radciteTheme`. All access must catch unavailable storage, security errors, malformed JSON, and quota failures, returning the caller’s fallback value.

- [ ] **Step 6: Run the focused frontend tests**

Run:

```bash
npm test -- --run src/lib/projectNavigation.test.ts src/lib/storage.test.ts src/lib/projectCommands.test.ts
```

Expected: all helper and command-wrapper tests pass.

- [ ] **Step 7: Commit the frontend contract slice**

```bash
git add apps/desktop-ui/src/types.ts apps/desktop-ui/src/lib
git commit -m "feat: add project navigation contracts"
```

### Task 4: Implement the collapsible Active/Archived sidebar

**Files:**
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
- Modify: `apps/desktop-ui/src/styles.css`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
- Test: `apps/desktop-ui/src/lib/projectNavigation.test.ts`

- [ ] **Step 1: Add failing style-contract assertions for the sidebar contract**

Require `Active projects`, `Archived projects`, archive/restore action labels, accessible expand/collapse labels, status `title`/`aria-label` context, and the new plain-language status copy. Require that obsolete `Local DB ready` and `Sync off` strings are absent.

- [ ] **Step 2: Run the style contract to verify it fails**

```bash
cd apps/desktop-ui
npm run test:style
```

Expected: failure because the current sidebar and status strip do not expose the new contract.

- [ ] **Step 3: Add a failing navigation mutation test**

Extend `projectNavigation.test.ts` with a failed archive/restore outcome. Assert that the selected project ID, expanded project IDs, and Archived-section state are unchanged when the command rejects; this is the contract the app-level error path must preserve.

- [ ] **Step 4: Add the app-level archive/restore callbacks and selection logic**

Import the new wrappers and helpers, map `archived_at`, use safe theme storage, refresh the single project collection after archive/restore, and reset project-scoped state only when the selected project changes. Preserve the current project when an unselected project changes state. A failed archive/restore must leave selection and expansion state unchanged.

- [ ] **Step 5: Implement the sidebar sections and controls**

Split the supplied project list into active and archived sections. Add a compact project header, a separate chevron control, archive/restore actions, an archived count, and an empty-state message. Use selection precedence from the design: selected archived projects open the Archived section, and archive/restore actions retain or move selection as specified.

- [ ] **Step 6: Persist expansion state safely**

Load the versioned state with the safe-storage helper, ignore malformed IDs, auto-expand the selected project, and write state changes without allowing storage failures to affect navigation.

- [ ] **Step 7: Add styles for compact project rows and action controls**

Keep the existing RADcite visual language, dark-mode contrast, square button radii, and responsive sidebar behavior. Ensure the project title, status, chevron, and archive action do not overlap at narrow widths.

- [ ] **Step 8: Run frontend checks and style contracts**

Run from `apps/desktop-ui`:

```bash
npm test -- --run
npm run check
npm run test:style
```

Expected: all frontend tests, Svelte type checks, and style contracts pass.

- [ ] **Step 9: Commit the sidebar slice**

```bash
git add apps/desktop-ui/src apps/desktop-ui/scripts/verify-style-contract.mjs
git commit -m "feat: add active and archived project navigation"
```

### Task 5: Replace remaining technical local-status copy

**Files:**
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteReferencesWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteExportsWorkspace.svelte`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`

- [ ] **Step 1: Add failing exact-copy assertions**

Extend the style contract to check all expected labels and reject obsolete implementation wording in user-facing Svelte files.

- [ ] **Step 2: Run the style contract to verify it fails**

```bash
cd apps/desktop-ui
npm run test:style
```

Expected: failure because the current workspaces still contain database-oriented copy.

- [ ] **Step 3: Replace user-facing copy**

Use `Saved on this Mac` for the top-level ready state, `Local saving unavailable` for failure, `Cloud sync on` when configured, `Cloud sync not connected` otherwise, and `Saved locally` in action notices that describe persistence.

- [ ] **Step 4: Run frontend validation**

```bash
npm run check
npm run test:style
```

Expected: both commands pass with no obsolete status strings.

- [ ] **Step 5: Commit the copy slice**

```bash
git add apps/desktop-ui/src apps/desktop-ui/scripts/verify-style-contract.mjs
git commit -m "ux: clarify local project status"
```

### Task 6: Full verification and desktop smoke check

**Files:**
- No planned source changes; update `docs/development.md` only if a new verification command is required.

- [ ] **Step 1: Run formatting and static checks**

From the worktree root:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

Expected: all commands pass.

- [ ] **Step 2: Run the complete frontend build**

```bash
cd apps/desktop-ui
npm run build
```

Expected: Svelte checks and Vite production build pass.

- [ ] **Step 3: Build and launch the desktop bundle**

From the repository root, run:

```bash
cd apps/desktop-ui/src-tauri && cargo tauri build
```

Launch the generated macOS app from the bundle path reported by the Tauri CLI (the current workspace convention is `target/release/bundle/macos/`). For an interactive development smoke check, run `cd apps/desktop-ui/src-tauri && cargo tauri dev` from the repository root. Verify the sidebar manually with two projects: collapse/reload, archive/restore, the zero-active-project case, accessible status descriptions, light/dark themes, and narrow-width layout.

- [ ] **Step 4: Inspect the final diff and branch state**

```bash
git diff main...HEAD --check
git status --short --branch
git log --oneline --decorate -8
```

Expected: only the planned navigation/status files and spec/plan documents are changed, with no untracked build artifacts.

- [ ] **Step 5: Commit any required verification documentation**

Only if verification required a documentation update:

```bash
git add docs/development.md
git commit -m "docs: record project navigation verification"
```
