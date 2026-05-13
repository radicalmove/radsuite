<script lang="ts">
  import type { CourseReferenceSummary, ReviewParagraph } from "../types";

  type Props = {
    selectedParagraph: ReviewParagraph | null;
    courseReferences: CourseReferenceSummary[];
    onMarkResolved: (paragraphId: string) => void | Promise<void>;
    onAddManualCitation: (paragraphId: string, citationText: string) => void | Promise<void>;
    onVerifyCitation: (paragraphId: string) => void | Promise<void>;
    onLinkCitation: (citationId: string, referenceEntryId: string) => void | Promise<void>;
  };

  let {
    selectedParagraph,
    courseReferences,
    onMarkResolved,
    onAddManualCitation,
    onVerifyCitation,
    onLinkCitation,
  }: Props = $props();

  let manualCitationText = $state("");
  let selectedCitationId = $state("");
  let selectedReferenceId = $state("");
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

  $effect(() => {
    if (!selectedParagraph?.citations.some((citation) => citation.id === selectedCitationId)) {
      selectedCitationId = "";
    }
    if (!courseReferences.some((reference) => reference.id === selectedReferenceId)) {
      selectedReferenceId = "";
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

  function referenceLabel(reference: CourseReferenceSummary): string {
    return (
      reference.apa_citation ?? reference.citation_text ?? reference.title ?? "Untitled reference"
    );
  }

  function linkedReferenceLabel(referenceEntryId: string): string {
    const linkedReference = courseReferences.find((reference) => reference.id === referenceEntryId);
    return linkedReference ? referenceLabel(linkedReference) : "Linked reference";
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
                  <small>Verified</small>
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

      <div class="action-stack">
        <button class="secondary-button" type="button" disabled>Search sources</button>
        <button
          class="secondary-button"
          type="button"
          disabled={verifyDisabled}
          onclick={() => void onVerifyCitation(selectedParagraph.id)}
        >
          Verify citation
        </button>
        <button
          class="secondary-button"
          type="button"
          disabled={!selectedParagraph.needs_citation}
          onclick={() => void onMarkResolved(selectedParagraph.id)}
        >
          Mark as resolved
        </button>
      </div>

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
