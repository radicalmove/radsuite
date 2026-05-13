# RADcite Review Queue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add citation-linking status counts, filters, and indicators to the RADcite document review workspace.

**Architecture:** Keep all queue counts derived from the existing review response. Extend Rust `AnalyseDocxSummary`, update Svelte types, extract frontend paragraph filtering to a tested helper, and render the additional summary cards in the existing documents workspace.

**Tech Stack:** Rust, Svelte 5, TypeScript, Vitest, CSS, existing Tauri review commands.

---

## File Structure

- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add derived linked/suggested/unlinked summary fields.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add summary-count tests before and after accepting a suggestion.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add new summary fields and paragraph filters.
- Create: `apps/desktop-ui/src/lib/paragraphFilters.ts`
  - Own the paragraph filter predicate logic.
- Create: `apps/desktop-ui/src/lib/paragraphFilters.test.ts`
  - Cover existing and new filters.
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
  - Use helper, add queue summary cards, and show paragraph status indicators.
- Modify: `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
  - Rename visible review wording.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add compact queue indicator styling if needed.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require queue labels and reviewed wording.

## Task 1: Rust Summary Contract

- [x] Write failing desktop tests for `linked_citation_count`, `suggested_citation_count`, and `unlinked_citation_count`.
- [x] Run `cargo test -p radsuite-desktop review_queue` and confirm the summary fields are missing.
- [x] Extend `AnalyseDocxSummary`.
- [x] Compute counts from citations and reference suggestions.
- [x] Run focused desktop tests and confirm they pass.

## Task 2: Frontend Filter Contract

- [x] Add failing tests in `apps/desktop-ui/src/lib/paragraphFilters.test.ts` for `linked-citation`, `suggested-citation`, and `unlinked-citation`.
- [x] Run `npm test -- --run src/lib/paragraphFilters.test.ts` and confirm the helper is missing.
- [x] Implement `apps/desktop-ui/src/lib/paragraphFilters.ts`.
- [x] Update `apps/desktop-ui/src/types.ts`.
- [x] Run the focused Vitest file and confirm it passes.

## Task 3: Review Queue UI

- [x] Update `RadciteDocumentsWorkspace.svelte` to use the filter helper.
- [x] Add summary cards for linked citations, suggested matches, and unlinked citations.
- [x] Add visible paragraph-row indicators for suggested and unlinked citation status.
- [x] Rename right-panel wording to `Mark citations reviewed` / `Reviewed`.
- [x] Extend the style contract.
- [x] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.

## Task 4: Verification And Publish

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
