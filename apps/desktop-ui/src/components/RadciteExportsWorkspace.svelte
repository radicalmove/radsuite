<script lang="ts">
  import type {
    CourseModuleSummary,
    CourseReferencesExport,
    CourseReferenceSummary,
    ModuleReadingsExport,
    ModuleReadingSummary,
  } from "../types";

  type ExportMode = "course-references" | "module-readings";
  type HtmlExportResult = CourseReferencesExport | ModuleReadingsExport;

  type Props = {
    references: CourseReferenceSummary[];
    modules: CourseModuleSummary[];
    selectedModuleId: string | null;
    moduleReadings: ModuleReadingSummary[];
    referencesLoading: boolean;
    modulesLoading: boolean;
    readingsLoading: boolean;
    referenceExportResult: CourseReferencesExport | null;
    referenceExportLoading: boolean;
    referenceExportError: string | null;
    moduleExportResult: ModuleReadingsExport | null;
    moduleExportLoading: boolean;
    moduleExportError: string | null;
    onExportReferences: (forAkoLearn: boolean, allowIncomplete: boolean) => void | Promise<void>;
    onExportModuleReadings: (moduleId: string, forAkoLearn: boolean) => void | Promise<void>;
    onRefreshReferences: () => void | Promise<void>;
    onRefreshModules: () => void | Promise<void>;
    onSelectModule: (moduleId: string) => void | Promise<void>;
  };

  let {
    references,
    modules,
    selectedModuleId,
    moduleReadings,
    referencesLoading,
    modulesLoading,
    readingsLoading,
    referenceExportResult,
    referenceExportLoading,
    referenceExportError,
    moduleExportResult,
    moduleExportLoading,
    moduleExportError,
    onExportReferences,
    onExportModuleReadings,
    onRefreshReferences,
    onRefreshModules,
    onSelectModule,
  }: Props = $props();

  let exportMode = $state<ExportMode>("course-references");
  let forAkoLearn = $state(false);
  let allowIncomplete = $state(false);
  let copyNotice = $state<string | null>(null);
  let copyFailed = $state(false);

  let selectedModule = $derived(
    modules.find((module) => module.id === selectedModuleId) ?? modules[0] ?? null,
  );
  let activeExportResult = $derived<HtmlExportResult | null>(
    exportMode === "course-references" ? referenceExportResult : moduleExportResult,
  );
  let activeExportLoading = $derived(
    exportMode === "course-references" ? referenceExportLoading : moduleExportLoading,
  );
  let activeExportError = $derived(
    exportMode === "course-references" ? referenceExportError : moduleExportError,
  );
  let sourceLoading = $derived(
    exportMode === "course-references" ? referencesLoading : modulesLoading || readingsLoading,
  );
  let apaFixCount = $derived(
    references.filter((reference) => reference.validation_status === "needs_fix").length,
  );
  let apaWarningCount = $derived(
    references.filter((reference) => reference.validation_status === "unknown").length,
  );
  let exportDisabled = $derived(
    activeExportLoading ||
      sourceLoading ||
      (exportMode === "module-readings" && !selectedModule) ||
      (exportMode === "course-references" && apaFixCount > 0 && !allowIncomplete),
  );
  let resultActionDisabled = $derived(activeExportLoading || !activeExportResult);

  function moduleLabel(module: CourseModuleSummary): string {
    if (module.code) {
      return `${module.code} · ${module.title}`;
    }
    return module.title;
  }

  function exportCount(result: HtmlExportResult): number {
    return "reference_count" in result ? result.reference_count : result.reading_count;
  }

  function setExportMode(mode: ExportMode) {
    exportMode = mode;
    allowIncomplete = false;
    copyNotice = null;
    copyFailed = false;
  }

  async function refreshExportSource() {
    copyNotice = null;
    copyFailed = false;
    if (exportMode === "course-references") {
      await onRefreshReferences();
    } else {
      await onRefreshModules();
    }
  }

  async function generateExport() {
    copyNotice = null;
    copyFailed = false;
    if (exportMode === "course-references") {
      await onExportReferences(forAkoLearn, allowIncomplete);
      return;
    }

    if (selectedModule) {
      await onExportModuleReadings(selectedModule.id, forAkoLearn);
    }
  }

  async function copyHtml() {
    if (!activeExportResult) {
      return;
    }

    try {
      await navigator.clipboard.writeText(activeExportResult.html);
      copyNotice = "HTML copied.";
      copyFailed = false;
    } catch (reason: unknown) {
      copyNotice = reason instanceof Error ? reason.message : String(reason);
      copyFailed = true;
    }
  }

  function downloadHtml() {
    if (!activeExportResult) {
      return;
    }

    const blob = new Blob([activeExportResult.html], { type: activeExportResult.content_type });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = activeExportResult.filename;
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(url);
  }
</script>

<section class="exports-workspace" aria-labelledby="exports-heading">
  <div class="workspace-header">
    <div>
      <p class="eyebrow">RADcite</p>
      <h2 id="exports-heading">Course References Export</h2>
    </div>
    <button
      class="secondary-button compact-button"
      type="button"
      disabled={sourceLoading}
      onclick={() => void refreshExportSource()}
    >
      Refresh data
    </button>
  </div>

  <section class="export-panel" aria-label="Export controls">
    <div class="export-mode-toggle" aria-label="Export mode">
      <button
        class:is-active={exportMode === "course-references"}
        type="button"
        onclick={() => setExportMode("course-references")}
      >
        Course references
      </button>
      <button
        class:is-active={exportMode === "module-readings"}
        type="button"
        onclick={() => setExportMode("module-readings")}
      >
        Module readings
      </button>
    </div>

    <div class="export-summary">
      <p class="eyebrow">Saved locally</p>
      {#if exportMode === "course-references"}
        <strong>Course References Export</strong>
        <span>{references.length} references ready</span>
      {:else}
        <strong>Module readings export</strong>
        <span>{moduleReadings.length} readings ready</span>
      {/if}
    </div>

    {#if exportMode === "module-readings"}
      <div class="module-export-controls">
        <label>
          <span class="field-label">Module selector</span>
          <select
            class="path-input compact-select"
            disabled={modulesLoading || modules.length === 0}
            value={selectedModule?.id ?? ""}
            onchange={(event) => {
              const target = event.currentTarget as HTMLSelectElement;
              if (target.value) {
                void onSelectModule(target.value);
              }
            }}
          >
            {#each modules as module (module.id)}
              <option value={module.id}>{moduleLabel(module)}</option>
            {/each}
          </select>
        </label>
        <div class="module-export-current">
          <p class="eyebrow">Selected module</p>
          <strong>{selectedModule ? moduleLabel(selectedModule) : "No module selected"}</strong>
        </div>
      </div>
    {/if}

    <label class="checkbox-line">
      <input type="checkbox" bind:checked={forAkoLearn} />
      <span>AKO | LEARN</span>
    </label>

    {#if exportMode === "course-references" && apaFixCount > 0}
      <div class="notice export-notice" role="status">
        <strong>{apaFixCount} course reference{apaFixCount === 1 ? "" : "s"} need APA fixes.</strong>
        <span>Fix them before exporting, or explicitly allow an incomplete export.</span>
        <label class="checkbox-line">
          <input type="checkbox" bind:checked={allowIncomplete} />
          <span>Export with APA fixes pending</span>
        </label>
      </div>
    {:else if exportMode === "course-references" && apaWarningCount > 0}
      <div class="notice export-notice" role="status">
        {apaWarningCount} course reference{apaWarningCount === 1 ? " has" : "s have"} not been checked for APA issues.
      </div>
    {/if}

    <div class="export-actions">
      <button class="primary-button" type="button" disabled={exportDisabled} onclick={generateExport}>
        {activeExportLoading ? "Generating" : "Generate HTML"}
      </button>
      <button class="secondary-button" type="button" disabled={resultActionDisabled} onclick={copyHtml}>
        Copy HTML
      </button>
      <button class="secondary-button" type="button" disabled={resultActionDisabled} onclick={downloadHtml}>
        Download HTML
      </button>
    </div>
  </section>

  {#if activeExportError}
    <div class="notice export-notice">{activeExportError}</div>
  {/if}

  {#if copyNotice}
    <div class:notice={copyFailed} class="copy-notice">{copyNotice}</div>
  {/if}

  <section class="export-preview-panel" aria-label="Generated export preview">
    <div class="reference-list-heading">
      <p class="eyebrow">HTML preview</p>
      {#if activeExportResult}
        <strong>{exportCount(activeExportResult)} exported</strong>
        {#if
          exportMode === "course-references" &&
          "apa_error_count" in activeExportResult &&
          activeExportResult.apa_error_count > 0
        }
          <span>{activeExportResult.apa_error_count} exported with APA fixes pending</span>
        {:else if
          exportMode === "course-references" &&
          "apa_warning_count" in activeExportResult &&
          activeExportResult.apa_warning_count > 0
        }
          <span>{activeExportResult.apa_warning_count} not checked for APA issues</span>
        {/if}
      {:else}
        <strong>No export generated</strong>
      {/if}
    </div>

    {#if activeExportResult}
      <pre class="export-preview">{activeExportResult.html}</pre>
    {:else}
      <div class="references-empty">Generate HTML to preview the export.</div>
    {/if}
  </section>
</section>
