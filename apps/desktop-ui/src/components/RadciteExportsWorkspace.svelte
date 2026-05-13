<script lang="ts">
  import type { CourseReferencesExport, CourseReferenceSummary } from "../types";

  type Props = {
    references: CourseReferenceSummary[];
    referencesLoading: boolean;
    exportResult: CourseReferencesExport | null;
    exportLoading: boolean;
    exportError: string | null;
    onExportReferences: (forAkoLearn: boolean) => void | Promise<void>;
    onRefreshReferences: () => void | Promise<void>;
  };

  let {
    references,
    referencesLoading,
    exportResult,
    exportLoading,
    exportError,
    onExportReferences,
    onRefreshReferences,
  }: Props = $props();

  let forAkoLearn = $state(false);
  let copyNotice = $state<string | null>(null);
  let copyFailed = $state(false);

  let exportDisabled = $derived(exportLoading || referencesLoading);
  let resultActionDisabled = $derived(exportLoading || !exportResult);

  async function generateExport() {
    copyNotice = null;
    copyFailed = false;
    await onExportReferences(forAkoLearn);
  }

  async function copyHtml() {
    if (!exportResult) {
      return;
    }

    try {
      await navigator.clipboard.writeText(exportResult.html);
      copyNotice = "HTML copied.";
      copyFailed = false;
    } catch (reason: unknown) {
      copyNotice = reason instanceof Error ? reason.message : String(reason);
      copyFailed = true;
    }
  }

  function downloadHtml() {
    if (!exportResult) {
      return;
    }

    const blob = new Blob([exportResult.html], { type: exportResult.content_type });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = exportResult.filename;
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
      disabled={referencesLoading}
      onclick={() => void onRefreshReferences()}
    >
      Refresh references
    </button>
  </div>

  <section class="export-panel" aria-label="Course reference export controls">
    <div class="export-summary">
      <p class="eyebrow">Local DB</p>
      <strong>{references.length} references ready</strong>
      <span>Generate HTML from the current course references.</span>
    </div>

    <label class="checkbox-line">
      <input type="checkbox" bind:checked={forAkoLearn} />
      <span>AKO | LEARN</span>
    </label>

    <div class="export-actions">
      <button class="primary-button" type="button" disabled={exportDisabled} onclick={generateExport}>
        {exportLoading ? "Generating" : "Generate HTML"}
      </button>
      <button class="secondary-button" type="button" disabled={resultActionDisabled} onclick={copyHtml}>
        Copy HTML
      </button>
      <button class="secondary-button" type="button" disabled={resultActionDisabled} onclick={downloadHtml}>
        Download HTML
      </button>
    </div>
  </section>

  {#if exportError}
    <div class="notice export-notice">{exportError}</div>
  {/if}

  {#if copyNotice}
    <div class:notice={copyFailed} class="copy-notice">{copyNotice}</div>
  {/if}

  <section class="export-preview-panel" aria-label="Generated export preview">
    <div class="reference-list-heading">
      <p class="eyebrow">HTML preview</p>
      {#if exportResult}
        <strong>{exportResult.reference_count} exported</strong>
      {:else}
        <strong>No export generated</strong>
      {/if}
    </div>

    {#if exportResult}
      <pre class="export-preview">{exportResult.html}</pre>
    {:else}
      <div class="references-empty">Generate HTML to preview the export.</div>
    {/if}
  </section>
</section>
