# RADcite Managed Document Sources Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist a project-owned copy of every analysed DOCX/PDF and reuse it from saved RADcite reviews and Module Readings.

**Architecture:** Add a small desktop document-store boundary that copies sources into the existing app data directory using a document UUID and safe filename. Persist the managed path and file type through the existing `Document` repository and review contracts. Keep the original picker path in the UI for display, while using the managed path for automatic readings reuse.

**Tech Stack:** Rust 2024, SQLite/sqlx migrations, Tauri command contracts, Svelte 5, Vitest.

---

### Task 1: Add the managed document-store boundary

**Files:**
- Create: `crates/radsuite-desktop/src/document_store.rs`
- Modify: `crates/radsuite-desktop/src/lib.rs`
- Test: `crates/radsuite-desktop/src/document_store.rs`

- [ ] **Step 1: Write failing unit tests** for safe filenames, project/document destination layout, successful source copying, and missing-source errors.
- [ ] **Step 2: Run the focused Rust test and verify it fails because the store API is absent.**
- [ ] **Step 3: Implement `store_source` and a small `DocumentStorageError` with app-data-relative destination layout and filename sanitisation.**
- [ ] **Step 4: Run the focused test and verify it passes.**
- [ ] **Step 5: Commit the storage boundary.**

### Task 2: Persist managed paths through core and SQLite

**Files:**
- Modify: `crates/radsuite-core/src/domain.rs`
- Create: `crates/radsuite-db/migrations/0004_document_source_path.sql`
- Modify: `crates/radsuite-db/src/repositories.rs`
- Test: `crates/radsuite-db/tests/migration_upgrade.rs`
- Test: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Add failing domain/repository assertions** that a document source path round-trips and old rows load with `None`.
- [ ] **Step 2: Run the focused database tests and verify the new assertions fail.**
- [ ] **Step 3: Add nullable `Document.source_path`, migration `0004`, summary mapping, insert/load/select SQL, and row parsing.**
- [ ] **Step 4: Run migration and repository tests and verify they pass.**
- [ ] **Step 5: Commit the persistence layer.**

### Task 3: Copy sources during DOCX/PDF analysis and expose review contracts

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`
- Modify: `apps/desktop-ui/src/types.ts`
- Modify: `apps/desktop-ui/src/lib/savedReviewCommands.ts`
- Modify: `apps/desktop-ui/src/lib/archiveCommands.ts`

- [ ] **Step 1: Add failing desktop contract tests** proving DOCX and PDF analysis store a managed source, saved-review responses expose `source_path` and `source_file_type`, and a copy failure leaves no saved document.
- [ ] **Step 2: Run the focused desktop tests and verify the new assertions fail.**
- [ ] **Step 3: Copy the selected source before ingestion, attach the managed path to `Document`, map the storage error into DOCX/PDF analysis errors, and include source metadata in saved-review/archived-review responses.**
- [ ] **Step 4: Run focused desktop tests and verify they pass.**
- [ ] **Step 5: Commit the command contract.**

### Task 4: Restore source reuse in the Svelte workflow

**Files:**
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/lib/savedReviewCommands.test.ts`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`

- [ ] **Step 1: Add failing frontend assertions** for saved-review source metadata and the DOCX-only `Use for readings` action.
- [ ] **Step 2: Run the focused frontend tests and verify they fail.**
- [ ] **Step 3: Restore managed source metadata when opening a review, add the saved-review readings handoff, and preserve legacy reviews without source paths.**
- [ ] **Step 4: Run frontend tests, type-check, style contract, and production build.**
- [ ] **Step 5: Commit the Svelte workflow.**

### Task 5: Full verification and release artifact

**Files:**
- Modify: `docs/development.md` only if the final local-source behavior needs an operator note.

- [ ] **Step 1: Run `cargo fmt --all --check`.**
- [ ] **Step 2: Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.**
- [ ] **Step 3: Run `cargo test --workspace --all-features`.**
- [ ] **Step 4: Run `npm test -- --run`, `npm run check`, `npm run test:style`, and `npm run build` in `apps/desktop-ui`.**
- [ ] **Step 5: Run browser smoke verification for saved-review reuse at desktop and mobile widths.**
- [ ] **Step 6: Run `cargo tauri build` and verify the macOS `.app` and `.dmg` artifacts.**
- [ ] **Step 7: Review the diff, commit any final documentation, and prepare the branch for integration.**
