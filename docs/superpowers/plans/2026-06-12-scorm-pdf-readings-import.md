# SCORM PDF Readings Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add batch SCORM PDF import to RADcite Module readings so users can preview and save readings from multiple SCORM PDF outputs.

**Architecture:** Keep extraction side-effect free in `radsuite-cite`, expose a desktop preview command in `radsuite-desktop`, and reuse the existing readings preview/save UI. Saving remains centralized in `save_module_readings_import` so existing deduping and required-over-optional precedence continue to apply.

**Tech Stack:** Rust 2024, Tauri commands, Svelte 5, TypeScript, Vitest, existing RADsuite SQLite repositories and reading import models.

---

## File Structure

- Create: `crates/radsuite-cite/src/readings_pdf.rs`
  - PDF batch reading extraction, text-to-candidate adapter, filename/module/lesson inference helpers.
- Modify: `crates/radsuite-cite/src/lib.rs`
  - Export the PDF readings import API.
- Modify: `crates/radsuite-cite/Cargo.toml`
  - Add a PDF text extraction dependency if no suitable existing dependency is available.
- Create: `crates/radsuite-cite/tests/readings_pdf_import.rs`
  - Rust tests for batch extraction, microlearning filename inference, course heading inference, and required-over-optional precedence.
- Modify: `crates/radsuite-desktop/src/commands.rs`
  - Add `PreviewModuleReadingsPdfImportRequest` and `preview_module_readings_pdf_import`.
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Desktop command tests for empty selections and multi-PDF preview.
- Modify: `apps/desktop-ui/src/types.ts`
  - Add any extra candidate/source fields only if needed by the UI.
- Modify: `apps/desktop-ui/src/lib/readingCommands.ts`
  - Add `previewModuleReadingsPdfImport`.
- Modify: `apps/desktop-ui/src/lib/readingCommands.test.ts`
  - Verify trimmed multi-path payloads.
- Modify: `apps/desktop-ui/src/components/RadciteReadingsWorkspace.svelte`
  - Add PDF import source, multi-file/folder picker, and source-aware status text.
- Modify: `apps/desktop-ui/src/App.svelte`
  - Pass the PDF preview handler into the Readings workspace.

## Task 1: PDF Candidate Extraction API

- [ ] **Step 1: Write failing `radsuite-cite` tests**

Add tests in `crates/radsuite-cite/tests/readings_pdf_import.rs` for:

- microlearning filename inference from `COMS432 Module 6 Microlearning 3.pdf`
- course-style heading inference from text containing `Module 6`
- required-over-optional duplicate precedence
- rejecting non-PDF paths

Run:

```bash
cargo test -p radsuite-cite readings_pdf_import -- --nocapture
```

Expected: fail because `readings_pdf` API does not exist.

- [ ] **Step 2: Implement minimal API shell**

Create `crates/radsuite-cite/src/readings_pdf.rs` with:

- `PdfReadingExtractionRequest { paths: Vec<PathBuf> }`
- `PdfReadingExtractionError`
- `extract_pdf_reading_candidates(request) -> Result<Vec<ReadingImportCandidate>, PdfReadingExtractionError>`

Export it from `crates/radsuite-cite/src/lib.rs`.

- [ ] **Step 3: Add PDF text extraction**

Use a Rust PDF text extraction crate if available. If adding a new crate is required, add it to workspace dependencies and keep the public extraction API isolated so the dependency can be swapped later.

- [ ] **Step 4: Reuse existing reading heuristics**

Refactor shared candidate detection from `docx.rs` if needed so DOCX and PDF extraction use the same rules for:

- required/optional labels
- reference-looking lines
- standalone URLs
- bibliography skipping
- required-over-optional duplicate precedence

- [ ] **Step 5: Run focused tests**

```bash
cargo test -p radsuite-cite readings_pdf_import -- --nocapture
```

Expected: pass.

## Task 2: Desktop Preview Command

- [ ] **Step 1: Write failing desktop contract tests**

Add tests in `crates/radsuite-desktop/tests/desktop_contracts.rs` for:

- empty `paths` returns a user-facing import error
- multiple PDF paths return combined preview candidates

Run:

```bash
cargo test -p radsuite-desktop --test desktop_contracts module_readings_pdf_import -- --nocapture
```

Expected: fail because the command does not exist.

- [ ] **Step 2: Add command request and command**

In `crates/radsuite-desktop/src/commands.rs`, add:

- `PreviewModuleReadingsPdfImportRequest { paths: Vec<String> }`
- `preview_module_readings_pdf_import`

The command trims paths, validates at least one remains, calls `extract_pdf_reading_candidates`, and maps candidates through `module_reading_import_candidate_summary`.

- [ ] **Step 3: Expose command through Tauri**

Add the command to `apps/desktop-ui/src-tauri/src/main.rs`.

- [ ] **Step 4: Run focused desktop tests**

```bash
cargo test -p radsuite-desktop --test desktop_contracts module_readings_pdf_import -- --nocapture
```

Expected: pass.

## Task 3: TypeScript Command Wiring

- [ ] **Step 1: Write failing command tests**

Add `previewModuleReadingsPdfImport` tests in `apps/desktop-ui/src/lib/readingCommands.test.ts`.

Run:

```bash
npm test -- --run apps/desktop-ui/src/lib/readingCommands.test.ts
```

Expected: fail because the helper does not exist.

- [ ] **Step 2: Implement helper**

Add:

```ts
export type PreviewModuleReadingsPdfImportInput = {
  paths: string[];
};
```

and:

```ts
export function previewModuleReadingsPdfImport(
  input: PreviewModuleReadingsPdfImportInput,
): Promise<ModuleReadingImportCandidate[]> {
  return invoke<ModuleReadingImportCandidate[]>("preview_module_readings_pdf_import", {
    request: {
      paths: input.paths.map((path) => path.trim()).filter(Boolean),
    },
  });
}
```

- [ ] **Step 3: Run focused TypeScript tests**

```bash
npm test -- --run apps/desktop-ui/src/lib/readingCommands.test.ts
```

Expected: pass.

## Task 4: Readings Workspace PDF UI

- [ ] **Step 1: Add failing UI/workflow checks**

Update existing Svelte/Vitest coverage to assert:

- import source selector includes `PDF`
- PDF picker requests multiple files
- PDF preview calls the PDF command handler with all selected paths

- [ ] **Step 2: Extend props and state**

In `RadciteReadingsWorkspace.svelte`:

- extend `importSource` to `"docx" | "csv" | "pdf"`
- add `pdfPaths` state
- add `onPreviewReadingsPdfImport` prop
- update source labels and empty-state text

- [ ] **Step 3: Add multi-file/folder selection**

Use Tauri dialog:

```ts
await open({
  multiple: true,
  directory: false,
  filters: [{ name: "PDF documents", extensions: ["pdf"] }],
});
```

If folder import is needed in the same slice, add a separate `Choose folder` action and enumerate PDF paths on the backend. Otherwise keep folder import as the next slice.

- [ ] **Step 4: Wire `App.svelte`**

Pass `previewModuleReadingsPdfImport` into `RadciteReadingsWorkspace`.

- [ ] **Step 5: Run frontend tests**

```bash
npm test -- --run
```

Expected: pass.

## Task 5: Verification And App Rebuild

- [ ] **Step 1: Format and lint**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: pass.

- [ ] **Step 2: Run Rust tests**

```bash
cargo test --workspace --all-features
```

Expected: pass.

- [ ] **Step 3: Run frontend tests**

```bash
npm test -- --run
```

Expected: pass.

- [ ] **Step 4: Rebuild desktop app**

```bash
cargo tauri build --debug
```

Run from `apps/desktop-ui` if that is the established app build directory.

- [ ] **Step 5: Manual smoke**

Open RADsuite, go to `RADcite > Readings`, select `PDF`, choose multiple PDF files, preview readings, save selected readings, and confirm the module readings export count updates.
