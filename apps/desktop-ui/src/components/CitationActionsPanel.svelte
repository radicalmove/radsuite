<script lang="ts">
  import type { ReviewParagraph } from "../types";

  type Props = {
    selectedParagraph: ReviewParagraph | null;
    onMarkResolved: (paragraphId: string) => void | Promise<void>;
    onAddManualCitation: (paragraphId: string, citationText: string) => void | Promise<void>;
    onVerifyCitation: (paragraphId: string) => void | Promise<void>;
  };

  let {
    selectedParagraph,
    onMarkResolved,
    onAddManualCitation,
    onVerifyCitation,
  }: Props = $props();

  let manualCitationText = $state("");
  let manualCitationDisabled = $derived(
    !selectedParagraph || manualCitationText.trim().length === 0,
  );
  let verifyDisabled = $derived(
    !selectedParagraph ||
      selectedParagraph.citations.length === 0 ||
      selectedParagraph.citations.every((citation) => citation.verified),
  );

  function submitManualCitation() {
    if (!selectedParagraph || manualCitationDisabled) {
      return;
    }

    void onAddManualCitation(selectedParagraph.id, manualCitationText);
    manualCitationText = "";
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
    </section>
  {:else}
    <div class="actions-empty">
      <strong>Select a paragraph</strong>
      <span>Paragraph-specific citation actions will appear here.</span>
    </div>
  {/if}
</aside>
