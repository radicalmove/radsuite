# RADcite Document Management Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users edit saved RADcite document metadata locally and make document-level reference exclusion behave like the original app.

**Architecture:** Extend the existing `CitationDocumentRepository` with a metadata-only update operation and expose it through a project-scoped Tauri command. Include effective document metadata in saved-review summaries, filter linked references through the existing command-level loading paths, and keep the Svelte editor state in a pure helper so it remains testable without a new component-test framework.

**Tech Stack:** Rust, SQLite/sqlx, Tauri commands, Svelte 5, TypeScript, Vitest.

---

### Task 1: Persist document metadata safely

**Files:**
- Modify: `crates/radsuite-db/src/repositories.rs`
- Test: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write the failing repository test**

Add a round-trip test named `radcite_document_metadata_can_be_updated` that inserts an analysed document, changes display name, document number, document variant, and exclusion status through the repository, then loads it and checks all metadata changed while ID, project ID, original filename, source path, file type, and archive state remain unchanged. Also check that a blank stored display name is represented by the effective original filename in the summary.

- [ ] **Step 2: Run the focused test to verify it fails**

Run: `cargo test -p radsuite-db --test repository_roundtrip radcite_document_metadata_can_be_updated -- --exact`

Expected: FAIL because the repository update operation is not yet available.

- [ ] **Step 3: Extend the repository contract and SQL implementation**

Add `update_document_metadata(&self, document: &Document) -> Result<(), DbError>` to `CitationDocumentRepository`. Update only `notes`, `doc_variant`, `doc_number`, `exclude_from_references`, and `updated_at`, with `archived_at IS NULL` in the predicate. Expose `CitationDocumentSummary.display_name` as a non-optional effective label: use trimmed `notes` when present, otherwise `original_filename`. Add `display_name`, `doc_variant`, `doc_number`, and `exclude_from_references` to `CitationDocumentSummary`, all three document summary queries, and `citation_summary_from_row`.

- [ ] **Step 4: Run the focused test to verify it passes**

Run: `cargo test -p radsuite-db --test repository_roundtrip radcite_document_metadata_can_be_updated -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the persistence slice**

Run: `git add crates/radsuite-db/src/repositories.rs crates/radsuite-db/tests/repository_roundtrip.rs && git commit -m "feat: persist RADcite document metadata"`

### Task 2: Add the project-scoped desktop command

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Test: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing command contract tests**

Add a focused test named `radcite_document_metadata_contract` covering a valid update, empty display-name fallback to the original filename, zero/negative document-number rejection, archived-document rejection, project mismatch rejection, and response fields for effective display name, enum wire value, number, and exclusion flag.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p radsuite-desktop --test desktop_contracts radcite_document_metadata_contract -- --exact`

Expected: FAIL because `update_radcite_document` and its request/error contract do not yet exist.

- [ ] **Step 3: Implement the command contract**

Add `UpdateRadciteDocumentRequest`, a dedicated `RadciteDocumentError`, and `update_radcite_document`. Load the document analysis, enforce the optional project ID, reject archived or missing documents, normalize display name via `trimmed_optional`, reject document numbers below one, apply `DocumentVariant`, persist through the repository, and return a refreshed `SavedRadciteReviewSummary`. Add explicit effective display-name and editable metadata fields to `AnalyseDocxReviewResponse`, and add a contract test that loads a renamed review and asserts the complete response shape.

- [ ] **Step 4: Register the Tauri bridge**

Add the wrapper command and include it in `tauri::generate_handler!` alongside the existing saved-review commands.

- [ ] **Step 5: Run the focused tests to verify they pass**

Run: `cargo test -p radsuite-desktop --test desktop_contracts radcite_document_metadata -- --exact`

Expected: PASS.

- [ ] **Step 6: Commit the command slice**

Run: `git add crates/radsuite-desktop/src/commands.rs apps/desktop-ui/src-tauri/src/main.rs crates/radsuite-desktop/tests/desktop_contracts.rs && git commit -m "feat: add RADcite document metadata command"`

### Task 3: Enforce document exclusion in RADcite output and matching

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Test: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing exclusion tests**

Add a focused test named `radcite_excluded_document_filtering` that creates linked and unlinked reference entries, marks the linked document excluded, and asserts the linked entry is absent from `list_course_references`, review matching, course-reference export, and module-reading export while the unlinked entry remains.

- [ ] **Step 2: Run the focused tests to verify they fail**

Run: `cargo test -p radsuite-desktop --test desktop_contracts radcite_excluded_document_filtering -- --exact`

Expected: FAIL because current command paths do not consult `exclude_from_references`.

- [ ] **Step 3: Add one shared filtering path**

Load active project documents through `SqliteCitationDocumentRepository`, build the excluded-document ID set, and add a shared filter helper. Use it from `load_course_reference_entries`, `list_course_references` (which currently has a direct repository path), and module-reading export. Keep entries without `document_id` visible.

- [ ] **Step 4: Run the focused tests to verify they pass**

Run: `cargo test -p radsuite-desktop --test desktop_contracts radcite_excluded_document -- --exact`

Expected: PASS.

- [ ] **Step 5: Commit the exclusion slice**

Run: `git add crates/radsuite-desktop/src/commands.rs crates/radsuite-desktop/tests/desktop_contracts.rs && git commit -m "feat: honor RADcite document reference exclusions"`

### Task 4: Add the frontend editor and command helper

**Files:**
- Create: `apps/desktop-ui/src/lib/documentCommands.ts`
- Create: `apps/desktop-ui/src/lib/documentEditorState.ts`
- Test: `apps/desktop-ui/src/lib/documentCommands.test.ts`
- Test: `apps/desktop-ui/src/lib/documentEditorState.test.ts`
- Modify: `apps/desktop-ui/src/types.ts`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/styles.css` if needed for the inline editor layout

- [ ] **Step 1: Write failing frontend helper tests**

Test that the Tauri helper sends the exact command payload, and that the pure editor helper supports draft creation, cancel/reset, successful replacement with the saved response, and save-failure retention of the draft.

- [ ] **Step 2: Run focused tests to verify they fail**

Run from `apps/desktop-ui`: `npm test -- --run src/lib/documentCommands.test.ts src/lib/documentEditorState.test.ts`

Expected: FAIL because the helpers and metadata types do not yet exist.

- [ ] **Step 3: Implement the frontend contract**

Add TypeScript types for document variants, editable metadata, and the enriched saved-review summary. Implement `updateRadciteDocument` and the pure editor-state helper.

- [ ] **Step 4: Run focused tests to verify they pass**

Run from `apps/desktop-ui`: `npm test -- --run src/lib/documentCommands.test.ts src/lib/documentEditorState.test.ts`

Expected: PASS.

- [ ] **Step 5: Wire the inline editor into Documents**

Add an Edit action beside each saved review. Render display-name, document-number, document-type, and exclusion controls with Save/Cancel. Keep the row stable, show the original filename as secondary text, retain draft values on errors, refresh the saved list after success, and update the active review header when the edited document is open.

- [ ] **Step 6: Run frontend checks**

Run from `apps/desktop-ui`: `npm test -- --run && npm run check && npm run test:style && npm run build`

Expected: all tests and checks pass.

- [ ] **Step 7: Commit the frontend slice**

Run: `git add apps/desktop-ui/src apps/desktop-ui/src-tauri/src/main.rs && git commit -m "feat: add RADcite document metadata editor"`

### Task 5: Full verification and integration

**Files:**
- No planned source changes.

- [ ] **Step 1: Run Rust formatting, lint, and tests**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`

- [ ] **Step 2: Run the complete frontend verification**

Run from `apps/desktop-ui`: `npm test -- --run && npm run check && npm run test:style && npm run build`

- [ ] **Step 3: Inspect the final diff and working tree**

Run: `git diff main...HEAD --stat && git status --short`

Expected: only the document-management implementation, tests, and its design/plan documents are present.

- [ ] **Step 4: Push and open a ready pull request**

Run: `git push -u origin codex/radcite-document-management` and `gh pr create --base main --head codex/radcite-document-management --title "Add RADcite document metadata management" --body-file <prepared-summary>`.

- [ ] **Step 5: Wait for CI and merge only after green checks**

Run: `gh pr checks <number> --watch --interval 10`, then merge with the repository’s established non-interactive workflow and fast-forward the canonical `main` worktree.
