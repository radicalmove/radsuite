<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { invoke } from "@tauri-apps/api/core";
  import type {
    AnalyseDocxReviewResponse,
    ParagraphFilter,
    ReviewParagraph,
    SavedRadciteReviewSummary,
  } from "../types";

  type Props = {
    activeFilter: ParagraphFilter;
    analysisResult: AnalyseDocxReviewResponse | null;
    savedReviews: SavedRadciteReviewSummary[];
    savedReviewsLoading: boolean;
    savedReviewsError: string | null;
    selectedParagraphId: string | null;
    selectedDocumentId: string | null;
    onFilterChange: (filter: ParagraphFilter) => void;
    onAnalysisResult: (result: AnalyseDocxReviewResponse | null) => void;
    onLoadSavedReview: (documentId: string) => void | Promise<void>;
    onRefreshSavedReviews: () => void | Promise<void>;
    onSelectParagraph: (paragraphId: string | null) => void;
  };

  let {
    activeFilter,
    analysisResult,
    savedReviews,
    savedReviewsLoading,
    savedReviewsError,
    selectedParagraphId,
    selectedDocumentId,
    onFilterChange,
    onAnalysisResult,
    onLoadSavedReview,
    onRefreshSavedReviews,
    onSelectParagraph,
  }: Props = $props();

  let docxPath = $state("");
  let analysisLoading = $state(false);
  let analysisError = $state<string | null>(null);
  let analysisDisabled = $derived(analysisLoading || docxPath.trim().length === 0);

  let filteredParagraphs = $derived(
    analysisResult
      ? analysisResult.paragraphs.filter((paragraph) => {
          if (activeFilter === "citation-total") {
            return paragraph.citations.length > 0;
          }
          if (activeFilter === "has-citation") {
            return paragraph.citations.length > 0;
          }
          if (activeFilter === "needs-citation") {
            return paragraph.needs_citation;
          }
          return true;
        })
      : [],
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

  async function onChooseDocx() {
    analysisError = null;

    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "Word documents",
            extensions: ["docx"],
          },
        ],
      });

      if (typeof selected === "string") {
        docxPath = selected;
      } else if (Array.isArray(selected) && typeof selected[0] === "string") {
        docxPath = selected[0];
      }
    } catch (reason: unknown) {
      analysisError = `Could not open the DOCX picker: ${toErrorMessage(reason)}`;
    }
  }

  async function analyseDocx() {
    const path = docxPath.trim();
    if (!path) {
      analysisError = "Choose a DOCX file before running RADcite analysis.";
      return;
    }

    analysisLoading = true;
    analysisError = null;
    onAnalysisResult(null);
    onSelectParagraph(null);

    try {
      const result = await invoke<AnalyseDocxReviewResponse>("analyse_docx_for_review", {
        request: {
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
      </div>
    {/if}
  </div>

  <form
    class="document-import"
    onsubmit={(event) => {
      event.preventDefault();
      void analyseDocx();
    }}
  >
    <label class="field-label" for="docx-path">Import DOCX</label>
    <div class="path-row">
      <input
        id="docx-path"
        class="path-input"
        type="text"
        bind:value={docxPath}
        placeholder="/Users/name/Documents/source.docx"
        autocomplete="off"
      />
      <button
        class="secondary-button choose-docx-button"
        type="button"
        disabled={analysisLoading}
        onclick={() => void onChooseDocx()}
      >
        Choose DOCX
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
          <button
            class="saved-review-row"
            class:is-active={selectedDocumentId === review.document_id}
            type="button"
            onclick={() => void onLoadSavedReview(review.document_id)}
          >
            <span>
              <strong>{review.original_filename}</strong>
              <small>{reviewStats(review)}</small>
            </span>
            <span class="saved-review-action">Open</span>
          </button>
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
      <span>Import a DOCX to start reviewing paragraphs and citation status.</span>
    </div>
  {/if}
</section>
