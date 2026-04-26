# RADcite DOCX Ingestion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real RADcite document pipeline: read a DOCX file, extract paragraphs and table text, analyse citations, and persist the analysed records through the existing repository.

**Architecture:** Keep ingestion in `radsuite-cite` so extraction and citation analysis live together, but do not let it write to SQLite directly. Add a small DOCX parser using ZIP + XML, return `AnalysedDocument`, then test persistence separately through `radsuite-db`.

**Tech Stack:** Rust 2024, `zip` for DOCX containers, `quick-xml` for Word XML, existing `radsuite-core` domain records, existing `CitationAnalyzer`, existing `SqliteCitationDocumentRepository`.

---

### File Structure

- Modify `Cargo.toml`: add workspace dependencies for `quick-xml` and `zip`.
- Modify `crates/radsuite-cite/Cargo.toml`: depend on `quick-xml`, `thiserror`, and `zip`.
- Modify `crates/radsuite-cite/src/lib.rs`: export ingestion module.
- Create `crates/radsuite-cite/src/docx.rs`: DOCX request/response types, error type, extraction and analysis pipeline.
- Create `crates/radsuite-cite/tests/docx_ingestion.rs`: generated DOCX fixture tests.
- Modify `crates/radsuite-db/Cargo.toml`: add `radsuite-cite` as a dev-dependency only.
- Modify `crates/radsuite-db/tests/repository_roundtrip.rs`: add ingestion-to-persistence integration coverage.

### Task 1: DOCX Fixture And Public API Contract

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/radsuite-cite/Cargo.toml`
- Modify: `crates/radsuite-cite/src/lib.rs`
- Create: `crates/radsuite-cite/tests/docx_ingestion.rs`

- [ ] **Step 1: Write failing DOCX ingestion test**

Create a generated minimal `.docx` fixture in the test using `zip`. Include:

- one normal paragraph with `(Smith, 2020)`
- one statistics paragraph with no citation
- one hyperlink paragraph whose target URL is stored in `word/_rels/document.xml.rels`
- one table with two non-empty cells

Expected assertions:

- `ingest_docx(DocxIngestionRequest { ... })` exists
- document file type is `Docx`
- paragraphs are ordered from zero
- the hyperlink URL is included in extracted text
- table text is extracted as an `is_table` paragraph
- one citation record is created for `(Smith, 2020)`
- the statistics paragraph has `needs_citation = true`

- [ ] **Step 2: Run test to verify red**

Run: `cargo test -p radsuite-cite docx_ingestion_extracts_and_analyses_paragraphs`

Expected: FAIL because `DocxIngestionRequest` and `ingest_docx` do not exist yet.

- [ ] **Step 3: Add dependency declarations and empty API shell**

Add:

```toml
[workspace.dependencies]
quick-xml = "0.38"
zip = "2"
```

and in `crates/radsuite-cite/Cargo.toml`:

```toml
quick-xml.workspace = true
thiserror.workspace = true
zip.workspace = true
```

Create the module with:

```rust
pub struct DocxIngestionRequest {
    pub project_id: ProjectId,
    pub path: PathBuf,
    pub original_filename: String,
}

pub struct AnalysedDocument {
    pub document: Document,
    pub paragraphs: Vec<Paragraph>,
    pub citations: Vec<Citation>,
}

pub fn ingest_docx(request: DocxIngestionRequest) -> Result<AnalysedDocument, DocxIngestionError>
```

- [ ] **Step 4: Re-run test**

Run: `cargo test -p radsuite-cite docx_ingestion_extracts_and_analyses_paragraphs`

Expected: Still FAIL until extraction is implemented.

### Task 2: DOCX Extraction Implementation

**Files:**
- Modify: `crates/radsuite-cite/src/docx.rs`
- Modify: `crates/radsuite-cite/tests/docx_ingestion.rs`

- [ ] **Step 1: Implement ZIP/XML reading**

Read `word/document.xml` from the DOCX ZIP. If present, also read `word/_rels/document.xml.rels` and map relationship IDs to hyperlink targets.

- [ ] **Step 2: Extract paragraphs**

Use `quick-xml` events to collect text from `w:p`, `w:t`, `w:tab`, and `w:br`. Ignore empty paragraphs. Track order with `order_index`.

- [ ] **Step 3: Handle hyperlinks**

When a paragraph contains `w:hyperlink r:id="..."`, append the target URL if the URL is not already in the paragraph text.

- [ ] **Step 4: Extract tables**

Collect table cell text from `w:tbl` in row/cell order and append one table paragraph marked `is_table = true`.

- [ ] **Step 5: Analyse paragraphs**

For each final paragraph, run `CitationAnalyzer::analyse_paragraph`, set `needs_citation`, and create `Citation` records for detected citations.

- [ ] **Step 6: Re-run focused test**

Run: `cargo test -p radsuite-cite docx_ingestion_extracts_and_analyses_paragraphs`

Expected: PASS.

### Task 3: Error Handling And Edge Cases

**Files:**
- Modify: `crates/radsuite-cite/src/docx.rs`
- Modify: `crates/radsuite-cite/tests/docx_ingestion.rs`

- [ ] **Step 1: Write failing unsupported extension test**

Pass a `.txt` path and assert `DocxIngestionError::UnsupportedExtension`.

- [ ] **Step 2: Write failing missing document XML test**

Create a ZIP without `word/document.xml` and assert `DocxIngestionError::MissingDocumentXml`.

- [ ] **Step 3: Implement error variants**

Use `thiserror` and keep variants suitable for later Tauri command responses.

- [ ] **Step 4: Run cite tests**

Run: `cargo test -p radsuite-cite`

Expected: PASS.

### Task 4: Persistence Integration

**Files:**
- Modify: `crates/radsuite-db/Cargo.toml`
- Modify: `crates/radsuite-db/tests/repository_roundtrip.rs`

- [ ] **Step 1: Write failing DB integration test**

Generate a minimal DOCX fixture, call `ingest_docx`, persist the returned records with `SqliteCitationDocumentRepository::insert_document_analysis`, then reload and assert paragraph/citation counts.

- [ ] **Step 2: Run focused DB test**

Run: `cargo test -p radsuite-db docx_ingestion_output_can_be_persisted`

Expected: FAIL until dev-dependency and fixture helper are wired.

- [ ] **Step 3: Add dev-dependency and test helper**

Add `radsuite-cite = { path = "../radsuite-cite" }` to `crates/radsuite-db/Cargo.toml` dev-dependencies. Keep fixture generation local to the test file or a small helper function.

- [ ] **Step 4: Re-run focused DB test**

Run: `cargo test -p radsuite-db docx_ingestion_output_can_be_persisted`

Expected: PASS.

### Task 5: Verification And Commit

**Files:**
- All changed files.

- [ ] **Step 1: Format**

Run: `cargo fmt --all`

- [ ] **Step 2: Run cite tests**

Run: `cargo test -p radsuite-cite`

Expected: PASS.

- [ ] **Step 3: Run DB tests**

Run: `cargo test -p radsuite-db`

Expected: PASS.

- [ ] **Step 4: Run Clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`

Expected: PASS.

- [ ] **Step 5: Run workspace tests**

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/radsuite-cite crates/radsuite-db docs/superpowers
git commit -m "feat: add RADcite DOCX ingestion"
```

---

The plan review subagent step from the writing-plans workflow was not run because the active user instruction is to proceed with the recommended approach, and subagents are only used when explicitly requested in this Codex session.
