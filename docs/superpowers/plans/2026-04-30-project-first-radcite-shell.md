# Project-First RADcite Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first real project-first RADsuite shell with a RADcite Documents review workspace backed by the existing Rust DOCX pipeline.

**Architecture:** Add a richer desktop command that returns analysed document details, not just counts. Replace the current single Svelte screen with a three-zone shell: project sidebar, RADcite document review workspace, and contextual citation actions panel. Keep state local to the app/components for this slice and seed one local project until full project CRUD is ready.

**Tech Stack:** Rust 2024, Tauri 2, Svelte 5, TypeScript, `sqlx` SQLite, `radsuite-cite`, `radsuite-db`, `radsuite-core`.

---

## File Structure

- Modify `crates/radsuite-desktop/src/commands.rs`
  - Add paragraph/citation DTOs and `analyse_docx_for_review`.
  - Reuse existing validation, ingestion, persistence, and count logic.
- Modify `crates/radsuite-desktop/tests/desktop_contracts.rs`
  - Add failing tests for review details.
  - Keep existing summary command tests.
- Modify `apps/desktop-ui/src-tauri/src/main.rs`
  - Expose `analyse_docx_for_review` through Tauri.
- Create `apps/desktop-ui/src/types.ts`
  - Shared frontend types for app status, project nav items, RADcite review responses, filters, and paragraph/citation records.
- Create `apps/desktop-ui/src/components/ProjectSidebar.svelte`
  - Project-first navigation shell with one seeded local project and tool areas.
- Create `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
  - DOCX import/review workspace, summary chips, filters, paragraph list.
- Create `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
  - Context panel for selected paragraph.
- Modify `apps/desktop-ui/src/App.svelte`
  - Compose the shell and own top-level UI state.
- Modify `apps/desktop-ui/src/styles.css`
  - Replace placeholder panel styles with dense project-first app layout.

## Task 1: Rust Review Command Contract

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `crates/radsuite-desktop/tests/desktop_contracts.rs`

- [ ] **Step 1: Write failing review-command test**

Add imports for the new command in `desktop_contracts.rs`:

```rust
use radsuite_desktop::{
    AnalyseDocxError, AnalyseDocxRequest, AppPaths, DesktopState, analyse_docx_for_review,
    analyse_docx_path, get_app_status,
};
```

Add a test after `analyse_docx_path_persists_document_and_returns_summary`:

```rust
#[tokio::test]
async fn analyse_docx_for_review_returns_ordered_paragraphs_and_citations() {
    let state = desktop_state_with_migrated_pool().await;
    let path = write_minimal_docx("desktop-review-analysis.docx");

    let response = analyse_docx_for_review(
        &state,
        AnalyseDocxRequest {
            path: path.to_string_lossy().into_owned(),
            original_filename: Some("review-source.docx".to_string()),
        },
    )
    .await
    .expect("analyse docx for review");

    assert_eq!(response.original_filename, "review-source.docx");
    assert_eq!(response.summary.paragraph_count, 2);
    assert_eq!(response.summary.citation_count, 1);
    assert_eq!(response.summary.cited_paragraph_count, 1);
    assert_eq!(response.summary.missing_citation_count, 1);
    assert_eq!(response.paragraphs.len(), 2);
    assert_eq!(response.paragraphs[0].order_index, 0);
    assert_eq!(response.paragraphs[0].citations.len(), 1);
    assert_eq!(response.paragraphs[0].citations[0].text, "Smith (2020)");
    assert!(!response.paragraphs[0].needs_citation);
    assert_eq!(response.paragraphs[1].order_index, 1);
    assert!(response.paragraphs[1].needs_citation);
}
```

- [ ] **Step 2: Run focused failing test**

Run:

```bash
cargo test -p radsuite-desktop analyse_docx_for_review
```

Expected: FAIL because `analyse_docx_for_review` and review response types do not exist.

- [ ] **Step 3: Add review DTOs and helper mapping**

In `commands.rs`, import citation and paragraph ids:

```rust
use radsuite_core::{Citation, CitationId, DocumentId, Paragraph, ParagraphId, Project, ProjectId, UserId};
```

Add these serialisable types after `AnalyseDocxResponse`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxReviewResponse {
    pub project_id: ProjectId,
    pub project_title: String,
    pub document_id: DocumentId,
    pub original_filename: String,
    pub summary: AnalyseDocxSummary,
    pub paragraphs: Vec<ReviewParagraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyseDocxSummary {
    pub paragraph_count: usize,
    pub citation_count: usize,
    pub cited_paragraph_count: usize,
    pub missing_citation_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewParagraph {
    pub id: ParagraphId,
    pub order_index: i32,
    pub page: Option<i32>,
    pub text: String,
    pub formatted_text: Option<String>,
    pub is_table: bool,
    pub needs_citation: bool,
    pub citations: Vec<ReviewCitation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCitation {
    pub id: CitationId,
    pub text: String,
    pub start: i32,
    pub end: i32,
    pub verified: bool,
}
```

Add helper functions:

```rust
fn build_summary(paragraphs: &[Paragraph], citations: &[Citation]) -> AnalyseDocxSummary {
    let cited_paragraph_count = paragraphs
        .iter()
        .filter(|paragraph| citations.iter().any(|citation| citation.paragraph_id == Some(paragraph.id)))
        .count();
    let missing_citation_count = paragraphs
        .iter()
        .filter(|paragraph| paragraph.needs_citation)
        .count();

    AnalyseDocxSummary {
        paragraph_count: paragraphs.len(),
        citation_count: citations.len(),
        cited_paragraph_count,
        missing_citation_count,
    }
}

fn build_review_paragraphs(paragraphs: Vec<Paragraph>, citations: Vec<Citation>) -> Vec<ReviewParagraph> {
    paragraphs
        .into_iter()
        .map(|paragraph| {
            let paragraph_citations = citations
                .iter()
                .filter(|citation| citation.paragraph_id == Some(paragraph.id))
                .map(|citation| ReviewCitation {
                    id: citation.id,
                    text: citation.text.clone(),
                    start: citation.start,
                    end: citation.end,
                    verified: citation.verified,
                })
                .collect();

            ReviewParagraph {
                id: paragraph.id,
                order_index: paragraph.order_index,
                page: paragraph.page,
                text: paragraph.text,
                formatted_text: paragraph.formatted_text,
                is_table: paragraph.is_table,
                needs_citation: paragraph.needs_citation,
                citations: paragraph_citations,
            }
        })
        .collect()
}
```

- [ ] **Step 4: Add minimal review command implementation**

Refactor `analyse_docx_path` to use `build_summary`, then add:

```rust
pub async fn analyse_docx_for_review(
    state: &DesktopState,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxReviewResponse, AnalyseDocxError> {
    let analysed = analyse_docx(state, request).await?;
    let summary = build_summary(&analysed.paragraphs, &analysed.citations);
    let paragraphs = build_review_paragraphs(analysed.paragraphs, analysed.citations);

    Ok(AnalyseDocxReviewResponse {
        project_id: analysed.project.id,
        project_title: analysed.project.title,
        document_id: analysed.document.id,
        original_filename: analysed.document.original_filename,
        summary,
        paragraphs,
    })
}
```

Use a private helper such as:

```rust
struct DesktopAnalysedDocument {
    project: Project,
    document: radsuite_core::Document,
    paragraphs: Vec<Paragraph>,
    citations: Vec<Citation>,
}
```

This keeps validation/ingestion/persistence shared between summary and review commands.

- [ ] **Step 5: Run focused tests**

Run:

```bash
cargo test -p radsuite-desktop analyse_docx
```

Expected: PASS for summary and review command tests.

## Task 2: Tauri Bridge

**Files:**
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`

- [ ] **Step 1: Add failing bridge compile expectation**

Update imports to include `AnalyseDocxReviewResponse` and add a wrapper:

```rust
use radsuite_desktop::{
    AnalyseDocxRequest, AnalyseDocxResponse, AnalyseDocxReviewResponse, AppStatus, DesktopState,
};

#[tauri::command]
async fn analyse_docx_for_review(
    state: tauri::State<'_, DesktopState>,
    request: AnalyseDocxRequest,
) -> Result<AnalyseDocxReviewResponse, String> {
    radsuite_desktop::analyse_docx_for_review(&state, request)
        .await
        .map_err(|error| error.to_string())
}
```

Register it:

```rust
.invoke_handler(tauri::generate_handler![
    get_app_status,
    analyse_docx_path,
    analyse_docx_for_review
])
```

- [ ] **Step 2: Run Tauri crate test**

Run:

```bash
cargo test -p radsuite-tauri
```

Expected: PASS.

## Task 3: Svelte Component Structure

**Files:**
- Create: `apps/desktop-ui/src/types.ts`
- Create: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
- Create: `apps/desktop-ui/src/components/CitationActionsPanel.svelte`
- Create: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/App.svelte`

- [ ] **Step 1: Create shared TypeScript types**

Create `apps/desktop-ui/src/types.ts` with:

```ts
export type EngineStatus = {
  id: string;
  label: string;
  available: boolean;
  detail: string;
};

export type AppStatus = {
  app_name: string;
  database_ready: boolean;
  sync_configured: boolean;
  engines: EngineStatus[];
};

export type ProjectNavItem = {
  id: string;
  code: string;
  title: string;
  structureMode: "modules" | "weeks";
};

export type ToolArea = "documents" | "references" | "readings" | "exports" | "radcast" | "radtts";
export type ParagraphFilter = "all" | "citation-total" | "has-citation" | "needs-citation";

export type ReviewCitation = {
  id: string;
  text: string;
  start: number;
  end: number;
  verified: boolean;
};

export type ReviewParagraph = {
  id: string;
  order_index: number;
  page: number | null;
  text: string;
  formatted_text: string | null;
  is_table: boolean;
  needs_citation: boolean;
  citations: ReviewCitation[];
};

export type AnalyseDocxSummary = {
  paragraph_count: number;
  citation_count: number;
  cited_paragraph_count: number;
  missing_citation_count: number;
};

export type AnalyseDocxReviewResponse = {
  project_id: string;
  project_title: string;
  document_id: string;
  original_filename: string;
  summary: AnalyseDocxSummary;
  paragraphs: ReviewParagraph[];
};
```

- [ ] **Step 2: Create `ProjectSidebar.svelte`**

Props:

```ts
import type { ProjectNavItem, ToolArea } from "../types";

type Props = {
  projects: ProjectNavItem[];
  selectedProjectId: string;
  activeArea: ToolArea;
  onSelectProject: (projectId: string) => void;
  onSelectArea: (area: ToolArea) => void;
};
```

Render project-first navigation with RADcite areas and disabled future tools. Keep click handlers as buttons, not links.

- [ ] **Step 3: Create `CitationActionsPanel.svelte`**

Props:

```ts
import type { ReviewParagraph } from "../types";

type Props = {
  selectedParagraph: ReviewParagraph | null;
};
```

Render an empty state when no paragraph is selected. For a selected paragraph, show paragraph metadata, full text, citation badges, citation-needed status, and disabled placeholder buttons for Search, Verify, Ignore, and Add citation.

- [ ] **Step 4: Create `RadciteDocumentsWorkspace.svelte`**

Props:

```ts
import { invoke } from "@tauri-apps/api/core";
import type { AnalyseDocxReviewResponse, ParagraphFilter, ReviewParagraph } from "../types";

type Props = {
  activeFilter: ParagraphFilter;
  analysisResult: AnalyseDocxReviewResponse | null;
  selectedParagraphId: string | null;
  onFilterChange: (filter: ParagraphFilter) => void;
  onAnalysisResult: (result: AnalyseDocxReviewResponse | null) => void;
  onSelectParagraph: (paragraphId: string | null) => void;
};
```

The component owns `docxPath`, `analysisLoading`, and `analysisError`. Call:

```ts
invoke<AnalyseDocxReviewResponse>("analyse_docx_for_review", {
  request: { path, original_filename: null },
});
```

Render:

- path import form
- error notice
- empty document state
- summary chips as filter buttons
- filtered paragraph rows

- [ ] **Step 5: Refactor `App.svelte` to compose components**

`App.svelte` should own:

- app status
- seeded project list
- selected project id
- active area
- analysis result
- active filter
- selected paragraph id

Render:

```svelte
<main class="app-shell">
  <ProjectSidebar ... />
  <section class="main-workspace">...</section>
  <CitationActionsPanel selectedParagraph={selectedParagraph} />
</main>
```

Keep non-RADcite active areas as focused placeholders.

- [ ] **Step 6: Run frontend build**

Run:

```bash
npm run build
```

Expected: PASS with 0 `svelte-check` diagnostics.

## Task 4: Layout And Interaction Styling

**Files:**
- Modify: `apps/desktop-ui/src/styles.css`

- [ ] **Step 1: Replace placeholder layout styles**

Define stable three-zone shell:

```css
.app-shell {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr) 340px;
  min-height: 100vh;
}
```

Add responsive behaviour below `980px` by stacking sidebar, workspace, and right panel.

- [ ] **Step 2: Style the project sidebar**

Add classes for:

- `.project-sidebar`
- `.sidebar-header`
- `.project-list`
- `.project-card`
- `.project-button`
- `.tool-list`
- `.tool-area-button`
- `.tool-area-button.is-active`
- `.tool-area-button:disabled`

Keep the sidebar dense and utilitarian.

- [ ] **Step 3: Style the RADcite workspace**

Add classes for:

- `.main-workspace`
- `.workspace-header`
- `.document-import`
- `.summary-strip`
- `.summary-chip`
- `.paragraph-list`
- `.paragraph-row`
- `.paragraph-row.is-selected`
- `.citation-badge`
- `.status-warning`

- [ ] **Step 4: Style the right action panel**

Add classes for:

- `.actions-panel`
- `.actions-empty`
- `.selected-paragraph`
- `.action-stack`
- `.secondary-button`

- [ ] **Step 5: Re-run frontend build**

Run:

```bash
npm run build
```

Expected: PASS with 0 `svelte-check` diagnostics.

## Task 5: Full Verification And Commit

**Files:**
- Workspace verification only.

- [ ] **Step 1: Format Rust**

Run:

```bash
cargo fmt --all
```

- [ ] **Step 2: Run desktop tests**

Run:

```bash
cargo test -p radsuite-desktop
```

Expected: PASS.

- [ ] **Step 3: Run Tauri tests**

Run:

```bash
cargo test -p radsuite-tauri
```

Expected: PASS.

- [ ] **Step 4: Run frontend build**

Run:

```bash
npm run build
```

from `apps/desktop-ui`.

Expected: PASS.

- [ ] **Step 5: Run workspace tests**

Run:

```bash
cargo test --workspace
```

Expected: PASS.

- [ ] **Step 6: Run clippy**

Run:

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Expected: PASS.

- [ ] **Step 7: Commit**

Commit message:

```bash
git commit -m "feat: add project-first RADcite shell"
```

Plan/spec reviewer subagent steps are not included because this Codex session only permits subagents when the user explicitly asks for delegation.
