# RADcite Course Reference Merge Implementation Plan

## Task 1: Repository merge transaction

- Add a failing SQLite repository test with a primary reference, duplicate reference, and paragraph citation links.
- Add `merge_reference_entries` to `ReferenceEntryRepository`.
- Consolidate the primary row, reassign citation links, archive duplicate rows, and commit atomically.
- Run focused repository tests.

## Task 2: Desktop command contract

- Add a failing desktop contract for merge validation, metadata fallback, and citation-link preservation.
- Add `MergeCourseReferencesRequest` and command error variants.
- Validate active same-project course references and invoke the repository transaction.
- Run focused desktop tests.

## Task 3: Svelte command and UI

- Add a failing TypeScript command-wrapper test.
- Add the merge wrapper and response typing.
- Add selectable reference rows, primary selection, confirmation, clear selection, and failure-preserving state.
- Wire the parent handler and refresh behavior.
- Extend the style contract and run frontend tests, type-check, and build.

## Task 4: Verification and integration

- Run formatting, clippy, all Rust tests, all frontend checks, and the release build.
- Push a ready PR, wait for CI, merge if green, and fast-forward canonical `main`.
