# Plan: RADcite Module Reading Source Lookup

## Steps

1. Add small, pure frontend helpers and tests for identifying incomplete readings, selecting lookup queries, and preserving existing notes when adding Crossref provenance.
2. Add a reviewable Crossref lookup panel to `RadciteReadingsWorkspace.svelte`, including per-reading states, result selection, partial failure handling, and explicit apply controls.
3. Thread a boolean success result through the existing Svelte-to-Tauri update callback so bulk application can report per-reading save failures accurately.
4. Add focused styles and extend the style contract for the new reading lookup workflow.
5. Run focused frontend and Rust tests, then full format, clippy, tests, type checks, style checks, and build.
6. Commit, push, open a ready PR, wait for CI, merge, and fast-forward the canonical `main` worktree.

## Acceptance criteria

- The selected module can search Crossref for incomplete readings from the Module readings screen.
- Search results are reviewable and never mutate data until the user applies them.
- Applying selected results preserves unrelated reading fields and reports partial failures.
- Required/optional labels and existing reading import/edit workflows continue to work.
- Local verification and GitHub CI are green.
