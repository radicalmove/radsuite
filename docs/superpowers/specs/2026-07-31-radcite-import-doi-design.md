# RADcite Imported DOI Preservation Design

## Context

RADsuite stores DOI values separately for manually entered module readings, but the DOCX, CSV, and SCORM PDF import candidates currently expose only citation text and URL. An imported DOI can therefore be treated as an ordinary URL or disappear when the candidate is saved.

## Goal

Preserve DOI metadata through the existing review-before-save import workflow without changing the database schema or bypassing user review.

## Design

- Add an optional canonical `doi` field to the shared `ReadingImportCandidate` model and all desktop/frontend preview and save contracts.
- Extract DOI values from DOI URLs, `doi:` labels, and bare DOI strings while leaving the original URL in the URL field when one exists.
- Apply the same extraction to DOCX, CSV, and PDF candidates so all import sources behave consistently.
- Show DOI as an editable field in each import-preview row.
- Persist the reviewed DOI through `save_module_readings_import` into the existing `ReferenceEntry.doi` field.
- Keep existing URLs unchanged; exports continue to prefer an explicit URL and fall back to the stored DOI when no URL exists.
- Preserve current duplicate handling and required-over-optional precedence.

## Error Handling

Malformed or absent DOI text remains `None`; it does not make an otherwise valid reading candidate fail. DOI extraction is conservative and strips only known prefixes and trailing citation punctuation.

## Testing

- Rust extractor tests cover DOCX, CSV, and PDF DOI forms.
- Desktop contract tests prove preview exposes DOI and save persists it.
- Frontend command tests prove DOI is trimmed and included in save payloads.
- Existing workspace tests, formatting, lint, and production build remain required.
