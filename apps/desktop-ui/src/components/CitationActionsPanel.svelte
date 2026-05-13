<script lang="ts">
  import type { ReviewParagraph } from "../types";

  type Props = {
    selectedParagraph: ReviewParagraph | null;
  };

  let { selectedParagraph }: Props = $props();
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
              <span class="citation-badge">{citation.text}</span>
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
        <button class="secondary-button" type="button" disabled>Verify citation</button>
        <button class="secondary-button" type="button" disabled>Mark as resolved</button>
        <button class="secondary-button" type="button" disabled>Add citation manually</button>
      </div>
    </section>
  {:else}
    <div class="actions-empty">
      <strong>Select a paragraph</strong>
      <span>Paragraph-specific citation actions will appear here.</span>
    </div>
  {/if}
</aside>
