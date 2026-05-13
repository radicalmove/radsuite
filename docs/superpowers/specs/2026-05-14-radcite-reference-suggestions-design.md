# RADcite Reference Suggestions Design

## Status

Approved in conversation on 2026-05-14 as the next RADcite Phase 2 slice after manual citation-to-reference linking.

## Context

RADcite can now ingest DOCX files, persist paragraph citation reviews, store course references, and manually link a detected citation to a course reference. The remaining workflow gap is that the user has to inspect every detected citation and manually choose the matching reference.

The next step should reduce that friction without pretending citation matching is perfect.

## Goal

Suggest likely course-reference matches for detected RADcite citations, then let the user accept a suggestion explicitly.

## Non-Goals

- Do not silently auto-link citations.
- Do not call external reference APIs.
- Do not build full APA validation.
- Do not attempt fuzzy bibliography parsing beyond the fields RADsuite already stores.
- Do not replace the manual link form; it remains the fallback.

## Matching Approach

Add a deterministic Rust matcher that compares detected citation text with stored course references.

The matcher should extract:

- publication year from the detected citation
- candidate author tokens from the detected citation

It should compare those against:

- `ReferenceEntry.authors`
- `ReferenceEntry.publication_year`
- `ReferenceEntry.apa_citation`
- `ReferenceEntry.citation_text`
- `ReferenceEntry.title` as a weak fallback only

Confidence should stay simple:

- `strong`: author and year both match
- `possible`: year matches and either author/title text overlaps, or author matches without a usable year

If multiple references match, sort strong suggestions before possible suggestions, then by score and label.

## Data Shape

Extend each `ReviewCitation` with a `reference_suggestions` list. Each suggestion should include:

- reference entry id
- label suitable for display
- confidence
- short reason, for example `Author and year match`

This keeps the UI simple and avoids forcing Svelte to duplicate matching logic.

## UX

In the right citation action panel:

- keep the existing detected citation badges
- show a new `Suggested references` block when the selected paragraph has unlinked citations with suggestions
- for each suggestion, show citation text, reference label, confidence, and an `Accept` button
- accepting a suggestion should reuse the existing citation-link command and refresh the review

Manual linking remains available below this block.

## Error Handling

Suggestion generation should be best-effort. A malformed or sparse reference should simply produce no suggestion for that citation. Accepting a suggestion should use the existing persistence error path.

## Testing

Rust:

- test a detected `Smith (2020)` citation gets a strong suggestion for a stored Smith 2020 reference
- test an unmatched citation gets no suggestion
- test the desktop review response includes suggestions before a citation is linked

Frontend:

- test the accept-suggestion command payload reuses `link_radcite_citation_reference`
- update the style contract to require suggestion UI affordances
- build and type-check the Svelte UI

## Review Note

The normal brainstorming workflow asks for spec-review subagents. This Codex session only allows subagents when the user explicitly asks for them, so that review step is intentionally skipped.
