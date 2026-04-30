<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CitationActionsPanel from "./components/CitationActionsPanel.svelte";
  import ProjectSidebar from "./components/ProjectSidebar.svelte";
  import RadciteDocumentsWorkspace from "./components/RadciteDocumentsWorkspace.svelte";
  import type {
    AnalyseDocxReviewResponse,
    AppStatus,
    ParagraphFilter,
    ProjectNavItem,
    ReviewParagraph,
    ToolArea,
  } from "./types";

  const fallbackStatus: AppStatus = {
    app_name: "RADsuite",
    database_ready: false,
    sync_configured: false,
    engines: [],
  };

  const projects: ProjectNavItem[] = [
    {
      id: "radcite-demo",
      code: "CRJU150",
      title: "RADcite Functional Testing",
      structureMode: "modules",
    },
  ];

  let status = $state<AppStatus>(fallbackStatus);
  let bridgeError = $state<string | null>(null);
  let selectedProjectId = $state(projects[0].id);
  let activeArea = $state<ToolArea>("documents");
  let analysisResult = $state<AnalyseDocxReviewResponse | null>(null);
  let activeFilter = $state<ParagraphFilter>("all");
  let selectedParagraphId = $state<string | null>(null);

  let selectedProject = $derived(
    projects.find((project) => project.id === selectedProjectId) ?? projects[0],
  );
  let selectedParagraph = $derived<ReviewParagraph | null>(
    analysisResult?.paragraphs.find((paragraph) => paragraph.id === selectedParagraphId) ?? null,
  );

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function handleAnalysisResult(result: AnalyseDocxReviewResponse | null) {
    analysisResult = result;
    selectedParagraphId = null;
  }

  onMount(() => {
    invoke<AppStatus>("get_app_status")
      .then((nextStatus) => {
        status = nextStatus;
        bridgeError = null;
      })
      .catch((reason: unknown) => {
        bridgeError = toErrorMessage(reason);
      });
  });
</script>

<main class="app-shell">
  <ProjectSidebar
    {projects}
    {selectedProjectId}
    {activeArea}
    onSelectProject={(projectId) => {
      selectedProjectId = projectId;
    }}
    onSelectArea={(area) => {
      activeArea = area;
      selectedParagraphId = null;
    }}
  />

  <section class="main-workspace" aria-label="Main workspace">
    <header class="workspace-topbar">
      <div>
        <p class="eyebrow">Project</p>
        <h2>{selectedProject.code} · {selectedProject.title}</h2>
      </div>
      <div class="status-strip" aria-label="Application status">
        <span class="pill" class:pill-active={status.database_ready}>Local DB</span>
        <span class="pill" class:pill-active={status.sync_configured}>Sync</span>
      </div>
    </header>

    {#if bridgeError}
      <div class="notice">Command bridge unavailable: {bridgeError}</div>
    {/if}

    {#if activeArea === "documents"}
      <RadciteDocumentsWorkspace
        {activeFilter}
        {analysisResult}
        {selectedParagraphId}
        onFilterChange={(filter) => {
          activeFilter = filter;
          selectedParagraphId = null;
        }}
        onAnalysisResult={handleAnalysisResult}
        onSelectParagraph={(paragraphId) => {
          selectedParagraphId = paragraphId;
        }}
      />
    {:else}
      <section class="workspace-placeholder">
        <p class="eyebrow">Coming later</p>
        <h2>{activeArea}</h2>
        <span>This area will be connected after the document review workspace is stable.</span>
      </section>
    {/if}
  </section>

  <CitationActionsPanel {selectedParagraph} />
</main>
