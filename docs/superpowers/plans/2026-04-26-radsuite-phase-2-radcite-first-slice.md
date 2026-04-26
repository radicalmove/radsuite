# RADsuite Phase 2 RADcite First Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first durable RADcite core slice: document/paragraph/citation domain contracts, citation analysis, and local SQLite persistence for analysed citation documents.

**Architecture:** Keep shared RADcite record shapes in `radsuite-core`, put text/citation heuristics in a new `radsuite-cite` crate, and store analysed documents through `radsuite-db` repositories. This slice deliberately stops before full DOCX/PDF extraction, Crossref/OpenAlex lookup, and desktop upload UI; it creates the tested contracts those features will use.

**Tech Stack:** Rust 2024 workspace, `serde`, `chrono`, `uuid`, `regex`, `sqlx` SQLite migrations, async repository tests with `tokio`.

---

### Task 1: Shared RADcite Domain Contracts

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/radsuite-core/src/domain.rs`
- Modify: `crates/radsuite-core/tests/domain_contracts.rs`

- [ ] **Step 1: Write the failing domain test**

Add a test that constructs a `Document`, `Paragraph`, `Citation`, and `ReferenceEntry` for one project, serializes the document to JSON, and verifies stable UUID-backed IDs and RADcite-specific fields.

- [ ] **Step 2: Run the focused test**

Run: `cargo test -p radsuite-core radcite_document_contracts_are_serializable`

Expected: FAIL because the RADcite domain types do not exist yet.

- [ ] **Step 3: Add minimal domain types**

Add `DocumentFileType`, `DocumentVariant`, `Document`, `Paragraph`, `Citation`, `ReferenceEntryType`, `ApaValidationStatus`, and `ReferenceEntry` to `radsuite-core`. Keep fields aligned with the approved spec and the existing RADcite reference app, but omit module/readings details until a later slice.

- [ ] **Step 4: Re-run the focused test**

Run: `cargo test -p radsuite-core radcite_document_contracts_are_serializable`

Expected: PASS.

### Task 2: RADcite Citation Analyzer Crate

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/radsuite-cite/Cargo.toml`
- Create: `crates/radsuite-cite/src/lib.rs`
- Create: `crates/radsuite-cite/src/analysis.rs`
- Create: `crates/radsuite-cite/tests/analysis_contracts.rs`

- [ ] **Step 1: Write failing analyzer tests**

Cover parenthetical citations, narrative citations, semicolon-separated citations, subsequent `et al.` mentions, and missing-citation flags for statistics or research-claim paragraphs.

- [ ] **Step 2: Run analyzer tests**

Run: `cargo test -p radsuite-cite`

Expected: FAIL before the crate and analyzer implementation exist.

- [ ] **Step 3: Implement minimal analyzer**

Port the core regex heuristics from `/Users/rcd58/citation-checker/app/utils/text_processor.py` into `CitationAnalyzer`, returning `CitationAnalysis` with `citations`, `needs_citation`, and keyword extraction.

- [ ] **Step 4: Re-run analyzer tests**

Run: `cargo test -p radsuite-cite`

Expected: PASS.

### Task 3: Local RADcite Repository

**Files:**
- Modify: `crates/radsuite-db/migrations/0001_foundation.sql`
- Modify: `crates/radsuite-db/src/repositories.rs`
- Modify: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write failing repository roundtrip test**

Insert one analysed document with two paragraphs and detected citations, then list document summaries and reload the full document with paragraphs and citations ordered by paragraph index.

- [ ] **Step 2: Run focused DB test**

Run: `cargo test -p radsuite-db radcite_document_can_be_inserted_and_loaded`

Expected: FAIL because the tables and repository methods do not exist.

- [ ] **Step 3: Add schema and repository methods**

Add `documents`, `paragraphs`, `paragraph_citations`, and `reference_entries` tables. Implement `CitationDocumentRepository` for inserting and loading RADcite document records.

- [ ] **Step 4: Re-run focused DB test**

Run: `cargo test -p radsuite-db radcite_document_can_be_inserted_and_loaded`

Expected: PASS.

### Task 4: Verification

**Files:**
- Workspace verification only.

- [ ] **Step 1: Run Rust workspace tests**

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 2: Run desktop UI build**

Run: `npm run build` from `apps/desktop-ui`

Expected: PASS.

---

The plan review subagent step from the writing-plans workflow was not run while creating this document because the active Codex tool policy only permits subagents when the user explicitly asks for delegation. If a reviewer is desired, ask for a subagent plan review before implementation continues.
