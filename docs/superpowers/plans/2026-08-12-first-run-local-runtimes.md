# First-run local runtimes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a fresh RADsuite installation start cleanly and provide a small, architecture-aware first-run path that installs the local RADcast and RADTTS runtimes when they are needed.

**Architecture:** The desktop app remains a small Tauri bundle. It discovers local runtimes without blocking the UI, reports setup state explicitly, and launches an external bootstrap script that installs Python environments and leaves model downloads to first use. Setup is reachable before a project exists. Project storage starts empty rather than creating the old RADcite test project.

**Tech Stack:** Rust/Tauri, Svelte 5, TypeScript, shell scripting, Cargo tests, Vitest.

---

### Task 1: Remove the fresh-install sample project

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] Add a regression test proving a migrated but empty local database returns zero projects.
- [ ] Run the focused Rust test and confirm it fails because `CRJU150` is auto-created.
- [ ] Remove automatic project creation from the project-listing path and replace the UI fake project with an explicit empty state.
- [ ] Guard project-scoped refreshes and workspaces until a real project is selected.
- [ ] Add a regression check covering the complete fresh-start refresh sequence, including all project-scoped lists, and keep the sidebar usable with no selected project.
- [ ] Run the focused Rust and UI contract tests.

### Task 2: Make local-save wording platform neutral

**Files:**
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/lib/helpContent.ts`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`

- [ ] Add a UI contract assertion for platform-neutral local storage wording.
- [ ] Replace Mac-only visible labels and help text with “Saved locally”.
- [ ] Run the style contract check.

### Task 3: Make RADTTS discovery bounded and asynchronous

**Files:**
- Modify: `crates/radsuite-desktop/src/radt_ts.rs`
- Modify: `crates/radsuite-desktop/src/radt_ts_tools.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`

- [ ] Add a bounded probe test using a deliberately slow executable.
- [ ] Run it and confirm it fails or hangs with the current unbounded probe behavior.
- [ ] Add process timeout/termination handling and run discovery on a blocking worker from the Tauri command.
- [ ] Route media transcription/clip capability checks through the same bounded path.
- [ ] Run the focused Rust tests and desktop contract suite.

### Task 4: Add the small local-runtime bootstrap

**Files:**
- Create: `scripts/setup-local-runtimes.sh`
- Create: `scripts/setup-local-runtimes.ps1`
- Create: `docs/local-runtime-setup.md`
- Modify: `apps/desktop-ui/src-tauri/tauri.conf.json`

- [ ] Add architecture-aware Python 3.11 checks, per-user virtual environments, idempotent installs, and clear progress/error output for macOS and Windows.
- [ ] Install RADcast and RADTTS from their published Git repositories while leaving large model downloads to first use.
- [ ] Add a dry-run/diagnostic mode and documentation for the app’s expected helper locations.
- [ ] Validate shell syntax and run the diagnostic mode on this Mac.

### Task 5: Surface setup state in the app and package the result

**Files:**
- Modify: `apps/desktop-ui/src/components/RadtTsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadtTsToolsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadcastWorkspace.svelte`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `apps/desktop-ui/src-tauri/tauri.conf.json`

- [ ] Add a first-run setup action on the no-project screen that launches the bundled bootstrap and reports progress/failure without freezing the workspace.
- [ ] Keep the initial app bundle small and make the runtime download explicit.
- [ ] Build and inspect Apple Silicon and Intel packages, then run the full verification suite.
