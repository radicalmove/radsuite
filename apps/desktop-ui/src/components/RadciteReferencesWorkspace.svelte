<script lang="ts">
  import type { UpdateCourseReferenceInput } from "../lib/referenceCommands";
  import type { CourseReferenceSummary } from "../types";

  type Props = {
    references: CourseReferenceSummary[];
    referencesLoading: boolean;
    referencesError: string | null;
    onAddReference: (
      apaCitation: string,
      notes: string | null,
    ) => CourseReferenceSummary | null | void | Promise<CourseReferenceSummary | null | void>;
    onUpdateReference: (input: UpdateCourseReferenceInput) => void | Promise<void>;
    onArchiveReference: (referenceId: string) => void | Promise<void>;
    onRefreshReferences: () => void | Promise<void>;
  };

  let {
    references,
    referencesLoading,
    referencesError,
    onAddReference,
    onUpdateReference,
    onArchiveReference,
    onRefreshReferences,
  }: Props = $props();

  let editingReferenceId = $state<string | null>(null);
  let apaCitation = $state("");
  let notes = $state("");
  let editingReference = $derived(
    references.find((reference) => reference.id === editingReferenceId) ?? null,
  );
  let submitDisabled = $derived(referencesLoading || apaCitation.trim().length === 0);

  function referenceText(reference: CourseReferenceSummary): string {
    return reference.apa_citation ?? reference.citation_text ?? "Untitled reference";
  }

  function resetReferenceForm() {
    editingReferenceId = null;
    apaCitation = "";
    notes = "";
  }

  function beginEditReference(reference: CourseReferenceSummary) {
    editingReferenceId = reference.id;
    apaCitation = reference.apa_citation ?? reference.citation_text ?? "";
    notes = reference.notes ?? "";
  }

  async function archiveReference(reference: CourseReferenceSummary) {
    if (!window.confirm("Remove this course reference from active reference lists?")) {
      return;
    }

    await onArchiveReference(reference.id);
    if (editingReferenceId === reference.id) {
      resetReferenceForm();
    }
  }

  async function submitReference() {
    const nextApaCitation = apaCitation.trim();
    if (!nextApaCitation) {
      return;
    }

    if (editingReferenceId) {
      await onUpdateReference({
        reference_id: editingReferenceId,
        apa_citation: nextApaCitation,
        notes: notes.trim() || null,
      });
    } else {
      await onAddReference(nextApaCitation, notes.trim() || null);
    }

    resetReferenceForm();
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
    <div class="form-section-heading">
      <div>
        <p class="eyebrow">{editingReferenceId ? "Edit reference" : "Add reference"}</p>
        <strong>{editingReference ? "Course reference" : "APA source"}</strong>
      </div>
      {#if editingReferenceId}
        <button class="secondary-button compact-button" type="button" onclick={resetReferenceForm}>
          Cancel edit
        </button>
      {/if}
    </div>

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
      <button class="primary-button" type="submit" disabled={submitDisabled}>
        {editingReferenceId ? "Update reference" : "Add reference"}
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
            <div class="reference-row-header">
              <p>{referenceText(reference)}</p>
              <div class="reference-row-actions" aria-label="Course reference actions">
                <button
                  class="secondary-button compact-button"
                  type="button"
                  onclick={() => beginEditReference(reference)}
                >
                  Edit reference
                </button>
                <button
                  class="secondary-button compact-button danger-button"
                  type="button"
                  onclick={() => void archiveReference(reference)}
                >
                  Remove reference
                </button>
              </div>
            </div>
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
