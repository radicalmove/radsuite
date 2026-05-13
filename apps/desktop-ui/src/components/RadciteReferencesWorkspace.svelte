<script lang="ts">
  import type { CourseReferenceSummary } from "../types";

  type Props = {
    references: CourseReferenceSummary[];
    referencesLoading: boolean;
    referencesError: string | null;
    onAddReference: (apaCitation: string, notes: string | null) => void | Promise<void>;
    onRefreshReferences: () => void | Promise<void>;
  };

  let {
    references,
    referencesLoading,
    referencesError,
    onAddReference,
    onRefreshReferences,
  }: Props = $props();

  let apaCitation = $state("");
  let notes = $state("");
  let addDisabled = $derived(referencesLoading || apaCitation.trim().length === 0);

  function referenceText(reference: CourseReferenceSummary): string {
    return reference.apa_citation ?? reference.citation_text ?? "Untitled reference";
  }

  async function submitReference() {
    const nextApaCitation = apaCitation.trim();
    if (!nextApaCitation) {
      return;
    }

    await onAddReference(nextApaCitation, notes.trim() || null);
    apaCitation = "";
    notes = "";
  }
</script>

<section class="references-workspace" aria-labelledby="references-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcite</p>
      <h2 id="references-heading">Course References</h2>
    </div>
    <button
      class="secondary-button compact-button"
      type="button"
      disabled={referencesLoading}
      onclick={() => void onRefreshReferences()}
    >
      Refresh
    </button>
  </div>

  <form
    class="reference-add-form"
    onsubmit={(event) => {
      event.preventDefault();
      void submitReference();
    }}
  >
    <label class="field-label" for="reference-apa">APA reference</label>
    <textarea
      id="reference-apa"
      class="reference-textarea"
      bind:value={apaCitation}
      rows="4"
    ></textarea>

    <label class="field-label" for="reference-notes">Notes</label>
    <input id="reference-notes" class="path-input" type="text" bind:value={notes} />

    <div class="reference-form-actions">
      <button class="primary-button" type="submit" disabled={addDisabled}>
        Add reference
      </button>
    </div>
  </form>

  {#if referencesError}
    <div class="notice reference-notice">{referencesError}</div>
  {/if}

  <section class="reference-list-panel" aria-label="Course references">
    <div class="reference-list-heading">
      <p class="eyebrow">Local DB</p>
      <strong>{references.length} references</strong>
    </div>

    {#if referencesLoading}
      <div class="references-empty">Loading references</div>
    {:else if references.length}
      <div class="reference-list">
        {#each references as reference (reference.id)}
          <article class="reference-row">
            <p>{referenceText(reference)}</p>
            <div class="reference-meta">
              <span>{reference.validation_status.replace("_", " ")}</span>
              {#if reference.notes}
                <span>{reference.notes}</span>
              {/if}
            </div>
          </article>
        {/each}
      </div>
    {:else}
      <div class="references-empty">No course references yet.</div>
    {/if}
  </section>
</section>
