# RADcite Module Readings Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a working RADcite module-readings HTML export from stored Local DB module readings.

**Architecture:** Add a Rust desktop command that loads a selected RADcite module and formats its stored readings as Moodle/AKO HTML. Expose the command through Tauri, add a TypeScript wrapper, and extend the existing Exports workspace with a second export mode for module readings.

**Tech Stack:** Rust, SQLite repositories, Tauri commands, Svelte 5, TypeScript, Vitest, CSS.

---

## File Structure

- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add module-readings export request/response/error types, formatting helpers, AKO cleanup helpers, and `export_module_readings`.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add focused desktop command tests for module-readings HTML export.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose `export_module_readings` to the Tauri bridge.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add module-readings export request/response types.
- Modify: `apps/desktop-ui/src/lib/exportCommands.ts`
  - Add `exportModuleReadings`.
- Modify: `apps/desktop-ui/src/lib/exportCommands.test.ts`
  - Verify the new command payload.
- Modify: `apps/desktop-ui/src/components/RadciteExportsWorkspace.svelte`
  - Add course/reference export modes, module selector, module-reading counts, and shared preview/copy/download actions.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Store module-readings export state, call the command, and refresh modules when Exports opens.
- Modify: `apps/desktop-ui/src/styles.css`
  - Add/adjust compact controls for segmented export mode and module selector.
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
  - Require module-readings export UI and Tauri command hooks.

## Task 1: Backend Export Contract

- [ ] Add a failing desktop contract test named `module_readings_can_be_exported_as_html`.
- [ ] The test should create a module, add one compulsory reading with a URL and metadata, add one optional reading, call `export_module_readings` with `for_ako_learn: false`, and assert:
  - `reading_count == 2`
  - filename ends with `module-readings.html`
  - HTML includes `{GENERICO:type="references"}` and `{GENERICO:type="references_end"}`
  - HTML includes `Compulsory readings` and `Optional readings`
  - HTML includes escaped text and lesson-code prefixes
  - HTML includes a new-tab link for the stored URL
  - metadata appears after a `references_end` marker
- [ ] Add a failing desktop contract test named `module_readings_export_can_emit_ako_html`.
- [ ] The test should call `export_module_readings` with `for_ako_learn: true` and assert Generico tokens are absent and the hanging indent style is present.
- [ ] Add a failing desktop contract test named `module_readings_export_rejects_missing_module`.
- [ ] Run `cargo test -p radsuite-desktop module_readings_export` and confirm the command/types are missing.
- [ ] Implement the export types, error type, and command in `crates/radsuite-desktop/src/commands.rs`.
- [ ] Add HTML helpers that escape stored text, group readings, append URL links, strip Generico for AKO, and apply hanging indent to reading paragraphs.
- [ ] Run `cargo test -p radsuite-desktop module_readings_export` and confirm the tests pass.
- [ ] Commit backend export work.

## Task 2: Tauri And TypeScript Command Contract

- [ ] Add module-readings export request/response types to `apps/desktop-ui/src/types.ts`.
- [ ] Add a failing Vitest case for `exportModuleReadings({ module_id, for_ako_learn })`.
- [ ] Run `npm test -- --run src/lib/exportCommands.test.ts` and confirm the helper is missing.
- [ ] Add `exportModuleReadings` in `apps/desktop-ui/src/lib/exportCommands.ts`.
- [ ] Expose `export_module_readings` in `apps/desktop-ui/src-tauri/src/main.rs`.
- [ ] Run the focused Vitest file and confirm it passes.
- [ ] Commit bridge/helper work.

## Task 3: Exports Workspace UI

- [ ] Extend `RadciteExportsWorkspace.svelte` props for modules, selected module id, module readings, module export state, and module export callbacks.
- [ ] Add export mode controls with visible hooks:
  - `Course references`
  - `Module readings`
  - `Module readings export`
  - `Module selector`
  - `Generate HTML`
  - `Copy HTML`
  - `Download HTML`
- [ ] Update preview counts so course exports show reference count and module-reading exports show reading count.
- [ ] Wire `App.svelte` to refresh modules when Exports opens.
- [ ] Add `handleExportModuleReadings` in `App.svelte`.
- [ ] Preserve existing course-reference export behaviour.
- [ ] Update `styles.css` for compact segmented controls and module export summary.
- [ ] Extend `verify-style-contract.mjs` for the new UI and command hooks.
- [ ] Run `npm run test:style`, `npm test -- --run`, and `npm run build`.
- [ ] Commit UI work.

## Task 4: Full Verification And Publish

- [ ] Run `cargo fmt --all`.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `npm run test:style`.
- [ ] Run `npm test -- --run`.
- [ ] Run `npm run build`.
- [ ] Browser smoke-test the Exports route in Vite.
- [ ] Commit any verification fixes.
- [ ] Push the branch.
- [ ] Open a PR, wait for CI, then merge if checks pass.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session's active instructions only allow subagents when the user explicitly asks for them, so that review step is intentionally skipped.
