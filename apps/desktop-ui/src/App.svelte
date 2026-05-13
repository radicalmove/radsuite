<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CitationActionsPanel from "./components/CitationActionsPanel.svelte";
  import ProjectSidebar from "./components/ProjectSidebar.svelte";
  import RadciteDocumentsWorkspace from "./components/RadciteDocumentsWorkspace.svelte";
  import RadciteExportsWorkspace from "./components/RadciteExportsWorkspace.svelte";
  import RadciteReferencesWorkspace from "./components/RadciteReferencesWorkspace.svelte";
  import moonIcon from "./assets/moon.png";
  import { exportCourseReferences } from "./lib/exportCommands";
  import { addCourseReference, listCourseReferences } from "./lib/referenceCommands";
  import {
    persistAddManualCitation,
    persistLinkCitationToReference,
    persistMarkParagraphResolved,
    persistVerifyParagraphCitations,
  } from "./lib/reviewActionCommands";
  import { listSavedRadciteReviews, loadSavedRadciteReview } from "./lib/savedReviewCommands";
  import type {
    AnalyseDocxReviewResponse,
    AppStatus,
    CourseReferenceSummary,
    CourseReferencesExport,
    ParagraphFilter,
    ProjectNavItem,
    ReviewParagraph,
    SavedRadciteReviewSummary,
    ToolArea,
  } from "./types";

  const fallbackStatus: AppStatus = {
    app_name: "RADsuite",
    database_ready: false,
    sync_configured: false,
    engines: [],
  };
  const themeStorageKey = "radciteTheme";

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
  let theme = $state<"light" | "dark">("light");
  let reviewActionError = $state<string | null>(null);
  let savedReviews = $state<SavedRadciteReviewSummary[]>([]);
  let savedReviewsLoading = $state(false);
  let savedReviewsError = $state<string | null>(null);
  let courseReferences = $state<CourseReferenceSummary[]>([]);
  let courseReferencesLoading = $state(false);
  let courseReferencesError = $state<string | null>(null);
  let referencesExport = $state<CourseReferencesExport | null>(null);
  let referencesExportLoading = $state(false);
  let referencesExportError = $state<string | null>(null);

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
    reviewActionError = null;
    if (result) {
      void refreshSavedReviews();
    }
  }

  async function refreshSavedReviews() {
    savedReviewsLoading = true;
    savedReviewsError = null;
    try {
      savedReviews = await listSavedRadciteReviews();
    } catch (reason: unknown) {
      savedReviewsError = `Could not load saved reviews: ${toErrorMessage(reason)}`;
    } finally {
      savedReviewsLoading = false;
    }
  }

  async function handleLoadSavedReview(documentId: string) {
    savedReviewsError = null;
    reviewActionError = null;
    selectedParagraphId = null;
    try {
      analysisResult = await loadSavedRadciteReview(documentId);
      activeFilter = "all";
    } catch (reason: unknown) {
      savedReviewsError = `Could not open saved review: ${toErrorMessage(reason)}`;
    }
  }

  async function refreshCourseReferences() {
    courseReferencesLoading = true;
    courseReferencesError = null;
    try {
      courseReferences = await listCourseReferences();
    } catch (reason: unknown) {
      courseReferencesError = `Could not load course references: ${toErrorMessage(reason)}`;
    } finally {
      courseReferencesLoading = false;
    }
  }

  async function handleAddCourseReference(apaCitation: string, notes: string | null) {
    courseReferencesError = null;
    try {
      await addCourseReference({ apa_citation: apaCitation, notes });
      referencesExport = null;
      await refreshCourseReferences();
    } catch (reason: unknown) {
      courseReferencesError = `Could not add course reference: ${toErrorMessage(reason)}`;
    }
  }

  async function handleExportCourseReferences(forAkoLearn: boolean) {
    referencesExportLoading = true;
    referencesExportError = null;
    try {
      referencesExport = await exportCourseReferences({ for_ako_learn: forAkoLearn });
    } catch (reason: unknown) {
      referencesExportError = `Could not export course references: ${toErrorMessage(reason)}`;
    } finally {
      referencesExportLoading = false;
    }
  }

  async function handleMarkResolved(paragraphId: string) {
    if (!analysisResult) {
      return;
    }

    reviewActionError = null;
    try {
      analysisResult = await persistMarkParagraphResolved(analysisResult, paragraphId);
      void refreshSavedReviews();
    } catch (reason: unknown) {
      reviewActionError = `Could not save citation action: ${toErrorMessage(reason)}`;
    }
  }

  async function handleAddManualCitation(paragraphId: string, citationText: string) {
    if (!analysisResult) {
      return;
    }

    reviewActionError = null;
    try {
      analysisResult = await persistAddManualCitation(analysisResult, paragraphId, citationText);
      void refreshSavedReviews();
    } catch (reason: unknown) {
      reviewActionError = `Could not save citation action: ${toErrorMessage(reason)}`;
    }
  }

  async function handleVerifyCitation(paragraphId: string) {
    if (!analysisResult) {
      return;
    }

    reviewActionError = null;
    try {
      analysisResult = await persistVerifyParagraphCitations(analysisResult, paragraphId);
      void refreshSavedReviews();
    } catch (reason: unknown) {
      reviewActionError = `Could not save citation action: ${toErrorMessage(reason)}`;
    }
  }

  async function handleLinkCitation(citationId: string, referenceEntryId: string) {
    if (!analysisResult) {
      return;
    }

    reviewActionError = null;
    try {
      analysisResult = await persistLinkCitationToReference(
        analysisResult,
        citationId,
        referenceEntryId,
      );
      void refreshSavedReviews();
    } catch (reason: unknown) {
      reviewActionError = `Could not save citation action: ${toErrorMessage(reason)}`;
    }
  }

  function applyTheme(nextTheme: "light" | "dark") {
    theme = nextTheme;
    document.documentElement.dataset.theme = nextTheme;
  }

  function toggleTheme() {
    const nextTheme = theme === "dark" ? "light" : "dark";
    applyTheme(nextTheme);
    localStorage.setItem(themeStorageKey, nextTheme);
  }

  onMount(() => {
    const savedTheme = localStorage.getItem(themeStorageKey);
    applyTheme(savedTheme === "dark" ? "dark" : "light");

    invoke<AppStatus>("get_app_status")
      .then((nextStatus) => {
        status = nextStatus;
        bridgeError = null;
      })
      .catch((reason: unknown) => {
        bridgeError = toErrorMessage(reason);
      });
    void refreshSavedReviews();
    void refreshCourseReferences();
  });
</script>

<main class="app-shell" data-theme={theme}>
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
      if (area === "references" || area === "exports") {
        void refreshCourseReferences();
      }
    }}
  />

  <section class="main-workspace" aria-label="Main workspace">
    <header class="workspace-topbar">
      <div>
        <p class="eyebrow">Project</p>
        <h2>{selectedProject.code} · {selectedProject.title}</h2>
      </div>
      <div class="status-strip" aria-label="Application status">
        <span class="status-chip" class:is-ready={status.database_ready}>
          <span class="status-dot"></span>
          <span>{status.database_ready ? "Local DB ready" : "Local DB offline"}</span>
        </span>
        <span class="status-chip" class:is-ready={status.sync_configured}>
          <span class="status-dot"></span>
          <span>{status.sync_configured ? "Sync configured" : "Sync off"}</span>
        </span>
        <button
          class="theme-toggle"
          type="button"
          aria-label={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          aria-pressed={theme === "dark"}
          title={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          onclick={toggleTheme}
        >
          <img src={moonIcon} alt="" aria-hidden="true" />
        </button>
      </div>
    </header>

    {#if bridgeError}
      <div class="notice">Command bridge unavailable: {bridgeError}</div>
    {/if}
    {#if reviewActionError}
      <div class="notice">{reviewActionError}</div>
    {/if}

    {#if activeArea === "documents"}
      <RadciteDocumentsWorkspace
        {activeFilter}
        {analysisResult}
        {savedReviews}
        {savedReviewsLoading}
        {savedReviewsError}
        {selectedParagraphId}
        selectedDocumentId={analysisResult?.document_id ?? null}
        onFilterChange={(filter) => {
          activeFilter = filter;
          selectedParagraphId = null;
        }}
        onAnalysisResult={handleAnalysisResult}
        onLoadSavedReview={(documentId) => {
          void handleLoadSavedReview(documentId);
        }}
        onRefreshSavedReviews={() => {
          void refreshSavedReviews();
        }}
        onSelectParagraph={(paragraphId) => {
          selectedParagraphId = paragraphId;
        }}
      />
    {:else if activeArea === "references"}
      <RadciteReferencesWorkspace
        references={courseReferences}
        referencesLoading={courseReferencesLoading}
        referencesError={courseReferencesError}
        onAddReference={(apaCitation, notes) => {
          void handleAddCourseReference(apaCitation, notes);
        }}
        onRefreshReferences={() => {
          void refreshCourseReferences();
        }}
      />
    {:else if activeArea === "exports"}
      <RadciteExportsWorkspace
        references={courseReferences}
        referencesLoading={courseReferencesLoading}
        exportResult={referencesExport}
        exportLoading={referencesExportLoading}
        exportError={referencesExportError}
        onExportReferences={(forAkoLearn) => {
          void handleExportCourseReferences(forAkoLearn);
        }}
        onRefreshReferences={() => {
          void refreshCourseReferences();
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

  <CitationActionsPanel
    {selectedParagraph}
    {courseReferences}
    onMarkResolved={handleMarkResolved}
    onAddManualCitation={handleAddManualCitation}
    onVerifyCitation={handleVerifyCitation}
    onLinkCitation={handleLinkCitation}
  />
</main>
