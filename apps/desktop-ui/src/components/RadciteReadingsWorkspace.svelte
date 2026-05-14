<script lang="ts">
  import type { CourseModuleSummary, ModuleReadingSummary } from "../types";
  import type { AddModuleReadingInput, AddRadciteModuleInput } from "../lib/readingCommands";

  type Props = {
    modules: CourseModuleSummary[];
    selectedModuleId: string | null;
    readings: ModuleReadingSummary[];
    modulesLoading: boolean;
    readingsLoading: boolean;
    modulesError: string | null;
    readingsError: string | null;
    onRefreshModules: () => void | Promise<void>;
    onSelectModule: (moduleId: string) => void | Promise<void>;
    onAddModule: (input: AddRadciteModuleInput) => void | Promise<void>;
    onAddReading: (input: AddModuleReadingInput) => void | Promise<void>;
  };

  let {
    modules,
    selectedModuleId,
    readings,
    modulesLoading,
    readingsLoading,
    modulesError,
    readingsError,
    onRefreshModules,
    onSelectModule,
    onAddModule,
    onAddReading,
  }: Props = $props();

  let moduleTitle = $state("");
  let moduleCode = $state("");
  let moduleOrder = $state("");
  let moduleDescription = $state("");
  let readingCategory = $state<"compulsory" | "optional">("compulsory");
  let lessonCode = $state("");
  let apaCitation = $state("");
  let citationText = $state("");
  let readingUrl = $state("");
  let notes = $state("");
  let readingNotes = $state("");
  let estimatedReadingTime = $state("");

  let selectedModule = $derived(
    modules.find((module) => module.id === selectedModuleId) ?? modules[0] ?? null,
  );
  let compulsoryReadings = $derived(
    readings.filter((reading) => reading.reading_category === "compulsory"),
  );
  let optionalReadings = $derived(
    readings.filter((reading) => reading.reading_category === "optional"),
  );
  let addModuleDisabled = $derived(modulesLoading || moduleTitle.trim().length === 0);
  let addReadingDisabled = $derived(
    readingsLoading ||
      !selectedModule ||
      (apaCitation.trim().length === 0 && citationText.trim().length === 0),
  );

  function moduleLabel(module: CourseModuleSummary): string {
    if (module.code) {
      return `${module.code} · ${module.title}`;
    }
    return module.title;
  }

  function readingText(reading: ModuleReadingSummary): string {
    return reading.apa_citation ?? reading.citation_text ?? reading.title ?? "Untitled reading";
  }

  async function submitModule() {
    const title = moduleTitle.trim();
    if (!title) {
      return;
    }

    const orderValue = moduleOrder.trim();
    await onAddModule({
      title,
      code: moduleCode.trim() || null,
      order_index: orderValue ? Number(orderValue) : null,
      description: moduleDescription.trim() || null,
    });

    moduleTitle = "";
    moduleCode = "";
    moduleOrder = "";
    moduleDescription = "";
  }

  async function submitReading() {
    if (!selectedModule || addReadingDisabled) {
      return;
    }

    await onAddReading({
      module_id: selectedModule.id,
      reading_category: readingCategory,
      lesson_code: lessonCode.trim() || null,
      apa_citation: apaCitation.trim() || null,
      citation_text: citationText.trim() || null,
      url: readingUrl.trim() || null,
      notes: notes.trim() || null,
      reading_notes: readingNotes.trim() || null,
      estimated_reading_time: estimatedReadingTime.trim() || null,
    });

    lessonCode = "";
    apaCitation = "";
    citationText = "";
    readingUrl = "";
    notes = "";
    readingNotes = "";
    estimatedReadingTime = "";
  }
</script>

<section class="readings-workspace" aria-labelledby="readings-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcite</p>
      <h2 id="readings-heading">Module readings</h2>
    </div>
    <button
      class="secondary-button compact-button"
      type="button"
      disabled={modulesLoading || readingsLoading}
      onclick={() => void onRefreshModules()}
    >
      Refresh
    </button>
  </div>

  {#if modulesError}
    <div class="notice reading-notice">{modulesError}</div>
  {/if}
  {#if readingsError}
    <div class="notice reading-notice">{readingsError}</div>
  {/if}

  <section class="module-selector" aria-label="Course modules">
    <div class="reference-list-heading">
      <div>
        <p class="eyebrow">Modules</p>
        <strong>{modules.length} available</strong>
      </div>
      {#if selectedModule}
        <span class="module-current">{moduleLabel(selectedModule)}</span>
      {/if}
    </div>

    {#if modulesLoading}
      <div class="references-empty">Loading modules</div>
    {:else if modules.length}
      <div class="module-button-list">
        {#each modules as module (module.id)}
          <button
            class="module-select-button"
            class:is-active={module.id === selectedModuleId}
            type="button"
            onclick={() => void onSelectModule(module.id)}
          >
            <strong>{module.title}</strong>
            <span>{module.code ?? `Order ${module.order_index ?? "-"}`}</span>
          </button>
        {/each}
      </div>
    {:else}
      <div class="references-empty">No modules yet.</div>
    {/if}
  </section>

  <form
    class="module-add-form"
    onsubmit={(event) => {
      event.preventDefault();
      void submitModule();
    }}
  >
    <div class="form-grid form-grid-module">
      <label>
        <span class="field-label">Module title</span>
        <input class="path-input" type="text" bind:value={moduleTitle} />
      </label>
      <label>
        <span class="field-label">Code</span>
        <input class="path-input" type="text" bind:value={moduleCode} />
      </label>
      <label>
        <span class="field-label">Order</span>
        <input class="path-input" type="number" min="1" step="1" bind:value={moduleOrder} />
      </label>
    </div>
    <label>
      <span class="field-label">Description</span>
      <input class="path-input" type="text" bind:value={moduleDescription} />
    </label>
    <div class="reference-form-actions">
      <button class="primary-button" type="submit" disabled={addModuleDisabled}>Add module</button>
    </div>
  </form>

  <form
    class="reading-add-form"
    onsubmit={(event) => {
      event.preventDefault();
      void submitReading();
    }}
  >
    <div class="reading-form-heading">
      <div>
        <p class="eyebrow">Add reading</p>
        <strong>{selectedModule ? moduleLabel(selectedModule) : "Select a module"}</strong>
      </div>
      <label class="field-label compact-field">
        Category
        <select class="path-input compact-select" bind:value={readingCategory}>
          <option value="compulsory">Compulsory</option>
          <option value="optional">Optional</option>
        </select>
      </label>
    </div>

    <div class="form-grid form-grid-reading">
      <label>
        <span class="field-label">Lesson code</span>
        <input class="path-input" type="text" bind:value={lessonCode} />
      </label>
      <label>
        <span class="field-label">URL</span>
        <input class="path-input" type="url" bind:value={readingUrl} />
      </label>
      <label>
        <span class="field-label">Reading time</span>
        <input class="path-input" type="text" bind:value={estimatedReadingTime} />
      </label>
    </div>

    <label class="field-label" for="reading-apa">APA reference</label>
    <textarea
      id="reading-apa"
      class="reference-textarea"
      bind:value={apaCitation}
      rows="3"
    ></textarea>

    <label class="field-label" for="reading-original">Original text</label>
    <textarea
      id="reading-original"
      class="reference-textarea compact-textarea"
      bind:value={citationText}
      rows="2"
    ></textarea>

    <div class="form-grid form-grid-reading">
      <label>
        <span class="field-label">Notes</span>
        <input class="path-input" type="text" bind:value={notes} />
      </label>
      <label>
        <span class="field-label">Student notes</span>
        <input class="path-input" type="text" bind:value={readingNotes} />
      </label>
    </div>

    <div class="reference-form-actions">
      <button class="primary-button" type="submit" disabled={addReadingDisabled}>
        Add reading
      </button>
    </div>
  </form>

  <section class="reading-list-panel" aria-label="Module readings list">
    <div class="reference-list-heading">
      <div>
        <p class="eyebrow">Local DB</p>
        <strong>{readings.length} readings</strong>
      </div>
      {#if readingsLoading}
        <span class="reading-loading">Loading</span>
      {/if}
    </div>

    {#if !selectedModule}
      <div class="references-empty">Create or select a module to add readings.</div>
    {:else if readingsLoading}
      <div class="references-empty">Loading readings</div>
    {:else if readings.length}
      <div class="reading-groups">
        <section class="reading-group" aria-label="Compulsory readings">
          <h3>Compulsory</h3>
          {#if compulsoryReadings.length}
            {#each compulsoryReadings as reading (reading.id)}
              <article class="reading-row">
                <div class="reading-row-main">
                  {#if reading.lesson_code}
                    <span class="reading-lesson">{reading.lesson_code}</span>
                  {/if}
                  <p>{readingText(reading)}</p>
                </div>
                <div class="reference-meta">
                  <span>{reading.validation_status.replace("_", " ")}</span>
                  {#if reading.estimated_reading_time}
                    <span>{reading.estimated_reading_time}</span>
                  {/if}
                  {#if reading.url}
                    <span>{reading.url}</span>
                  {/if}
                </div>
              </article>
            {/each}
          {:else}
            <div class="references-empty">No compulsory readings yet.</div>
          {/if}
        </section>

        <section class="reading-group" aria-label="Optional readings">
          <h3>Optional</h3>
          {#if optionalReadings.length}
            {#each optionalReadings as reading (reading.id)}
              <article class="reading-row">
                <div class="reading-row-main">
                  {#if reading.lesson_code}
                    <span class="reading-lesson">{reading.lesson_code}</span>
                  {/if}
                  <p>{readingText(reading)}</p>
                </div>
                <div class="reference-meta">
                  <span>{reading.validation_status.replace("_", " ")}</span>
                  {#if reading.estimated_reading_time}
                    <span>{reading.estimated_reading_time}</span>
                  {/if}
                  {#if reading.url}
                    <span>{reading.url}</span>
                  {/if}
                </div>
              </article>
            {/each}
          {:else}
            <div class="references-empty">No optional readings yet.</div>
          {/if}
        </section>
      </div>
    {:else}
      <div class="references-empty">No readings recorded for this module yet.</div>
    {/if}
  </section>
</section>
