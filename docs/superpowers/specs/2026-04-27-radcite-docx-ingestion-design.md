# RADcite DOCX Ingestion Design

## Status

Approved in conversation on 2026-04-27 as the next Phase 2 slice after the merged RADcite first slice.

## Context

The first Phase 2 RADcite slice added shared RADcite domain records, citation analysis, SQLite tables, and repository roundtrip coverage. It deliberately stopped before reading real DOCX/PDF files.

The old RADcite implementation extracts DOCX content in `/Users/rcd58/citation-checker/app/services/document_service.py`. Its useful behaviours for this slice are:

- Read `.docx` paragraph text.
- Preserve paragraph order.
- Extract enough formatting context to append hyperlink targets for citation/reference detection.
- Detect explicit page breaks where available.
- Extract table text as additional table paragraphs.
- Merge URL-only paragraphs into preceding reference-like paragraphs.
- Run citation detection and missing-citation heuristics on extracted text.

## Goal

Add a Rust DOCX ingestion path that turns a DOCX file into analysed RADcite records ready for local persistence.

## Non-Goals

- PDF extraction.
- Desktop file-picker or upload UI.
- Crossref/OpenAlex lookup.
- APA reference validation.
- Manual citation-reference linking.
- Full-fidelity DOCX rendering.

## Design

Create a document ingestion layer inside `radsuite-cite`.

The new ingestion API should accept a DOCX file path and return an analysed document payload containing:

- `Document`
- ordered `Paragraph` records
- detected `Citation` records

The ingestion layer should use the existing `CitationAnalyzer` for citation detection and missing-citation flags. It should not duplicate citation heuristics.

The public API should be small and testable, for example:

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
```

Implementation should rely on a Rust DOCX/ZIP/XML approach rather than shelling out to Python. The first implementation only needs stable plain-text extraction, not perfect Word layout fidelity.

## Extraction Behaviour

For normal paragraphs:

- Preserve document order.
- Ignore empty paragraphs.
- Store plain text in `Paragraph.text`.
- Store minimal formatted text only if it can be extracted without adding fragile complexity.
- Use `order_index` starting at zero.
- Set `page` only when explicit page-break markers are present; otherwise leave it unset.

For hyperlinks:

- Include visible link text.
- Append relationship targets when the target URL is not already present in the paragraph text.

For tables:

- Extract non-empty cell text in row order.
- Join cell text into one table paragraph per table.
- Mark those paragraphs with `is_table = true`.

For URL-only paragraphs:

- If a paragraph contains only a URL and the previous paragraph looks reference-like, merge the URL into the previous paragraph.

For analysis:

- Run `CitationAnalyzer::analyse_paragraph` on each final paragraph.
- Set `Paragraph.needs_citation` from the analysis result.
- Create `Citation` records for each detected citation, preserving text and position.

## Persistence

The ingestion layer should not write directly to SQLite. Tests can pass its output to `SqliteCitationDocumentRepository::insert_document_analysis`. This keeps extraction, analysis, and persistence independently testable.

## Error Handling

Use a cite-specific error type for:

- unsupported file extension
- unreadable file
- invalid DOCX ZIP structure
- missing `word/document.xml`
- invalid XML or relationship data

Errors should be suitable for later Tauri command responses, but this slice does not expose them through the UI yet.

## Testing

Add tests in `radsuite-cite` with generated minimal DOCX fixtures. The tests should verify:

- paragraph extraction and ordering
- hyperlink URL inclusion
- table extraction
- citation detection integration
- missing-citation flags

Add a DB integration test that ingests a generated DOCX and persists the returned records through the existing citation document repository.

Run:

- `cargo test -p radsuite-cite`
- `cargo test -p radsuite-db`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
