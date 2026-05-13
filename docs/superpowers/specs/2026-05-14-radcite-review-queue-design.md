# RADcite Review Queue Design

## Status

Approved in conversation on 2026-05-14 after merging RADcite reference suggestions.

## Context

RADcite can now ingest DOCX files, store course references, suggest likely citation/reference matches, and let the user accept those suggestions. The next workflow gap is visibility: suggested and unlinked citations are only obvious after selecting individual paragraphs.

## Goal

Make the document review workspace act more like a queue by showing citation-linking status at the document level and letting the user jump directly to paragraphs that need reference-linking work.

## Non-Goals

- Do not introduce project/course CRUD.
- Do not change the Local DB schema.
- Do not silently accept suggested matches.
- Do not replace the right citation actions panel.
- Do not build export/report generation in this slice.

## Data Model

Extend the review summary response with derived counts:

- `linked_citation_count`: citations already linked to a course reference.
- `suggested_citation_count`: unlinked citations with at least one suggested reference.
- `unlinked_citation_count`: citations not yet linked to a course reference.

These counts are computed from the existing review response data. No migration is needed.

## UI Direction

In the RADcite Documents workspace:

- add summary cards for linked citations, suggested matches, and unlinked citations
- make those cards filter the paragraph list
- show a small suggested/unlinked indicator in paragraph rows
- keep the right panel as the place where suggestions are accepted

In the right Citation Actions panel:

- rename visible `Verify citation` language to `Mark citations reviewed`
- rename `Verified` badges to `Reviewed`
- keep the underlying `verified` field and Tauri command unchanged for now

## Filters

Add three paragraph filters:

- `linked-citation`: paragraphs with at least one linked citation
- `suggested-citation`: paragraphs with at least one unlinked citation that has suggestions
- `unlinked-citation`: paragraphs with at least one unlinked citation

The existing `all`, `citation-total`, `has-citation`, and `needs-citation` filters remain.

## Testing

Rust:

- assert review summaries include linked, suggested, and unlinked citation counts
- assert accepting a suggestion moves a citation from suggested/unlinked into linked

Frontend:

- assert filter helpers include linked/suggested/unlinked behavior
- assert style contract requires the new queue labels and reviewed wording
- run Svelte type-check/build

## Review Note

The normal brainstorming workflow asks for spec-review subagents. This Codex session only allows subagents when the user explicitly asks for them, so that review step is intentionally skipped.
