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
- Modify: `apps/desktop-ui/src/types.ts`
- Modify: `apps/desktop-ui/src/styles.css`

## Task 1: Baseline And Contracts

- [ ] Install/check dependencies for the new worktree with `npm ci` if needed.
- [ ] Run `npm run test:style`, `npm run build`, and `cargo test --workspace --all-features` to confirm the baseline.
- [ ] Extend `verify-style-contract.mjs` to require:
  - `@tauri-apps/plugin-dialog`
  - `choose-docx-button`
  - `onChooseDocx`
  - `review-action-form`
  - `manualCitationText`
  - `onMarkResolved`
  - `onAddManualCitation`
  - `onVerifyCitation`
- [ ] Run `npm run test:style` and confirm it fails on the missing picker/action contract.

## Task 2: Review Action Reducers

- [ ] Add Vitest to `apps/desktop-ui`.
- [ ] Write failing tests in `apps/desktop-ui/src/lib/reviewActions.test.ts` for:
  - resolving a paragraph updates `needs_citation` and summary counts.
  - adding a manual citation appends a verified citation and clears `needs_citation`.
  - verifying citations marks selected paragraph citations as verified.
  - empty manual citation text leaves state unchanged.
- [ ] Implement `apps/desktop-ui/src/lib/reviewActions.ts`.
- [ ] Run the focused Vitest file and confirm it passes.

## Task 3: Native DOCX Picker

- [ ] Add `tauri-plugin-dialog` to the Rust workspace dependencies and app dependency.
- [ ] Add `@tauri-apps/plugin-dialog` to the desktop UI dependencies.
- [ ] Register `.plugin(tauri_plugin_dialog::init())` in `apps/desktop-ui/src-tauri/src/main.rs`.
- [ ] Add `apps/desktop-ui/src-tauri/capabilities/default.json` with `dialog:default`.
- [ ] Update `RadciteDocumentsWorkspace.svelte` to import `open`, add a `Choose DOCX` button, filter to `.docx`, and fill `docxPath`.

## Task 4: Citation Actions UI

- [ ] Update `App.svelte` to import the reducer helpers and expose action callbacks to `CitationActionsPanel`.
- [ ] Update `CitationActionsPanel.svelte` to:
  - enable `Mark as resolved` only when `needs_citation` is true.
  - show an inline manual citation form.
  - enable `Verify citation` only when citations exist and any are unverified.
  - show verified state on citation badges.
- [ ] Update `types.ts` only if the UI needs a small local action type.
- [ ] Update `styles.css` for the picker row, inline form, verified badge state, and action status copy.

## Task 5: Verification And Publish

- [ ] Run `npm run test:style`.
- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build`.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Commit and push the branch.
- [ ] Open a draft PR.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
