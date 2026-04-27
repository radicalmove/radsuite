# RADcite Functional DOCX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a minimal user-visible RADcite DOCX workflow that runs ingestion from the Svelte desktop app and persists the result locally.

**Architecture:** Extend `radsuite-desktop` state with a SQLite pool and add an async command that orchestrates project setup, DOCX ingestion, and repository persistence. Expose that command through Tauri and add a compact Svelte form/result panel while preserving the existing shell.

**Tech Stack:** Rust 2024, Tauri 2, Svelte 5, `sqlx` SQLite, `radsuite-cite`, `radsuite-db`, `radsuite-core`.

---

### Task 1: Desktop State And Command Contract

**Files:**
- Modify: `crates/radsuite-desktop/Cargo.toml`
- Modify: `crates/radsuite-desktop/src/state.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing desktop command tests**

Add tests that create an in-memory migrated desktop state, generate a minimal `.docx`, call `analyse_docx_path`, and assert returned paragraph/citation/missing-citation counts. Add a second test for empty path rejection.

- [ ] **Step 2: Run focused tests**

Run: `cargo test -p radsuite-desktop analyse_docx`

Expected: FAIL because the command contract does not exist.

- [ ] **Step 3: Add state and command types**

Add `SqlitePool` to `DesktopState`. Add `AnalyseDocxRequest`, `AnalyseDocxResponse`, and `AnalyseDocxError` to `commands.rs`.

- [ ] **Step 4: Implement command orchestration**

Create or reuse a local demo `Project`, ingest the DOCX with `radsuite-cite`, persist with `SqliteCitationDocumentRepository`, and return summary counts.

- [ ] **Step 5: Re-run focused tests**

Run: `cargo test -p radsuite-desktop analyse_docx`

Expected: PASS.

### Task 2: Tauri Bridge

**Files:**
- Modify: `apps/desktop-ui/src-tauri/Cargo.toml`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`

- [ ] **Step 1: Wire async command wrapper**

Expose `analyse_docx_path` through `tauri::generate_handler!`, returning `Result<AnalyseDocxResponse, String>`.

- [ ] **Step 2: Initialize real desktop state**

Use an async setup path that creates the local data directory, opens `radsuite.sqlite3`, runs migrations, and manages `DesktopState`.

- [ ] **Step 3: Run Tauri crate check**

Run: `cargo test -p radsuite-tauri`

Expected: PASS.

### Task 3: Svelte Functional Panel

**Files:**
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/styles.css`

- [ ] **Step 1: Add command types and local state**

Add request/response types, path input state, loading state, result state, and analysis error state.

- [ ] **Step 2: Add RADcite panel UI**

Add a compact panel with a path input, Analyse button, and result/error area. Keep the existing status and engine shell.

- [ ] **Step 3: Wire invoke call**

Call `invoke<AnalyseDocxResponse>("analyse_docx_path", { request })` and render counts.

- [ ] **Step 4: Build frontend**

Run: `npm run build` from `apps/desktop-ui`

Expected: PASS with 0 `svelte-check` diagnostics.

### Task 4: Full Verification And Commit

**Files:**
- Workspace verification only.

- [ ] **Step 1: Format Rust**

Run: `cargo fmt --all`

- [ ] **Step 2: Run focused desktop tests**

Run: `cargo test -p radsuite-desktop`

Expected: PASS.

- [ ] **Step 3: Run frontend build**

Run: `npm run build` from `apps/desktop-ui`

Expected: PASS.

- [ ] **Step 4: Run workspace tests**

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit message: `feat: add RADcite DOCX desktop workflow`
