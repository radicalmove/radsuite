<script lang="ts">
  import {
    buildCrossrefSearchUrl,
    searchCrossrefWorks,
    suggestedSourceSearchQuery,
    type CrossrefSourceResult,
  } from "../lib/sourceSearch";
  import type {
    CourseReferenceSummary,
    ReviewCitationReferenceSuggestion,
    ReviewParagraph,
  } from "../types";

  type SuggestedCitationLink = {
    citationId: string;
    citationText: string;
    suggestion: ReviewCitationReferenceSuggestion;
  };

  type Props = {
    selectedParagraph: ReviewParagraph | null;
    courseReferences: CourseReferenceSummary[];
    onMarkResolved: (paragraphId: string) => void | Promise<void>;
    onAddManualCitation: (paragraphId: string, citationText: string) => void | Promise<void>;
    onAddCourseReference: (
      apaCitation: string,
      notes: string | null,
    ) => CourseReferenceSummary | null | void | Promise<CourseReferenceSummary | null | void>;
    onVerifyCitation: (paragraphId: string) => void | Promise<void>;
    onLinkCitation: (citationId: string, referenceEntryId: string) => void | Promise<void>;
  };

  let {
    selectedParagraph,
    courseReferences,
    onMarkResolved,
    onAddManualCitation,
    onAddCourseReference,
    onVerifyCitation,
    onLinkCitation,
  }: Props = $props();

  let manualCitationText = $state("");
  let selectedCitationId = $state("");
  let selectedReferenceId = $state("");
  let sourceSearchOpen = $state(false);
  let sourceSearchQuery = $state("");
  let sourceSearchParagraphId = $state<string | null>(null);
  let sourceSearchResultsQuery = $state("");
  let sourceSearchResults = $state<CrossrefSourceResult[]>([]);
  let sourceSearchLoading = $state(false);
  let sourceSearchError = $state<string | null>(null);
  let sourceReferenceSavingKey = $state<string | null>(null);
  let sourceReferenceStatus = $state<string | null>(null);
  let manualCitationDisabled = $derived(
    !selectedParagraph || manualCitationText.trim().length === 0,
  );
  let verifyDisabled = $derived(
    !selectedParagraph ||
      selectedParagraph.citations.length === 0 ||
      selectedParagraph.citations.every((citation) => citation.verified),
  );
  let linkDisabled = $derived(
    !selectedParagraph ||
      selectedParagraph.citations.length === 0 ||
      courseReferences.length === 0 ||
      selectedCitationId.length === 0 ||
      selectedReferenceId.length === 0,
  );
  let suggestedCitationLinks = $derived<SuggestedCitationLink[]>(
    selectedParagraph?.citations.flatMap((citation) => {
      if (citation.reference_entry_id) {
        return [];
      }

      return citation.reference_suggestions.map((suggestion) => ({
        citationId: citation.id,
        citationText: citation.text,
        suggestion,
      }));
    }) ?? [],
  );
  let sourceSearchSuggestion = $derived(
    selectedParagraph ? suggestedSourceSearchQuery(selectedParagraph) : null,
  );
  let sourceSearchUrl = $derived(
    sourceSearchQuery.trim() ? buildCrossrefSearchUrl(sourceSearchQuery) : null,
  );
  let sourceSearchDisabled = $derived(sourceSearchLoading || sourceSearchQuery.trim().length === 0);

  $effect(() => {
    if (!selectedParagraph?.citations.some((citation) => citation.id === selectedCitationId)) {
      selectedCitationId = "";
    }
    if (!courseReferences.some((reference) => reference.id === selectedReferenceId)) {
      selectedReferenceId = "";
    }
  });

  $effect(() => {
    const paragraphId = selectedParagraph?.id ?? null;
    if (paragraphId !== sourceSearchParagraphId) {
      sourceSearchParagraphId = paragraphId;
      sourceSearchOpen = false;
      sourceSearchQuery = sourceSearchSuggestion?.query ?? "";
      sourceSearchResultsQuery = "";
      sourceSearchResults = [];
      sourceSearchError = null;
      sourceReferenceSavingKey = null;
      sourceReferenceStatus = null;
      return;
    }

    if (
      !sourceSearchOpen &&
      sourceSearchSuggestion &&
      sourceSearchQuery !== sourceSearchSuggestion.query
    ) {
      sourceSearchQuery = sourceSearchSuggestion.query;
    }
  });

  function submitManualCitation() {
    if (!selectedParagraph || manualCitationDisabled) {
      return;
    }

    void onAddManualCitation(selectedParagraph.id, manualCitationText);
    manualCitationText = "";
  }

  function submitCitationLink() {
    if (linkDisabled) {
      return;
    }

    void onLinkCitation(selectedCitationId, selectedReferenceId);
  }

  function toggleSourceSearch() {
    if (!sourceSearchSuggestion) {
      return;
    }

    if (!sourceSearchQuery.trim()) {
      sourceSearchQuery = sourceSearchSuggestion.query;
    }
    const nextOpen = !sourceSearchOpen;
    sourceSearchOpen = nextOpen;
    if (nextOpen && sourceSearchResultsQuery !== sourceSearchQuery.trim()) {
      void runSourceSearch();
    }
  }

  async function runSourceSearch() {
    const query = sourceSearchQuery.trim();
    if (!query) {
      return;
    }

    sourceSearchLoading = true;
    sourceSearchError = null;
    sourceReferenceStatus = null;
    sourceSearchResults = [];

    try {
      sourceSearchResults = await searchCrossrefWorks(query);
      sourceSearchResultsQuery = query;
    } catch (reason: unknown) {
      sourceSearchError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      sourceSearchLoading = false;
    }
  }

  async function addSourceReference(result: CrossrefSourceResult) {
    const key = sourceResultKey(result);
    sourceReferenceSavingKey = key;
    sourceReferenceStatus = null;

    try {
      const saved = await onAddCourseReference(
        result.apaCitation,
        result.doi
          ? `Imported from Crossref search. DOI: ${result.doi}`
          : "Imported from Crossref search.",
      );
      sourceReferenceStatus = saved
        ? "Reference saved to Course References."
        : "Could not save reference.";
    } catch (reason: unknown) {
      sourceReferenceStatus = reason instanceof Error ? reason.message : String(reason);
    } finally {
      sourceReferenceSavingKey = null;
    }
  }

  function referenceLabel(reference: CourseReferenceSummary): string {
    return (
      reference.apa_citation ?? reference.citation_text ?? reference.title ?? "Untitled reference"
    );
  }

  function linkedReferenceLabel(referenceEntryId: string): string {
    const linkedReference = courseReferences.find((reference) => reference.id === referenceEntryId);
    return linkedReference ? referenceLabel(linkedReference) : "Linked reference";
  }

  function sourceResultMeta(result: CrossrefSourceResult): string {
    return [result.authors, result.year, result.source].filter(Boolean).join(" · ");
  }

  function sourceResultKey(result: CrossrefSourceResult): string {
    return result.doi ?? result.url ?? result.title;
  }
</script>

<aside class="actions-panel" aria-label="Citation actions">
  <div class="panel-heading compact-heading">
    <p class="eyebrow">RADcite</p>
    <h2>Citation Actions</h2>
  </div>

  {#if selectedParagraph}
    <section class="selected-paragraph">
      <div class="paragraph-meta">
        <span>Paragraph {selectedParagraph.order_index + 1}</span>
        {#if selectedParagraph.page}
          <span>Page {selectedParagraph.page}</span>
        {/if}
        {#if selectedParagraph.is_table}
          <span>Table</span>
        {/if}
      </div>

      <div class="paragraph-full-text">
        {selectedParagraph.text}
      </div>

      <div class="citation-detail-block">
        <h3>Detected citations</h3>
        {#if selectedParagraph.citations.length}
          <div class="citation-badge-list">
            {#each selectedParagraph.citations as citation (citation.id)}
              <span class="citation-badge" class:is-verified={citation.verified}>
                {citation.text}
                {#if citation.reference_entry_id}
                  <small>{linkedReferenceLabel(citation.reference_entry_id)}</small>
                {/if}
                {#if citation.verified}
                  <small>Reviewed</small>
                {/if}
              </span>
            {/each}
          </div>
        {:else}
          <p>No inline citations detected.</p>
        {/if}
      </div>

      <div class="citation-detail-block">
        <h3>Status</h3>
        {#if selectedParagraph.needs_citation}
          <p class="status-warning">RADcite thinks this paragraph may need a citation.</p>
        {:else}
          <p>This paragraph does not currently need citation action.</p>
        {/if}
      </div>

      {#if suggestedCitationLinks.length}
        <div class="citation-detail-block suggestion-list-block">
          <h3>Suggested references</h3>
          <div class="suggestion-list">
            {#each suggestedCitationLinks as item (`${item.citationId}-${item.suggestion.reference_entry_id}`)}
              <div class="suggestion-card">
                <div class="suggestion-card-main">
                  <span class="suggestion-citation">{item.citationText}</span>
                  <strong>{item.suggestion.label}</strong>
                  <span>{item.suggestion.reason}</span>
                </div>
                <span class="confidence-badge" data-confidence={item.suggestion.confidence}>
                  {item.suggestion.confidence}
                </span>
                <button
                  class="secondary-button compact-button"
                  type="button"
                  onclick={() =>
                    void onLinkCitation(item.citationId, item.suggestion.reference_entry_id)}
                >
                  Accept
                </button>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <div class="action-stack">
        <button
          class="secondary-button"
          type="button"
          disabled={!sourceSearchSuggestion}
          aria-expanded={sourceSearchOpen}
          onclick={toggleSourceSearch}
        >
          Search sources
        </button>
        <button
          class="secondary-button"
          type="button"
          disabled={verifyDisabled}
          onclick={() => void onVerifyCitation(selectedParagraph.id)}
        >
          Mark citations reviewed
        </button>
        <button
          class="secondary-button"
          type="button"
          disabled={!selectedParagraph.needs_citation}
          onclick={() => void onMarkResolved(selectedParagraph.id)}
        >
          Not required
        </button>
      </div>

      {#if sourceSearchOpen && sourceSearchSuggestion}
        <div class="source-search-panel">
          <label class="field-label" for="source-search-query">
            Search query
            <span>{sourceSearchSuggestion.basis}</span>
          </label>
          <div class="source-search-row">
            <input
              id="source-search-query"
              class="path-input"
              type="text"
              bind:value={sourceSearchQuery}
            />
            <button
              class="primary-button compact-button"
              type="button"
              disabled={sourceSearchDisabled}
              onclick={() => void runSourceSearch()}
            >
              {sourceSearchLoading ? "Searching" : "Find matches"}
            </button>
            {#if sourceSearchUrl}
              <a
                class="secondary-button source-search-link"
                href={sourceSearchUrl}
                target="_blank"
                rel="noreferrer"
              >
                Open Crossref
              </a>
            {/if}
          </div>
          {#if sourceSearchError}
            <div class="notice source-search-notice">{sourceSearchError}</div>
          {/if}
          {#if sourceReferenceStatus}
            <div class="source-search-status">{sourceReferenceStatus}</div>
          {/if}
          {#if sourceSearchLoading}
            <div class="source-search-status">Searching Crossref</div>
          {:else if sourceSearchResults.length}
            <div class="source-result-list" aria-label="Crossref results">
              <h3>Crossref results</h3>
              {#each sourceSearchResults as result (sourceResultKey(result))}
                <article class="source-result-card">
                  <div class="source-result-main">
                    <strong>{result.title}</strong>
                    <span>{sourceResultMeta(result)}</span>
                    {#if result.doi}
                      <small>{result.doi}</small>
                    {/if}
                  </div>
                  <div class="source-result-actions">
                    <button
                      class="primary-button compact-button"
                      type="button"
                      disabled={sourceReferenceSavingKey !== null}
                      onclick={() => void addSourceReference(result)}
                    >
                      {sourceReferenceSavingKey === sourceResultKey(result)
                        ? "Adding"
                        : "Add reference"}
                    </button>
                    {#if result.url}
                      <a
                        class="secondary-button compact-button source-search-link"
                        href={result.url}
                        target="_blank"
                        rel="noreferrer"
                      >
                        Open DOI
                      </a>
                    {/if}
                  </div>
                </article>
              {/each}
            </div>
          {:else if sourceSearchResultsQuery}
            <div class="source-search-status">No Crossref results found.</div>
          {/if}
        </div>
      {/if}

      <form
        class="review-action-form"
        onsubmit={(event) => {
          event.preventDefault();
          submitManualCitation();
        }}
      >
        <label class="field-label" for="manual-citation">Add citation manually</label>
        <div class="manual-citation-row">
          <input
            id="manual-citation"
            class="path-input"
            type="text"
            bind:value={manualCitationText}
            placeholder="Smith (2024)"
            autocomplete="off"
          />
          <button class="primary-button" type="submit" disabled={manualCitationDisabled}>
            Add
          </button>
        </div>
        <p class="action-note">These changes are saved to the Local DB.</p>
      </form>

      <form
        class="review-action-form citation-link-form"
        onsubmit={(event) => {
          event.preventDefault();
          submitCitationLink();
        }}
      >
        <label class="field-label" for="citation-link-citation">Link citation to reference</label>
        <div class="citation-link-row">
          <select id="citation-link-citation" class="path-input" bind:value={selectedCitationId}>
            <option value="">Citation</option>
            {#each selectedParagraph.citations as citation (citation.id)}
              <option value={citation.id}>{citation.text}</option>
            {/each}
          </select>
          <select id="citation-link-reference" class="path-input" bind:value={selectedReferenceId}>
            <option value="">Course reference</option>
            {#each courseReferences as reference (reference.id)}
              <option value={reference.id}>{referenceLabel(reference)}</option>
            {/each}
          </select>
          <button class="primary-button" type="submit" disabled={linkDisabled}>
            Link
          </button>
        </div>
        <p class="action-note">Citation links are saved to the Local DB.</p>
      </form>
    </section>
  {:else}
    <div class="actions-empty">
      <strong>Select a paragraph</strong>
      <span>Paragraph-specific citation actions will appear here.</span>
    </div>
  {/if}
</aside>
