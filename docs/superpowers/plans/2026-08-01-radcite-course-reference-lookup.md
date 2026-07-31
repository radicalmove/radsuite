# RADcite Course Reference Lookup Implementation Plan

**Goal:** Add explicit Crossref lookup and apply-selected-result behavior to saved project-level course references.

## Tasks

1. Add a failing desktop contract test for updating optional citation text and URL metadata while preserving omitted fields.
2. Add a failing frontend wrapper test for the metadata-aware update payload.
3. Extend the Rust update request, command, and Tauri bridge with optional `citation_text` and `url` fields.
4. Add the `Find source` panel to `RadciteReferencesWorkspace.svelte`, reusing `searchCrossrefWorks` and its result type.
5. Add loading, empty, error, saving, and success states with selection-safe refresh behavior.
6. Extend the style contract and CSS for lookup controls and result cards.
7. Run focused Rust/frontend checks, then the full Rust workspace suite, frontend check/test/build, and style contract.
8. Push a ready PR, wait for both CI jobs, and merge after green verification.
