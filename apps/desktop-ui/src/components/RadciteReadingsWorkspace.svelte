<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import type {
    CourseModuleSummary,
    ModuleReadingImportCandidate,
    ModuleReadingSummary,
  } from "../types";
  import type {
    AddModuleReadingInput,
    AddRadciteModuleInput,
    PreviewModuleReadingsImportInput,
    SaveModuleReadingsImportInput,
    UpdateModuleReadingInput,
    UpdateRadciteModuleInput,
  } from "../lib/readingCommands";

  type EditableImportCandidate = Omit<
    ModuleReadingImportCandidate,
    "lesson_code" | "citation_text" | "url"
  > & {
    id: string;
    selected: boolean;
    module_id: string;
    lesson_code: string;
    citation_text: string;
    url: string;
    notes: string;
    reading_notes: string;
    estimated_reading_time: string;
  };

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
    onUpdateModule: (input: UpdateRadciteModuleInput) => void | Promise<void>;
    onArchiveModule: (moduleId: string) => void | Promise<void>;
    onAddReading: (input: AddModuleReadingInput) => void | Promise<void>;
    onUpdateReading: (input: UpdateModuleReadingInput) => void | Promise<void>;
    onArchiveReading: (readingId: string) => void | Promise<void>;
    onPreviewReadingsImport: (
      input: PreviewModuleReadingsImportInput,
    ) => ModuleReadingImportCandidate[] | Promise<ModuleReadingImportCandidate[]>;
    onSaveReadingsImport: (
      input: SaveModuleReadingsImportInput,
    ) => ModuleReadingSummary[] | Promise<ModuleReadingSummary[]>;
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
    onUpdateModule,
    onArchiveModule,
    onAddReading,
    onUpdateReading,
    onArchiveReading,
    onPreviewReadingsImport,
    onSaveReadingsImport,
  }: Props = $props();

  let importCandidateCounter = 0;
  let importPath = $state("");
  let importCandidates = $state<EditableImportCandidate[]>([]);
  let importLoading = $state(false);
  let importSaving = $state(false);
  let importError = $state<string | null>(null);
  let importStatus = $state<string | null>(null);
  let editingModuleId = $state<string | null>(null);
  let moduleTitle = $state("");
  let moduleCode = $state("");
  let moduleOrder = $state("");
  let moduleDescription = $state("");
  let editingReadingId = $state<string | null>(null);
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
  let editingModule = $derived(modules.find((module) => module.id === editingModuleId) ?? null);
  let editingReading = $derived(readings.find((reading) => reading.id === editingReadingId) ?? null);
  let compulsoryReadings = $derived(
    readings.filter((reading) => reading.reading_category === "compulsory"),
  );
  let optionalReadings = $derived(
    readings.filter((reading) => reading.reading_category === "optional"),
  );
  let moduleSubmitDisabled = $derived(modulesLoading || moduleTitle.trim().length === 0);
  let readingSubmitDisabled = $derived(
    readingsLoading ||
      !selectedModule ||
      (apaCitation.trim().length === 0 && citationText.trim().length === 0),
  );
  let selectedImportCount = $derived(
    importCandidates.filter((candidate) => candidate.selected).length,
  );
  let importSaveDisabled = $derived(
    importSaving ||
      modules.length === 0 ||
      selectedImportCount === 0 ||
      importCandidates.some((candidate) => candidate.selected && !candidate.module_id),
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

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function nextImportCandidateId(): string {
    importCandidateCounter += 1;
    return `reading-import-${importCandidateCounter}`;
  }

  function defaultModuleId(candidate: ModuleReadingImportCandidate): string {
    const byOrder = modules.find(
      (module) => module.order_index !== null && module.order_index === candidate.module_order,
    );
    if (byOrder) {
      return byOrder.id;
    }

    const moduleTitle = candidate.module_title?.trim().toLowerCase();
    const byTitle = moduleTitle
      ? modules.find((module) => module.title.trim().toLowerCase() === moduleTitle)
      : null;
    return byTitle?.id ?? selectedModule?.id ?? modules[0]?.id ?? "";
  }

  function editableImportCandidate(candidate: ModuleReadingImportCandidate): EditableImportCandidate {
    return {
      ...candidate,
      id: nextImportCandidateId(),
      selected: true,
      module_id: defaultModuleId(candidate),
      lesson_code: candidate.lesson_code ?? "",
      citation_text: candidate.citation_text ?? "",
      url: candidate.url ?? "",
      notes: "",
      reading_notes: "",
      estimated_reading_time: "",
    };
  }

  function candidateModuleLabel(candidate: EditableImportCandidate): string {
    const module = modules.find((item) => item.id === candidate.module_id);
    return module ? moduleLabel(module) : "Select module";
  }

  async function chooseReadingsDocx() {
    importError = null;
    importStatus = null;

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
        importPath = selected;
      } else if (Array.isArray(selected) && typeof selected[0] === "string") {
        importPath = selected[0];
      }
    } catch (reason: unknown) {
      importError = `Could not open the DOCX picker: ${toErrorMessage(reason)}`;
    }
  }

  async function previewReadingsImport() {
    const path = importPath.trim();
    if (!path) {
      importError = "Choose a DOCX file before previewing readings.";
      return;
    }

    importLoading = true;
    importError = null;
    importStatus = null;

    try {
      const candidates = await onPreviewReadingsImport({
        path,
        original_filename: null,
      });
      importCandidates = candidates.map(editableImportCandidate);
      importStatus = candidates.length
        ? `${candidates.length} reading candidates ready to review.`
        : "No reading candidates were detected in this DOCX.";
    } catch (reason: unknown) {
      importError = `Could not preview readings: ${toErrorMessage(reason)}`;
    } finally {
      importLoading = false;
    }
  }

  async function saveSelectedReadingsImport() {
    const selectedCandidates = importCandidates.filter((candidate) => candidate.selected);
    if (!selectedCandidates.length) {
      importError = "Select at least one reading before saving.";
      return;
    }

    importSaving = true;
    importError = null;
    importStatus = null;

    const input: SaveModuleReadingsImportInput = {
      candidates: selectedCandidates.map((candidate) => ({
        module_id: candidate.module_id,
        reading_category: candidate.reading_category,
        lesson_code: candidate.lesson_code,
        apa_citation: candidate.apa_citation,
        citation_text: candidate.citation_text,
        url: candidate.url,
        notes: candidate.notes,
        reading_notes: candidate.reading_notes,
        estimated_reading_time: candidate.estimated_reading_time,
      })),
    };

    try {
      const saved = await onSaveReadingsImport(input);
      const savedIds = new Set(selectedCandidates.map((candidate) => candidate.id));
      importCandidates = importCandidates.filter((candidate) => !savedIds.has(candidate.id));
      importStatus = `${saved.length} readings saved to the Local DB.`;
    } catch (reason: unknown) {
      importError = `Could not save selected readings: ${toErrorMessage(reason)}`;
    } finally {
      importSaving = false;
    }
  }

  function resetModuleForm() {
    editingModuleId = null;
    moduleTitle = "";
    moduleCode = "";
    moduleOrder = "";
    moduleDescription = "";
  }

  function beginEditModule(module: CourseModuleSummary) {
    editingModuleId = module.id;
    moduleTitle = module.title;
    moduleCode = module.code ?? "";
    moduleOrder = module.order_index?.toString() ?? "";
    moduleDescription = module.description ?? "";
  }

  async function archiveModule(module: CourseModuleSummary) {
    if (!window.confirm(`Remove ${moduleLabel(module)} from active module lists?`)) {
      return;
    }

    await onArchiveModule(module.id);
    if (editingModuleId === module.id) {
      resetModuleForm();
    }
  }

  async function submitModule() {
    const title = moduleTitle.trim();
    if (!title) {
      return;
    }

    const orderValue = moduleOrder.trim();
    const input = {
      title,
      code: moduleCode.trim() || null,
      order_index: orderValue ? Number(orderValue) : null,
      description: moduleDescription.trim() || null,
    };

    if (editingModuleId) {
      await onUpdateModule({
        ...input,
        module_id: editingModuleId,
      });
    } else {
      await onAddModule(input);
    }

    resetModuleForm();
  }

  function resetReadingForm() {
    editingReadingId = null;
    readingCategory = "compulsory";
    lessonCode = "";
    apaCitation = "";
    citationText = "";
    readingUrl = "";
    notes = "";
    readingNotes = "";
    estimatedReadingTime = "";
  }

  function beginEditReading(reading: ModuleReadingSummary) {
    editingReadingId = reading.id;
    readingCategory = reading.reading_category;
    lessonCode = reading.lesson_code ?? "";
    apaCitation = reading.apa_citation ?? "";
    citationText = reading.citation_text ?? "";
    readingUrl = reading.url ?? "";
    notes = reading.notes ?? "";
    readingNotes = reading.reading_notes ?? "";
    estimatedReadingTime = reading.estimated_reading_time ?? "";
  }

  async function archiveReading(reading: ModuleReadingSummary) {
    if (!window.confirm("Remove this reading from active reading lists?")) {
      return;
    }

    await onArchiveReading(reading.id);
    if (editingReadingId === reading.id) {
      resetReadingForm();
    }
  }

  async function submitReading() {
    if (!selectedModule || readingSubmitDisabled) {
      return;
    }

    const input = {
      reading_category: readingCategory,
      lesson_code: lessonCode.trim() || null,
      apa_citation: apaCitation.trim() || null,
      citation_text: citationText.trim() || null,
      url: readingUrl.trim() || null,
      notes: notes.trim() || null,
      reading_notes: readingNotes.trim() || null,
      estimated_reading_time: estimatedReadingTime.trim() || null,
    };

    if (editingReadingId) {
      await onUpdateReading({
        ...input,
        reading_id: editingReadingId,
      });
    } else {
      await onAddReading({
        ...input,
        module_id: selectedModule.id,
      });
    }

    resetReadingForm();
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

  <section class="reading-import-panel" aria-label="Import module readings">
    <form
      class="reading-import-form"
      onsubmit={(event) => {
        event.preventDefault();
        void previewReadingsImport();
      }}
    >
      <div class="form-section-heading">
        <div>
          <p class="eyebrow">DOCX import</p>
          <strong>Preview readings before saving</strong>
        </div>
        {#if importCandidates.length}
          <span class="module-current">{selectedImportCount} selected</span>
        {/if}
      </div>
      <label>
        <span class="field-label">Readings DOCX</span>
        <div class="path-row">
          <input
            class="path-input"
            type="text"
            bind:value={importPath}
            placeholder="/Users/name/Documents/module-readings.docx"
            autocomplete="off"
          />
          <button
            class="secondary-button choose-docx-button"
            type="button"
            disabled={importLoading || importSaving}
            onclick={() => void chooseReadingsDocx()}
          >
            Choose DOCX
          </button>
          <button
            class="primary-button"
            type="submit"
            disabled={importLoading || importSaving || importPath.trim().length === 0}
          >
            {importLoading ? "Previewing" : "Preview readings"}
          </button>
        </div>
      </label>
    </form>

    {#if importError}
      <div class="notice reading-notice">{importError}</div>
    {/if}
    {#if importStatus}
      <div class="import-status">{importStatus}</div>
    {/if}

    {#if importCandidates.length}
      <div class="reading-import-list" aria-label="Reading import candidates">
        {#each importCandidates as candidate (candidate.id)}
          <article class="reading-import-candidate">
            <div class="import-candidate-header">
              <label class="checkbox-line">
                <input type="checkbox" bind:checked={candidate.selected} />
                <span>Import</span>
              </label>
              <span class="module-current">{candidateModuleLabel(candidate)}</span>
            </div>

            <div class="form-grid form-grid-reading">
              <label>
                <span class="field-label">Module</span>
                <select class="path-input" bind:value={candidate.module_id}>
                  {#each modules as module (module.id)}
                    <option value={module.id}>{moduleLabel(module)}</option>
                  {/each}
                </select>
              </label>
              <label>
                <span class="field-label">Category</span>
                <select class="path-input" bind:value={candidate.reading_category}>
                  <option value="compulsory">Compulsory</option>
                  <option value="optional">Optional</option>
                </select>
              </label>
              <label>
                <span class="field-label">Lesson code</span>
                <input class="path-input" type="text" bind:value={candidate.lesson_code} />
              </label>
            </div>

            <label>
              <span class="field-label">APA reference</span>
              <textarea
                class="reference-textarea compact-textarea"
                rows="2"
                bind:value={candidate.apa_citation}
              ></textarea>
            </label>

            <div class="form-grid form-grid-reading">
              <label>
                <span class="field-label">URL</span>
                <input class="path-input" type="url" bind:value={candidate.url} />
              </label>
              <label>
                <span class="field-label">Student notes</span>
                <input class="path-input" type="text" bind:value={candidate.reading_notes} />
              </label>
              <label>
                <span class="field-label">Reading time</span>
                <input
                  class="path-input"
                  type="text"
                  bind:value={candidate.estimated_reading_time}
                />
              </label>
            </div>
          </article>
        {/each}
      </div>

      <div class="reference-form-actions">
        <button
          class="primary-button"
          type="button"
          disabled={importSaveDisabled}
          onclick={() => void saveSelectedReadingsImport()}
        >
          {importSaving ? "Saving" : "Save selected readings"}
        </button>
      </div>
    {/if}
  </section>

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
          <article class="module-card" class:is-active={module.id === selectedModuleId}>
            <button
              class="module-select-button"
              class:is-active={module.id === selectedModuleId}
              type="button"
              onclick={() => void onSelectModule(module.id)}
            >
              <strong>{module.title}</strong>
              <span>{module.code ?? `Order ${module.order_index ?? "-"}`}</span>
            </button>
            <div class="module-card-actions" aria-label={`${moduleLabel(module)} actions`}>
              <button
                class="secondary-button compact-button"
                type="button"
                onclick={() => beginEditModule(module)}
              >
                Edit module
              </button>
              <button
                class="secondary-button compact-button danger-button"
                type="button"
                onclick={() => void archiveModule(module)}
              >
                Remove module
              </button>
            </div>
          </article>
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
    <div class="form-section-heading">
      <div>
        <p class="eyebrow">{editingModuleId ? "Edit module" : "Add module"}</p>
        <strong>{editingModule ? moduleLabel(editingModule) : "Course module"}</strong>
      </div>
      {#if editingModuleId}
        <button class="secondary-button compact-button" type="button" onclick={resetModuleForm}>
          Cancel edit
        </button>
      {/if}
    </div>
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
      <button class="primary-button" type="submit" disabled={moduleSubmitDisabled}>
        {editingModuleId ? "Update module" : "Add module"}
      </button>
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
        <p class="eyebrow">{editingReadingId ? "Edit reading" : "Add reading"}</p>
        <strong>
          {editingReading
            ? readingText(editingReading)
            : selectedModule
              ? moduleLabel(selectedModule)
              : "Select a module"}
        </strong>
      </div>
      <div class="reading-form-controls">
        <label class="field-label compact-field">
          Category
          <select class="path-input compact-select" bind:value={readingCategory}>
            <option value="compulsory">Compulsory</option>
            <option value="optional">Optional</option>
          </select>
        </label>
        {#if editingReadingId}
          <button class="secondary-button compact-button" type="button" onclick={resetReadingForm}>
            Cancel edit
          </button>
        {/if}
      </div>
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
      <button class="primary-button" type="submit" disabled={readingSubmitDisabled}>
        {editingReadingId ? "Update reading" : "Add reading"}
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
                <div class="reading-row-header">
                  <div class="reading-row-main">
                    {#if reading.lesson_code}
                      <span class="reading-lesson">{reading.lesson_code}</span>
                    {/if}
                    <p>{readingText(reading)}</p>
                  </div>
                  <div class="reading-row-actions" aria-label="Reading actions">
                    <button
                      class="secondary-button compact-button"
                      type="button"
                      onclick={() => beginEditReading(reading)}
                    >
                      Edit reading
                    </button>
                    <button
                      class="secondary-button compact-button danger-button"
                      type="button"
                      onclick={() => void archiveReading(reading)}
                    >
                      Remove reading
                    </button>
                  </div>
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
                <div class="reading-row-header">
                  <div class="reading-row-main">
                    {#if reading.lesson_code}
                      <span class="reading-lesson">{reading.lesson_code}</span>
                    {/if}
                    <p>{readingText(reading)}</p>
                  </div>
                  <div class="reading-row-actions" aria-label="Reading actions">
                    <button
                      class="secondary-button compact-button"
                      type="button"
                      onclick={() => beginEditReading(reading)}
                    >
                      Edit reading
                    </button>
                    <button
                      class="secondary-button compact-button danger-button"
                      type="button"
                      onclick={() => void archiveReading(reading)}
                    >
                      Remove reading
                    </button>
                  </div>
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
