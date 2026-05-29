# RADcite Project Context And CSV Readings Import Design

## Goal

Make RADcite operations run against the selected course/project instead of the hardcoded CRJU150 functional-testing project, and add a CSV readings import path for real `course_readings.csv` inventories from `course-output-system`.

## Scope

This slice is limited to the local desktop RADcite workflow. It does not add server sync, account-level project membership, or full project administration. It adds the minimum local project management needed to select/create course projects and have documents, references, modules, readings, and exports use that selected project consistently.

## Backend Design

The desktop command layer will expose local project commands:

- `list_radcite_projects`
- `create_radcite_project`

`list_radcite_projects` will ensure the existing fallback project exists, then return all local projects. The fallback remains `CRJU150 / RADcite Functional Testing` so existing data and tests continue to work.

RADcite commands that create or list project-owned data will accept an optional `project_id`. When omitted, they will keep the current fallback behavior. When supplied, they will load that project and return a clear missing-project error if it does not exist.

Project-scoped command requests:

- DOCX analysis uses `AnalyseDocxRequest.project_id`.
- Saved review listing can filter by project.
- Course reference list/add/export uses `project_id`.
- Module list/add uses `project_id`.
- Existing module-reading save/update/export commands continue to derive project context from `module_id`.

## CSV Readings Import

A new deterministic CSV extractor will live in `radsuite-cite`. It will parse headered CSV files with the real `course_readings.csv` shape:

- `section_seq`
- `section_title`
- `week`
- `citation`
- `talis_article_id`

The extractor will also tolerate common aliases such as `module`, `module_title`, `lesson`, `reading`, `reference`, and `reading_category`. The candidate mapping will be:

- `citation` or equivalent becomes `apa_citation`.
- `week` becomes `lesson_code`.
- `section_title` becomes `module_title`.
- numeric `week` or `section_seq` becomes `module_order`.
- missing category defaults to `compulsory`.
- `talis_article_id` is not saved directly in this slice because the existing candidate contract has no source-id field.

Desktop adds `preview_module_readings_csv_import`, returning the same candidate shape as DOCX preview. Saving still uses the existing `save_module_readings_import`, so review-before-save behavior stays consistent.

## Frontend Design

The project sidebar becomes data-backed:

- on app start, load local RADcite projects;
- keep the selected project ID in app state;
- allow creating a compact local course project with code and title;
- switching project clears project-specific review state and reloads references/modules/saved reviews for that project.

The Readings import panel gains a DOCX/CSV source selector. CSV preview uses the same editable candidate table and save-selected action as DOCX preview.

The exported filenames should reflect the selected project code once commands receive `project_id`, e.g. `crju201-course-references.html` instead of `crju150-course-references.html`.

## Error Handling

Project-scoped commands return a missing-project error when a supplied `project_id` cannot be loaded. CSV preview returns empty-path, parse, and missing-citation errors with user-facing messages.

UI errors continue to appear in the existing notice areas. Switching projects clears stale export/review state to avoid showing data from the previous course.

## Testing

Backend contract tests will cover:

- creating/listing local RADcite projects;
- analysing a DOCX into a specified project;
- listing references/modules for separate projects without leakage;
- exporting course references with the selected project code;
- previewing real-shaped CSV reading candidates;
- saving CSV-preview candidates into a module.

Frontend tests will cover:

- project command payloads;
- project IDs passed through reading/reference/export helpers;
- CSV preview helper payload trimming.

Style contract will require the project commands and CSV import labels/hooks.

## Non-Goals

- No cloud/server project management changes.
- No CSV save-by-source-id or Talis integration yet.
- No migration of existing CRJU150 local data into newly created projects.
