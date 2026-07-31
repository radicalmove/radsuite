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
    onMergeReferences: (primaryReferenceId: string, mergeReferenceIds: string[]) => Promise<boolean>;
    onRefreshReferences: () => void | Promise<void>;
  };

  let {
    references,
    referencesLoading,
    referencesError,
    onAddReference,
    onUpdateReference,
    onArchiveReference,
    onMergeReferences,
    onRefreshReferences,
  }: Props = $props();

  let editingReferenceId = $state<string | null>(null);
  let apaCitation = $state("");
  let notes = $state("");
  let selectedReferenceIds = $state<string[]>([]);
  let primaryReferenceId = $state("");
  let editingReference = $derived(
    references.find((reference) => reference.id === editingReferenceId) ?? null,
  );
  let selectedReferences = $derived(
    references.filter((reference) => selectedReferenceIds.includes(reference.id)),
  );
  let mergeDisabled = $derived(referencesLoading || selectedReferences.length < 2);
  let submitDisabled = $derived(referencesLoading || apaCitation.trim().length === 0);

  $effect(() => {
    const availableIds = new Set(references.map((reference) => reference.id));
    const nextSelectedIds = selectedReferenceIds.filter((id) => availableIds.has(id));
    if (
      nextSelectedIds.length !== selectedReferenceIds.length ||
      nextSelectedIds.some((id, index) => id !== selectedReferenceIds[index])
    ) {
      selectedReferenceIds = nextSelectedIds;
    }
    if (!nextSelectedIds.includes(primaryReferenceId)) {
      primaryReferenceId = nextSelectedIds[0] ?? "";
    }
  });

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

  function toggleReferenceSelection(referenceId: string, selected: boolean) {
    if (selected) {
      if (!selectedReferenceIds.includes(referenceId)) {
        selectedReferenceIds = [...selectedReferenceIds, referenceId];
      }
      if (!primaryReferenceId) {
        primaryReferenceId = referenceId;
      }
      return;
    }

    selectedReferenceIds = selectedReferenceIds.filter((id) => id !== referenceId);
    if (primaryReferenceId === referenceId) {
      primaryReferenceId = selectedReferenceIds[0] ?? "";
    }
  }

  function clearReferenceSelection() {
    selectedReferenceIds = [];
    primaryReferenceId = "";
  }

  async function mergeSelectedReferences() {
    const primary = selectedReferences.find((reference) => reference.id === primaryReferenceId);
    const mergeReferences = selectedReferences.filter(
      (reference) => reference.id !== primaryReferenceId,
    );
    if (!primary || mergeReferences.length === 0) {
      return;
    }
    if (
      !window.confirm(
        `Keep "${referenceText(primary)}" and archive the other ${mergeReferences.length} selected course reference${mergeReferences.length === 1 ? "" : "s"}?`,
      )
    ) {
      return;
    }

    const merged = await onMergeReferences(
      primary.id,
      mergeReferences.map((reference) => reference.id),
    );
    if (merged) {
      clearReferenceSelection();
    }
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
      <div>
        <p class="eyebrow">Saved locally</p>
        <strong>{references.length} references</strong>
      </div>
      <div class="reference-bulk-actions">
        <span class="reference-selection-count">{selectedReferences.length} selected</span>
        {#if selectedReferences.length >= 2}
          <label class="reference-primary-select">
            <span>Keep</span>
            <select bind:value={primaryReferenceId} disabled={referencesLoading}>
              {#each selectedReferences as reference (reference.id)}
                <option value={reference.id}>{referenceText(reference)}</option>
              {/each}
            </select>
          </label>
          <button
            class="primary-button compact-button"
            type="button"
            disabled={mergeDisabled}
            onclick={() => void mergeSelectedReferences()}
          >
            Merge selected
          </button>
        {/if}
        <button
          class="secondary-button compact-button"
          type="button"
          disabled={selectedReferences.length === 0}
          onclick={clearReferenceSelection}
        >
          Clear selection
        </button>
      </div>
    </div>

    {#if referencesLoading}
      <div class="references-empty">Loading references</div>
    {:else if references.length}
      <div class="reference-list">
        {#each references as reference (reference.id)}
          <article class="reference-row">
            <div class="reference-row-header">
              <div class="reference-row-copy">
                <label class="reference-select">
                  <input
                    type="checkbox"
                    checked={selectedReferenceIds.includes(reference.id)}
                    onchange={(event) =>
                      toggleReferenceSelection(
                        reference.id,
                        (event.currentTarget as HTMLInputElement).checked,
                      )}
                  />
                  <span class="sr-only">Select reference</span>
                </label>
                <p>{referenceText(reference)}</p>
              </div>
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
