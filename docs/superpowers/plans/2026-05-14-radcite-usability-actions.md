# RADcite Usability Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native DOCX file selection and local citation review actions to the RADcite desktop review workspace.

**Architecture:** Use the official Tauri v2 dialog plugin for file selection and keep review actions as frontend session-state updates. Extract the action logic into a small TypeScript module so it can be tested without mounting Svelte components.

**Tech Stack:** Svelte 5, TypeScript, Vite/Vitest, Tauri 2, Rust, CSS, Node style contract.

---

## File Structure

- Modify: `Cargo.toml`
- Modify: `apps/desktop-ui/package.json`
- Modify: `apps/desktop-ui/package-lock.json`
- Modify: `apps/desktop-ui/src-tauri/Cargo.toml`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Create: `apps/desktop-ui/src-tauri/capabilities/default.json`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
- Create: `apps/desktop-ui/src/lib/reviewActions.ts`
- Create: `apps/desktop-ui/src/lib/reviewActions.test.ts`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
- Modify: `apps/desktop-ui/src/styles.css`

## Task 1: Baseline And Contracts

- [x] Install/check dependencies for the new worktree with `npm ci` if needed.
- [x] Run `npm run test:style`, `npm run build`, and `cargo test --workspace --all-features` to confirm the baseline.
- [x] Extend `verify-style-contract.mjs` to require:
  - `@tauri-apps/plugin-dialog`
  - `choose-docx-button`
  - `onChooseDocx`
  - `review-action-form`
  - `manualCitationText`
  - `onMarkResolved`
  - `onAddManualCitation`
  - `onVerifyCitation`
- [x] Run `npm run test:style` and confirm it fails on the missing picker/action contract.

## Task 2: Review Action Reducers

- [x] Add Vitest to `apps/desktop-ui`.
- [x] Write failing tests in `apps/desktop-ui/src/lib/reviewActions.test.ts` for:
  - resolving a paragraph updates `needs_citation` and summary counts.
  - adding a manual citation appends a verified citation and clears `needs_citation`.
  - verifying citations marks selected paragraph citations as verified.
  - empty manual citation text leaves state unchanged.
- [x] Implement `apps/desktop-ui/src/lib/reviewActions.ts`.
- [x] Run the focused Vitest file and confirm it passes.

## Task 3: Native DOCX Picker

- [x] Add `tauri-plugin-dialog` to the Rust workspace dependencies and app dependency.
- [x] Add `@tauri-apps/plugin-dialog` to the desktop UI dependencies.
- [x] Register `.plugin(tauri_plugin_dialog::init())` in `apps/desktop-ui/src-tauri/src/main.rs`.
- [x] Add `apps/desktop-ui/src-tauri/capabilities/default.json` with `dialog:default`.
- [x] Update `RadciteDocumentsWorkspace.svelte` to import `open`, add a `Choose DOCX` button, filter to `.docx`, and fill `docxPath`.

## Task 4: Citation Actions UI

- [x] Update `App.svelte` to import the reducer helpers and expose action callbacks to `CitationActionsPanel`.
- [x] Update `CitationActionsPanel.svelte` to:
  - enable `Mark as resolved` only when `needs_citation` is true.
  - show an inline manual citation form.
  - enable `Verify citation` only when citations exist and any are unverified.
  - show verified state on citation badges.
- [x] Confirm no `types.ts` change is needed because existing review types cover the local actions.
- [x] Update `styles.css` for the picker row, inline form, verified badge state, and action status copy.

## Task 5: Verification And Publish

- [x] Run `npm run test:style`.
- [x] Run `npm test -- --run`.
- [x] Run `npm run build`.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test --workspace --all-features`.
- [ ] Commit and push the branch.
- [ ] Open a draft PR.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
