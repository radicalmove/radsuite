<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import {
    filterParagraphs,
    hasLinkedCitation,
    hasSuggestedCitation,
    hasUnlinkedCitation,
  } from "../lib/paragraphFilters";
  import type {
    AnalyseDocxReviewResponse,
    ParagraphFilter,
    ReviewParagraph,
    SavedRadciteReviewSummary,
  } from "../types";

  type DocumentSource = "docx" | "pdf";

  type Props = {
    selectedProjectId: string | null;
    documentSource: DocumentSource;
    docxPath: string;
    activeFilter: ParagraphFilter;
    analysisResult: AnalyseDocxReviewResponse | null;
    savedReviews: SavedRadciteReviewSummary[];
    savedReviewsLoading: boolean;
    savedReviewsError: string | null;
    selectedParagraphId: string | null;
    selectedDocumentId: string | null;
    readingsDocxPath: string;
    onFilterChange: (filter: ParagraphFilter) => void;
    onAnalysisResult: (result: AnalyseDocxReviewResponse | null) => void;
    onDocumentSourceChange: (source: DocumentSource) => void;
    onDocxPathChange: (path: string) => void;
    onOpenReadings: () => void | Promise<void>;
    onLoadSavedReview: (documentId: string) => void | Promise<void>;
    onArchiveDocument: (documentId: string) => void | Promise<void>;
    onRefreshSavedReviews: () => void | Promise<void>;
    onSelectParagraph: (paragraphId: string | null) => void;
  };

  let {
    selectedProjectId,
    documentSource,
    docxPath,
    activeFilter,
    analysisResult,
    savedReviews,
    savedReviewsLoading,
    savedReviewsError,
    selectedParagraphId,
    selectedDocumentId,
    readingsDocxPath,
    onFilterChange,
    onAnalysisResult,
    onDocumentSourceChange,
    onDocxPathChange,
    onOpenReadings,
    onLoadSavedReview,
    onArchiveDocument,
    onRefreshSavedReviews,
    onSelectParagraph,
  }: Props = $props();

  let analysisLoading = $state(false);
  let analysisError = $state<string | null>(null);
  let analysisDisabled = $derived(analysisLoading || docxPath.trim().length === 0);
  let canOpenReadings = $derived(
    documentSource === "docx" && readingsDocxPath.trim().length > 0,
  );
  let sourceLabel = $derived(documentSource === "docx" ? "DOCX" : "PDF");
  let sourceDescription = $derived(documentSource === "docx" ? "Word document" : "PDF document");

  let filteredParagraphs = $derived(
    analysisResult ? filterParagraphs(analysisResult.paragraphs, activeFilter) : [],
  );

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function paragraphPreview(paragraph: ReviewParagraph): string {
    return paragraph.text.length > 360 ? `${paragraph.text.slice(0, 360)}...` : paragraph.text;
  }

  function reviewStats(review: SavedRadciteReviewSummary): string {
    return `${review.paragraph_count} paragraphs · ${review.citation_count} citations · ${review.missing_citation_count} flagged`;
  }

  function handleDocumentPathInput(event: Event) {
    onDocxPathChange((event.currentTarget as HTMLInputElement).value);
  }

  function selectDocumentSource(source: DocumentSource) {
    if (source !== documentSource) {
      analysisError = null;
      onAnalysisResult(null);
      onSelectParagraph(null);
      onDocumentSourceChange(source);
    }
  }

  async function onChooseDocx() {
    analysisError = null;

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: sourceDescription,
            extensions: [documentSource],
          },
        ],
      });

      if (typeof selected === "string") {
        onDocxPathChange(selected);
      } else if (Array.isArray(selected) && typeof selected[0] === "string") {
        onDocxPathChange(selected[0]);
      }
    } catch (reason: unknown) {
      analysisError = `Could not open the ${sourceLabel} picker: ${toErrorMessage(reason)}`;
    }
  }

  async function analyseDocument() {
    const path = docxPath.trim();
    if (!path) {
      analysisError = `Choose a ${sourceLabel} file before running RADcite analysis.`;
      return;
    }

    analysisLoading = true;
    analysisError = null;
    onAnalysisResult(null);
    onSelectParagraph(null);

    try {
      const command = documentSource === "docx" ? "analyse_docx_for_review" : "analyse_pdf_for_review";
      const result = await invoke<AnalyseDocxReviewResponse>(command, {
        request: {
          project_id: selectedProjectId,
          path,
          original_filename: null,
        },
      });
      onAnalysisResult(result);
      onFilterChange("all");
    } catch (reason: unknown) {
      analysisError = toErrorMessage(reason);
    } finally {
      analysisLoading = false;
    }
  }
</script>

<section class="documents-workspace" aria-labelledby="documents-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcite</p>
      <h2 id="documents-heading">Documents</h2>
    </div>
    {#if analysisResult}
      <div class="document-title-block">
        <strong>{analysisResult.original_filename}</strong>
        <span>{analysisResult.project_title}</span>
        {#if canOpenReadings}
          <button
            class="secondary-button compact-button"
            type="button"
            onclick={() => void onOpenReadings()}
          >
            Review readings
          </button>
        {/if}
      </div>
    {/if}
  </div>

  <form
    class="document-import"
    onsubmit={(event) => {
      event.preventDefault();
      void analyseDocument();
    }}
  >
    <div class="form-section-heading">
      <label class="field-label" for="document-path">Import {sourceLabel}</label>
      <div class="import-source-toggle" aria-label="Document import source">
        <button
          type="button"
          class:is-active={documentSource === "docx"}
          aria-pressed={documentSource === "docx"}
          onclick={() => selectDocumentSource("docx")}
        >
          DOCX
        </button>
        <button
          type="button"
          class:is-active={documentSource === "pdf"}
          aria-pressed={documentSource === "pdf"}
          onclick={() => selectDocumentSource("pdf")}
        >
          PDF
        </button>
      </div>
    </div>
    <div class="path-row">
      <input
        id="document-path"
        class="path-input"
        type="text"
        value={docxPath}
        oninput={handleDocumentPathInput}
        placeholder={`/Users/name/Documents/source.${documentSource}`}
        autocomplete="off"
      />
      <button
        class="secondary-button choose-docx-button"
        type="button"
        disabled={analysisLoading}
        onclick={() => void onChooseDocx()}
      >
        Choose {sourceLabel}
      </button>
      <button class="primary-button" type="submit" disabled={analysisDisabled}>
        {analysisLoading ? "Analysing" : "Analyse"}
      </button>
    </div>
  </form>

  {#if analysisError}
    <div class="notice analysis-notice">{analysisError}</div>
  {/if}

  <section class="saved-reviews" aria-labelledby="saved-reviews-heading">
    <div class="saved-reviews-heading">
      <div>
        <p class="eyebrow">Local DB</p>
        <h3 id="saved-reviews-heading">Saved reviews</h3>
      </div>
      <button
        class="secondary-button compact-button"
        type="button"
        disabled={savedReviewsLoading}
        onclick={() => void onRefreshSavedReviews()}
      >
        Refresh
      </button>
    </div>

    {#if savedReviewsError}
      <div class="notice saved-reviews-notice">{savedReviewsError}</div>
    {:else if savedReviewsLoading}
      <div class="saved-reviews-empty">Loading saved reviews</div>
    {:else if savedReviews.length}
      <div class="saved-reviews-list" aria-label="Saved RADcite reviews">
        {#each savedReviews as review (review.document_id)}
          <div class="saved-review-row" class:is-active={selectedDocumentId === review.document_id}>
            <button
              class="saved-review-open"
              type="button"
              onclick={() => void onLoadSavedReview(review.document_id)}
            >
              <span>
                <strong>{review.original_filename}</strong>
                <small>{reviewStats(review)}</small>
              </span>
              <span class="saved-review-action">Open</span>
            </button>
            <button
              class="secondary-button compact-button danger-button"
              type="button"
              onclick={() => void onArchiveDocument(review.document_id)}
            >
              Archive
            </button>
          </div>
        {/each}
      </div>
    {:else}
      <div class="saved-reviews-empty">No saved RADcite reviews yet.</div>
    {/if}
  </section>

  {#if analysisResult}
    <div class="summary-strip" aria-label="Document summary">
      <button
        class="summary-chip"
        class:is-active={activeFilter === "all"}
        data-filter="all"
        type="button"
        onclick={() => onFilterChange("all")}
      >
        <span>Total paragraphs</span>
        <strong>{analysisResult.summary.paragraph_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "citation-total"}
        data-filter="citation-total"
        type="button"
        onclick={() => onFilterChange("citation-total")}
      >
        <span>Total citations</span>
        <strong>{analysisResult.summary.citation_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "has-citation"}
        data-filter="has-citation"
        type="button"
        onclick={() => onFilterChange("has-citation")}
      >
        <span>With citations</span>
        <strong>{analysisResult.summary.cited_paragraph_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "needs-citation"}
        data-filter="needs-citation"
        type="button"
        onclick={() => onFilterChange("needs-citation")}
      >
        <span>Needs citations</span>
        <strong>{analysisResult.summary.missing_citation_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "linked-citation"}
        data-filter="linked-citation"
        type="button"
        onclick={() => onFilterChange("linked-citation")}
      >
        <span>Linked citations</span>
        <strong>{analysisResult.summary.linked_citation_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "suggested-citation"}
        data-filter="suggested-citation"
        type="button"
        onclick={() => onFilterChange("suggested-citation")}
      >
        <span>Suggested matches</span>
        <strong>{analysisResult.summary.suggested_citation_count}</strong>
      </button>
      <button
        class="summary-chip"
        class:is-active={activeFilter === "unlinked-citation"}
        data-filter="unlinked-citation"
        type="button"
        onclick={() => onFilterChange("unlinked-citation")}
      >
        <span>Unlinked citations</span>
        <strong>{analysisResult.summary.unlinked_citation_count}</strong>
      </button>
    </div>

    <div class="paragraph-list" aria-label="Analysed paragraphs">
      {#each filteredParagraphs as paragraph (paragraph.id)}
        <button
          class="paragraph-row"
          class:is-selected={selectedParagraphId === paragraph.id}
          type="button"
          onclick={() => onSelectParagraph(paragraph.id)}
        >
          <span class="paragraph-index">#{paragraph.order_index + 1}</span>
          <span class="paragraph-body">
            <span class="paragraph-meta">
              {#if paragraph.page}
                <span>Page {paragraph.page}</span>
              {/if}
              {#if paragraph.is_table}
                <span>Table</span>
              {/if}
              {#if paragraph.needs_citation}
                <span class="status-warning">Needs citation</span>
              {/if}
              {#if hasSuggestedCitation(paragraph)}
                <span class="queue-status is-suggested">Suggested match</span>
              {/if}
              {#if hasUnlinkedCitation(paragraph)}
                <span class="queue-status is-unlinked">Unlinked citation</span>
              {/if}
              {#if hasLinkedCitation(paragraph)}
                <span class="queue-status is-linked">Linked</span>
              {/if}
            </span>
            <span class="paragraph-preview">{paragraphPreview(paragraph)}</span>
            {#if paragraph.citations.length}
              <span class="citation-badge-list">
                {#each paragraph.citations as citation (citation.id)}
                  <span class="citation-badge">{citation.text}</span>
                {/each}
              </span>
            {/if}
          </span>
        </button>
      {/each}
    </div>
  {:else if !analysisLoading}
    <div class="document-empty">
      <strong>No document loaded</strong>
      <span>Import a {sourceLabel} to start reviewing paragraphs and citation status.</span>
    </div>
  {/if}
</section>
