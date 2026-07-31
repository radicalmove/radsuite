# RADcite PDF Document Review Design

## Status

Approved under the standing RADsuite migration instruction to continue with the next logical parity slice.

## Context

The RADcite Documents workspace currently analyses DOCX files for paragraph-level citation review. PDF extraction already exists for module-reading imports, including the native text parser used for SCORM reading PDFs. This leaves a gap for users whose source document is a PDF: they cannot send it through the same review queue, citation actions, saved-review list, or project-scoped Local DB workflow.

## Goal

Allow a user to choose either DOCX or PDF in Documents, analyse the selected file, and receive the same paragraph/citation review response and persisted analysis currently available for DOCX.

## Design

### Shared citation analysis

Reuse the existing PDF text extraction path and the existing `CitationAnalyzer`. Extracted PDF text lines become ordered paragraphs, and the shared paragraph-analysis routine creates citations and missing-citation flags. PDF page reconstruction and pixel-perfect document rendering remain out of scope for this slice; page numbers will remain unset when the text parser cannot provide them.

### Desktop command surface

Add a project-scoped `analyse_pdf_for_review` command with the same request shape and response shape as `analyse_docx_for_review`. The command persists the PDF document, paragraphs, and citations through the existing citation-document repository, so saved reviews and review actions work without a second persistence model.

The existing DOCX commands remain available for compatibility. PDF-specific validation errors identify an empty path, missing filename, unsupported extension, extraction failure, missing project, or database failure in the same way as the existing DOCX command.

### Documents workspace

Add a compact DOCX/PDF source selector to the Documents import form. The picker filter, input label, placeholder, validation message, and analysis command follow the selected source. The existing review queue, summary filters, saved reviews, and citation action panel remain shared. The automatic “Review readings” shortcut is shown only after DOCX analysis because that shortcut is specifically for the DOCX reading-list importer.

### Scope boundary

This slice does not change module-reading extraction, reference imports, citation suggestion logic, PDF visual rendering, or RADcast/RADTTS engines. It also does not infer PDF page numbers from layout; that can be added later if the review experience requires page-aware navigation.

## Testing

- Core ingestion test: a minimal PDF produces a PDF document, ordered paragraphs, a detected citation, and a missing-citation flag.
- Core validation test: non-PDF input is rejected.
- Desktop contract test: PDF review analysis persists to the Local DB and returns the same review summary shape as DOCX.
- UI verification: existing component tests plus type-check and production build confirm the source selector and command payload compile cleanly.
- Full workspace tests, formatting, clippy, and packaged desktop rebuild remain required before merge.

