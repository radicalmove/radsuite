# RADcite Reference Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a working RADcite Exports workspace that generates copyable/downloadable course-reference HTML from Local DB references.

**Architecture:** Add a Rust desktop command that formats the seeded RADcite project references as escaped HTML, then expose it through Tauri. Add a small Svelte Exports workspace with generate/copy/download controls and a tested frontend command helper.

**Tech Stack:** Rust, Tauri commands, Svelte 5, TypeScript, Vitest, CSS.

---

## File Structure

- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add export request/response/error types, HTML escaping/formatting helpers, and `export_course_references`.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add focused export contract tests.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose `export_course_references` to the Tauri bridge.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add export request/response types.
- Create: `apps/desktop-ui/src/lib/exportCommands.ts`
  - Own the Tauri invoke wrapper.
- Create: `apps/desktop-ui/src/lib/exportCommands.test.ts`
  - Verify the command payload.
- Create: `apps/desktop-ui/src/components/RadciteExportsWorkspace.svelte`
  - Render the active Exports surface and copy/download actions.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Render the Exports workspace for the existing `exports` tool area.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add compact export panel/preview styles.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require the Exports workspace route and controls.

## Task 1: Backend Export Contract

- [x] Add a failing desktop contract test named `course_references_can_be_exported_as_html`.
- [ ] The test should add two course references, call `export_course_references` with `for_ako_learn: false`, and assert:
  - `reference_count == 2`
  - filename ends with `course-references.html`
  - HTML includes `{GENERICO:type="references"}`
  - HTML escapes unsafe characters such as `&`
- [x] Add a failing desktop contract test named `course_reference_export_can_omit_generico_tags`.
- [x] Run `cargo test -p radsuite-desktop course_reference_export` and confirm the command/types are missing.
- [x] Implement the export types and command in `crates/radsuite-desktop/src/commands.rs`.
- [x] Run `cargo test -p radsuite-desktop course_reference_export` and confirm the tests pass.

## Task 2: Tauri And Frontend Command Contract

- [x] Add `CourseReferencesExportRequest` and `CourseReferencesExport` to `apps/desktop-ui/src/types.ts`.
- [x] Create `apps/desktop-ui/src/lib/exportCommands.test.ts` with a failing test for `exportCourseReferences({ for_ako_learn: true })`.
- [x] Run `npm test -- --run src/lib/exportCommands.test.ts` and confirm the helper is missing.
- [x] Create `apps/desktop-ui/src/lib/exportCommands.ts` using `invoke<CourseReferencesExport>("export_course_references", { request })`.
- [x] Expose the Rust command in `apps/desktop-ui/src-tauri/src/main.rs`.
- [x] Run the focused Vitest file and confirm it passes.

## Task 3: Exports Workspace UI

- [x] Create `apps/desktop-ui/src/components/RadciteExportsWorkspace.svelte`.
- [ ] Include visible text/control hooks:
  - `Course References Export`
  - `AKO | LEARN`
  - `Generate HTML`
  - `Copy HTML`
  - `Download HTML`
  - `export-preview`
- [x] Wire the component in `App.svelte` when `activeArea === "exports"`.
- [x] Store export loading/error/result state in `App.svelte`.
- [x] Implement browser-side copy/download actions in the component.
- [x] Update `styles.css` with export panel and preview styling.
- [x] Extend `verify-style-contract.mjs` for the active Exports route and controls.
- [x] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.

## Task 4: Verification And Publish

- [x] Run `cargo fmt --all`.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `npm run test:style`.
- [x] Run `npm test -- --run`.
- [x] Run `npm run build`.
- [x] Browser smoke-test the Exports route in the Vite preview.
- [ ] Commit and push the branch.
- [ ] Open a PR, wait for CI, then merge if checks pass.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
