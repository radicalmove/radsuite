# RADcite Course Reference Lookup Design

## Goal

Give users a deliberate way to improve a saved RADcite course reference by searching Crossref and choosing a metadata result. This restores the original RADcite reference-lookup workflow without introducing a new Rust network client or silently overwriting local data.

## Scope

The Course References workspace gains a per-reference `Find source` action. Opening it shows a prefilled search query, a `Search Crossref` command, and candidate results. Selecting `Use this result` updates only the chosen reference.

The first version covers project-level course references (`reference_type = reference`). Module-reading bulk lookup remains a separate slice because it needs progress reporting, partial-failure handling, and a clear apply policy across many records.

## Existing Patterns

- `apps/desktop-ui/src/lib/sourceSearch.ts` already formats Crossref results and has tested API/error handling.
- Citation actions already use direct browser-side Crossref requests, so this slice does not add a server dependency or a new Rust HTTP library.
- Rust remains the source of truth for persistence. The UI sends the selected result through the existing course-reference update command.

## User Flow

1. The user opens Course References and chooses `Find source` on a saved reference.
2. RADsuite pre-fills the search field with the current APA text, or the best available citation text.
3. The user runs `Search Crossref` and sees compact candidate cards with APA text, source metadata, DOI, and URL.
4. The user chooses `Use this result`.
5. RADsuite saves the chosen APA citation, citation text, URL, and a short provenance note. Existing fields are replaced only by the selected result; a failed request leaves the reference unchanged.
6. The panel reports success and closes. Search errors remain visible and do not alter saved data.

## Data Contract

Extend `UpdateCourseReferenceRequest` with optional `citation_text` and `url` fields. Existing manual edit callers omit them and retain current values. The selected Crossref result supplies both fields, with the APA citation as the citation text and the DOI URL preferred when Crossref does not provide a URL.

The frontend wrapper accepts the optional metadata fields. No schema migration is needed because the existing `reference_entries` columns already store them.

## UI Structure

- Each reference row gets a `Find source` secondary action.
- Only one lookup panel is open at a time to keep the list scannable.
- The panel uses the existing workspace controls and result-card styling, with explicit loading, no-result, error, and saving states.
- Results are not applied automatically. `Use this result` is the only mutating lookup action.
- Search text and transient result state are cleared when the panel is closed or a different reference is opened.

## Error Handling

- Empty query: disable search and show no network request.
- Crossref failure: show a user-facing error and preserve the saved reference.
- No candidates: show `No matching sources found.` and preserve the saved reference.
- Save failure: keep the lookup panel and result selection available for retry.
- Successful save: refresh the reference list so exports and later lookups use the persisted values.

## Testing

- Rust command contract verifies optional metadata is persisted while omitted fields retain existing values.
- Frontend command test verifies the selected Crossref metadata is sent through the Tauri bridge.
- Component/source-search tests cover opening a lookup, loading/error/no-result states, and applying a selected result where practical.
- Existing Crossref client tests remain the network-format contract; no live network test is added.
- Full Rust workspace, Svelte check, frontend tests, production build, and style contract remain required.

## Non-Goals

- Automatic bulk lookup for module readings.
- Automatic overwriting of references based on the first search result.
- New server-side or Rust-side network infrastructure.
