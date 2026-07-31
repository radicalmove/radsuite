# RADcite Recycle Bin Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the existing RADsuite archive state through a project-scoped RADcite recycle bin with restore support for documents, modules, course references, and module readings.

**Architecture:** Extend the existing repository traits with archived-list and restore operations. Add a desktop command contract that normalises the four record types into archive items and dispatches restore requests by item kind. Add a focused Svelte workspace and sidebar entry that refresh the archive and existing active lists after restoration.

**Tech Stack:** Rust, SQLx SQLite, Tauri command bridge, Svelte 5, Vitest.

---

### Task 1: Add repository archive contracts

**Files:**
- Modify: `crates/radsuite-db/src/repositories.rs`
- Test: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write failing repository tests** for listing/restoring an archived course reference, reading, module, and document while preserving active-list filtering.
- [ ] **Step 2: Run the focused repository tests** and confirm they fail because the repository traits do not expose archive listing/restoration.
- [ ] **Step 3: Add repository trait methods and SQL implementations** using the existing `archived_at` column. Use a transaction for module restore plus its child readings.
- [ ] **Step 4: Run the focused repository tests** and confirm all pass.

### Task 2: Add desktop archive contracts and commands

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
- Modify: `apps/desktop-ui/src/types.ts`
- Create: `apps/desktop-ui/src/lib/archiveCommands.ts`
- Test: `apps/desktop-ui/src/lib/archiveCommands.test.ts`

- [ ] **Step 1: Write failing desktop and command-helper tests** for project-scoped archive listing, restore dispatch, and invalid item kinds.
- [ ] **Step 2: Run those tests** and confirm they fail because the new command and helper do not exist.
- [ ] **Step 3: Add `RadciteArchiveItem`, listing/restore requests, and command implementations** that map repository records to stable serialisable item contracts.
- [ ] **Step 4: Add the Svelte invoke helpers and types** with narrow request/response functions.
- [ ] **Step 5: Run focused Rust and Vitest tests** and confirm they pass.

### Task 3: Build the recycle-bin workspace

**Files:**
- Create: `apps/desktop-ui/src/components/RadciteArchiveWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/styles.css`

- [ ] **Step 1: Add the archive area contract and rendering test hooks** for grouped records, empty state, restore action, loading, and error states.
- [ ] **Step 2: Implement the workspace** using existing workspace panels, labels, buttons, and responsive styles.
- [ ] **Step 3: Add project-scoped refresh and restore handlers** in `App.svelte`, refreshing active references, modules/readings, saved reviews, and the archive list after success.
- [ ] **Step 4: Run `npm run check` and the full frontend test suite**.

### Task 4: Integrate and verify

**Files:**
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs` if needed

- [ ] **Step 1: Register the new Tauri commands** and run the full Rust workspace tests.
- [ ] **Step 2: Run `cargo fmt --all --check` and strict clippy**.
- [ ] **Step 3: Run the frontend build and desktop package build**.
- [ ] **Step 4: Inspect the running desktop UI** for the new archive area and restore flow.
- [ ] **Step 5: Commit the implementation, push the branch, open a PR, wait for CI, and merge after checks pass.**

