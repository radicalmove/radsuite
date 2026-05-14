# RADcite Readings Edit Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add inline edit and soft-remove actions for RADcite modules and module readings.

**Architecture:** Extend the existing SQLite repositories with update/archive methods, expose focused desktop/Tauri commands, then wire the Readings workspace to reuse its add forms as edit forms. Soft-removal uses `archived_at`, keeping active list/export behaviour consistent with current queries.

**Tech Stack:** Rust, SQLx/SQLite, Tauri commands, Svelte 5, TypeScript, Vitest, CSS.

---

## File Structure

- Modify: `crates/radsuite-db/src/repositories.rs`
  - Add module/reference update and archive methods.
- Modify: `crates/radsuite-db/tests/repository_roundtrip.rs`
  - Add repository contract coverage for edit/archive.
- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add request types and commands for module/reading update/archive.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add desktop command coverage and validation tests.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose new commands.
- Modify: `apps/desktop-ui/src/lib/readingCommands.ts`
  - Add four Tauri helper functions.
- Modify: `apps/desktop-ui/src/lib/readingCommands.test.ts`
  - Verify new helper payloads.
- Modify: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
  - Add inline edit/remove UI and form mode switching.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Add handlers and refresh state after edit/archive actions.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add compact action-row styling.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require edit/remove controls and command hooks.

## Task 1: Repository Edit/Archive Contracts

- [x] Add failing repository tests for module update/archive and reading update/archive.
- [x] Run `cargo test -p radsuite-db module_readings_can_be_updated_and_archived` and confirm repository methods are missing.
- [x] Implement `update_course_module`, `archive_course_module`, `load_reference_entry`, `update_reference_entry`, and `archive_reference_entry`.
- [x] Ensure archived modules are hidden from list/load and archive child module readings.
- [x] Ensure archived readings are hidden from module reading lists.
- [x] Run the focused repository tests and confirm they pass.
- [x] Commit repository work.

## Task 2: Desktop Command Contracts

- [x] Add failing desktop tests for updating/archiving a module and updating/archiving a reading.
- [x] Add failing validation coverage for empty module title, missing module, missing reading, invalid category, and empty reading text.
- [x] Run `cargo test -p radsuite-desktop module_readings_update` and confirm command/types are missing.
- [x] Implement desktop request types and command functions.
- [x] Reuse existing validation helpers where possible.
- [x] Run focused desktop tests and confirm they pass.
- [x] Commit desktop command work.

## Task 3: Tauri And TypeScript Bridge

- [x] Add failing Vitest cases for `updateRadciteModule`, `archiveRadciteModule`, `updateModuleReading`, and `archiveModuleReading`.
- [x] Run `npm test -- --run src/lib/readingCommands.test.ts` and confirm helpers are missing.
- [x] Implement helpers in `apps/desktop-ui/src/lib/readingCommands.ts`.
- [x] Expose Tauri commands in `apps/desktop-ui/src-tauri/src/main.rs`.
- [x] Run focused Vitest and confirm it passes.
- [x] Commit bridge work.

## Task 4: Readings Workspace UI

- [x] Extend `RadciteReadingsWorkspace.svelte` props for update/archive callbacks.
- [x] Add module edit/remove actions and module form add/update modes.
- [x] Add reading edit/remove actions and reading form add/update modes.
- [x] Add confirmation before archive calls.
- [x] Wire new handlers in `App.svelte`, clearing export state when readings/modules change.
- [x] Add/adjust compact action styles.
- [x] Update `verify-style-contract.mjs`.
- [x] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.
- [x] Commit UI work.

## Task 5: Full Verification And Publish

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `npm run test:style`.
- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build`.
- [ ] Browser smoke-test the Readings route in Vite.
- [ ] Commit any verification fixes.
- [ ] Push the branch.
- [ ] Open a PR, wait for CI, then merge if checks pass.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session's active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
