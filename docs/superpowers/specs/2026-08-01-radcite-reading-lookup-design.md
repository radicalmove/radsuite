# RADcite Module Reading Source Lookup

## Goal

Bring the original RADcite module-level Crossref auto-fill workflow into the Rust desktop app without silently replacing reading data. The feature should help users find missing APA details and links while keeping the existing local-first model and the explicit review workflow used by course-reference lookup.

## User experience

- The Module readings workspace exposes `Find sources` for the selected module when at least one reading is missing an APA citation or URL.
- Clicking it searches only the current module's incomplete readings. Requests run through the existing browser-side Crossref client, one reading at a time, so the feature does not add server or Rust networking requirements.
- A review panel lists each reading, the query used, the best Crossref match when one is found, and a checkbox selected by default for matched results.
- Users can deselect individual matches, open the proposed source, or apply the selected matches. Searching never changes saved data.
- Applying a match updates the APA citation, citation text, URL, and DOI when Crossref provides them. Existing category, lesson code, notes, student notes, and estimated reading time remain intact. A short provenance note is appended without replacing existing notes.
- Results can be partially applied. A failed save remains visible and is reported without preventing other selected matches from being attempted.
- No-result, missing-query, network-error, and save-error states are shown in plain language. The existing readings list refreshes after successful updates.

## Data flow

1. The component filters the selected module's active readings to those missing APA data or a URL.
2. It derives a query from the saved APA citation, citation text, or title.
3. It calls `searchCrossrefWorks` for each query and keeps the first normalized result.
4. It stores the results in component state for review; no Tauri command or database write occurs during search.
5. On explicit apply, it calls the existing `update_module_reading` bridge with the complete current reading payload plus Crossref metadata.

## Error handling

- A failed Crossref request marks only that reading as failed and keeps other results available.
- Empty queries are shown as unavailable rather than sent to Crossref.
- A failed update marks only that reading as unsaved and leaves it selected for another attempt.
- The feature is disabled while a scan or apply operation is running to prevent duplicate requests.

## Testing

- Add component-facing helper tests for incomplete-reading filtering, lookup queries, provenance notes, and selection/application state where practical.
- Extend the style contract to require the user-facing lookup controls and review states.
- Keep the existing Rust update command contract covered by the full desktop test suite.
- Run frontend checks, focused tests, build, Rust format/clippy, and the full workspace test suite before opening the PR.
