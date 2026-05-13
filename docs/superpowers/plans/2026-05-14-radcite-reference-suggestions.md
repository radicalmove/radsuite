# RADcite Reference Suggestions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Suggest likely course-reference matches for detected RADcite citations and let the user accept a suggestion explicitly.

**Architecture:** Add deterministic matching in the desktop Rust layer, because it already has both review citations and course references. Return suggestions in `ReviewCitation`, then render an accept-suggestion section in the existing Svelte citation actions panel that reuses the persisted citation-link command.

**Tech Stack:** Rust, sqlx-backed repositories, Tauri commands, Svelte 5, TypeScript, Vitest, CSS style contract.

---

## File Structure

- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add suggestion response structs and pure matching helpers.
  - Include suggestions when building review paragraphs.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add desktop contract coverage for suggested course references.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add suggestion types to `ReviewCitation`.
- Modify: `apps/desktop-ui/src/lib/reviewActionCommands.test.ts`
  - Cover accept-suggestion payload behavior via the existing link command.
- Modify: `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
  - Show suggested references and accept buttons.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add compact suggestion styling that fits the right panel.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require the suggestion UI contract.

## Task 1: Desktop Suggestion Contract

- [x] Write failing tests in `crates/radsuite-desktop/tests/desktop_contracts.rs`:
  - A review with `Smith (2020)` and a Smith 2020 course reference includes a strong suggestion.
  - A review with no matching reference has no suggestions.
- [x] Run `cargo test -p radsuite-desktop reference_suggestions` and confirm the missing fields/helpers fail.
- [x] Add `ReviewCitationReferenceSuggestion` response fields.
- [x] Add pure matching helpers for citation text against `ReferenceEntry`.
- [x] Load project references in `load_review_response` and `analyse_docx_for_review`.
- [x] Add suggestions to each unlinked `ReviewCitation`.
- [x] Run the focused desktop tests and confirm they pass.

## Task 2: Frontend Types And Command Coverage

- [x] Update `apps/desktop-ui/src/types.ts` with `ReviewCitationReferenceSuggestion`.
- [x] Update existing `reviewActionCommands` test fixtures with `reference_suggestions`.
- [x] Add a test that accepting a suggested reference uses `persistLinkCitationToReference`.
- [x] Run `npm test -- --run src/lib/reviewActionCommands.test.ts` and confirm it passes.

## Task 3: Citation Actions UI

- [x] Update `CitationActionsPanel.svelte` to derive suggestion rows from selected paragraph citations.
- [x] Render `Suggested references` above the manual link form.
- [x] Add an `Accept` button per suggestion that calls `onLinkCitation(citation.id, suggestion.reference_entry_id)`.
- [x] Hide suggestions for citations already linked to a reference.
- [x] Add CSS for `.suggestion-list`, `.suggestion-card`, and confidence badges.
- [x] Extend `verify-style-contract.mjs` to require the suggestion section.
- [x] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.

## Task 4: Full Verification And Publish

- [x] Run `cargo fmt --all`.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `npm run test:style`.
- [x] Run `npm test -- --run`.
- [x] Run `npm run build`.
- [ ] Commit and push the branch.
- [ ] Open a PR, wait for CI, then merge if checks pass.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
