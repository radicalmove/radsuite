<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

  type EngineStatus = {
    id: string;
    label: string;
    available: boolean;
    detail: string;
  };

  type AppStatus = {
    app_name: string;
    database_ready: boolean;
    sync_configured: boolean;
    engines: EngineStatus[];
  };

  type AnalyseDocxResponse = {
    project_id: string;
    project_title: string;
    document_id: string;
    original_filename: string;
    paragraph_count: number;
    citation_count: number;
    missing_citation_count: number;
  };

  const fallbackStatus: AppStatus = {
    app_name: "RADsuite",
    database_ready: false,
    sync_configured: false,
    engines: [],
  };

  let status = $state<AppStatus>(fallbackStatus);
  let error = $state<string | null>(null);
  let docxPath = $state("");
  let analysisLoading = $state(false);
  let analysisResult = $state<AnalyseDocxResponse | null>(null);
  let analysisError = $state<string | null>(null);
  let analysisDisabled = $derived(analysisLoading || docxPath.trim().length === 0);

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  async function analyseDocx() {
    const path = docxPath.trim();
    if (!path) {
      analysisError = "Choose a DOCX file before running RADcite analysis.";
      return;
    }

    analysisLoading = true;
    analysisError = null;
    analysisResult = null;

    try {
      analysisResult = await invoke<AnalyseDocxResponse>("analyse_docx_path", {
        request: {
          path,
          original_filename: null,
        },
      });
    } catch (reason: unknown) {
      analysisError = toErrorMessage(reason);
    } finally {
      analysisLoading = false;
    }
  }

  onMount(() => {
    invoke<AppStatus>("get_app_status")
      .then((nextStatus) => {
        status = nextStatus;
        error = null;
      })
      .catch((reason: unknown) => {
        error = toErrorMessage(reason);
      });
  });
</script>

<main class="app-shell">
  <header class="topbar">
    <div>
      <p class="eyebrow">Internal alpha</p>
      <h1>{status.app_name}</h1>
    </div>
    <div class="status-strip" aria-label="Application status">
      <span class="pill" class:pill-active={status.database_ready}>Local DB</span>
      <span class="pill" class:pill-active={status.sync_configured}>Sync</span>
    </div>
  </header>

  {#if error}
    <div class="notice">Command bridge unavailable: {error}</div>
  {/if}

  <section class="workspace-grid">
    <section class="panel project-panel" aria-labelledby="projects-heading">
      <div class="panel-heading">
        <p class="eyebrow">RADcite</p>
        <h2 id="projects-heading">DOCX Analysis</h2>
      </div>
      <form
        class="analysis-form"
        onsubmit={(event) => {
          event.preventDefault();
          void analyseDocx();
        }}
      >
        <label class="field-label" for="docx-path">DOCX file path</label>
        <div class="path-row">
          <input
            id="docx-path"
            class="path-input"
            type="text"
            bind:value={docxPath}
            placeholder="/Users/name/Documents/source.docx"
            autocomplete="off"
          />
          <button class="primary-button" type="submit" disabled={analysisDisabled}>
            {analysisLoading ? "Analysing" : "Analyse"}
          </button>
        </div>
      </form>

      {#if analysisError}
        <div class="notice analysis-notice">{analysisError}</div>
      {/if}

      {#if analysisResult}
        <section class="analysis-result" aria-live="polite">
          <div>
            <p class="eyebrow">Result</p>
            <h3>{analysisResult.original_filename}</h3>
            <span>{analysisResult.project_title}</span>
          </div>
          <div class="metric-grid" aria-label="RADcite analysis counts">
            <div class="metric">
              <strong>{analysisResult.paragraph_count}</strong>
              <span>Paragraphs</span>
            </div>
            <div class="metric">
              <strong>{analysisResult.citation_count}</strong>
              <span>Citations</span>
            </div>
            <div class="metric metric-warn">
              <strong>{analysisResult.missing_citation_count}</strong>
              <span>Need citations</span>
            </div>
          </div>
        </section>
      {/if}
    </section>

    <section class="panel" aria-labelledby="engines-heading">
      <div class="panel-heading">
        <p class="eyebrow">Native runtimes</p>
        <h2 id="engines-heading">Engines</h2>
      </div>
      <div class="engine-list">
        {#each status.engines as engine (engine.id)}
          <article class="engine-row">
            <div>
              <strong>{engine.label}</strong>
              <span>{engine.detail}</span>
            </div>
            <span class="pill" class:pill-active={engine.available}>
              {engine.available ? "Ready" : "Missing"}
            </span>
          </article>
        {/each}
      </div>
    </section>
  </section>
</main>
