<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CitationActionsPanel from "./components/CitationActionsPanel.svelte";
  import ProjectSidebar from "./components/ProjectSidebar.svelte";
  import RadciteDocumentsWorkspace from "./components/RadciteDocumentsWorkspace.svelte";
  import RadciteExportsWorkspace from "./components/RadciteExportsWorkspace.svelte";
  import RadciteReferencesWorkspace from "./components/RadciteReferencesWorkspace.svelte";
  import RadciteReadingsWorkspace from "./components/RadciteReadingsWorkspace.svelte";
  import moonIcon from "./assets/moon.png";
  import { exportCourseReferences, exportModuleReadings } from "./lib/exportCommands";
  import { createRadciteProject, listRadciteProjects } from "./lib/projectCommands";
  import {
    addModuleReading,
    addRadciteModule,
    archiveModuleReading,
    archiveRadciteModule,
    listModuleReadings,
    listRadciteModules,
    previewModuleReadingsCsvImport,
    previewModuleReadingsImport,
    saveModuleReadingsImport,
    updateModuleReading,
    updateRadciteModule,
  } from "./lib/readingCommands";
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
    CourseModuleSummary,
    CourseReferenceSummary,
    CourseReferencesExport,
    ModuleReadingsExport,
    ModuleReadingSummary,
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

  const fallbackProject: ProjectNavItem = {
    id: "radcite-fallback",
    code: "CRJU150",
    title: "RADcite Functional Testing",
    structureMode: "modules",
  };

  let status = $state<AppStatus>(fallbackStatus);
  let bridgeError = $state<string | null>(null);
  let projects = $state<ProjectNavItem[]>([fallbackProject]);
  let projectsLoading = $state(false);
  let projectsError = $state<string | null>(null);
  let selectedProjectId = $state(fallbackProject.id);
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
  let radciteModules = $state<CourseModuleSummary[]>([]);
  let radciteModulesLoading = $state(false);
  let radciteModulesError = $state<string | null>(null);
  let selectedModuleId = $state<string | null>(null);
  let moduleReadings = $state<ModuleReadingSummary[]>([]);
  let moduleReadingsLoading = $state(false);
  let moduleReadingsError = $state<string | null>(null);
  let referencesExport = $state<CourseReferencesExport | null>(null);
  let referencesExportLoading = $state(false);
  let referencesExportError = $state<string | null>(null);
  let moduleReadingsExport = $state<ModuleReadingsExport | null>(null);
  let moduleReadingsExportLoading = $state(false);
  let moduleReadingsExportError = $state<string | null>(null);

  let selectedProject = $derived(
    projects.find((project) => project.id === selectedProjectId) ?? projects[0] ?? fallbackProject,
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

  function selectedProjectCommandId(): string | null {
    return selectedProjectId === fallbackProject.id ? null : selectedProjectId;
  }

  function projectNavItem(project: { id: string; code: string | null; title: string }): ProjectNavItem {
    return {
      id: project.id,
      code: project.code ?? "RADcite",
      title: project.title,
      structureMode: "modules",
    };
  }

  function resetProjectScopedState() {
    analysisResult = null;
    activeFilter = "all";
    selectedParagraphId = null;
    reviewActionError = null;
    savedReviews = [];
    savedReviewsError = null;
    courseReferences = [];
    courseReferencesError = null;
    radciteModules = [];
    radciteModulesError = null;
    selectedModuleId = null;
    moduleReadings = [];
    moduleReadingsError = null;
    referencesExport = null;
    referencesExportError = null;
    moduleReadingsExport = null;
    moduleReadingsExportError = null;
  }

  async function refreshProjects(preferredProjectId: string | null = selectedProjectId) {
    projectsLoading = true;
    projectsError = null;
    try {
      const loadedProjects = await listRadciteProjects();
      const nextProjects = loadedProjects.map(projectNavItem);
      projects = nextProjects.length ? nextProjects : [fallbackProject];
      selectedProjectId =
        (preferredProjectId && nextProjects.some((project) => project.id === preferredProjectId)
          ? preferredProjectId
          : nextProjects[0]?.id) ?? fallbackProject.id;
    } catch (reason: unknown) {
      projectsError = `Could not load projects: ${toErrorMessage(reason)}`;
      projects = [fallbackProject];
      selectedProjectId = fallbackProject.id;
    } finally {
      projectsLoading = false;
    }
  }

  async function handleCreateProject(input: Parameters<typeof createRadciteProject>[0]) {
    projectsError = null;
    try {
      const created = await createRadciteProject(input);
      await refreshProjects(created.id);
      resetProjectScopedState();
      await refreshSavedReviews();
      await refreshCourseReferences();
    } catch (reason: unknown) {
      projectsError = `Could not create project: ${toErrorMessage(reason)}`;
      throw reason;
    }
  }

  async function handleSelectProject(projectId: string) {
    if (projectId === selectedProjectId) {
      return;
    }

    selectedProjectId = projectId;
    resetProjectScopedState();
    await refreshSavedReviews();
    await refreshCourseReferences();
    if (activeArea === "readings" || activeArea === "exports") {
      await refreshRadciteModules(null);
    }
  }

  async function refreshSavedReviews() {
    savedReviewsLoading = true;
    savedReviewsError = null;
    try {
      savedReviews = await listSavedRadciteReviews(selectedProjectCommandId());
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
      courseReferences = await listCourseReferences(selectedProjectCommandId());
    } catch (reason: unknown) {
      courseReferencesError = `Could not load course references: ${toErrorMessage(reason)}`;
    } finally {
      courseReferencesLoading = false;
    }
  }

  async function refreshRadciteModules(preferredModuleId: string | null = selectedModuleId) {
    radciteModulesLoading = true;
    radciteModulesError = null;
    try {
      const previousSelectedModuleId = selectedModuleId;
      const nextModules = await listRadciteModules(selectedProjectCommandId());
      radciteModules = nextModules;
      const nextSelected =
        (preferredModuleId && nextModules.some((module) => module.id === preferredModuleId)
          ? preferredModuleId
          : nextModules[0]?.id) ?? null;
      selectedModuleId = nextSelected;
      if (nextSelected !== previousSelectedModuleId) {
        moduleReadingsExport = null;
      }
      if (nextSelected) {
        await refreshModuleReadings(nextSelected);
      } else {
        moduleReadings = [];
        moduleReadingsExport = null;
      }
    } catch (reason: unknown) {
      radciteModulesError = `Could not load modules: ${toErrorMessage(reason)}`;
    } finally {
      radciteModulesLoading = false;
    }
  }

  async function refreshModuleReadings(moduleId: string | null = selectedModuleId) {
    if (!moduleId) {
      moduleReadings = [];
      return;
    }

    moduleReadingsLoading = true;
    moduleReadingsError = null;
    try {
      moduleReadings = await listModuleReadings(moduleId);
    } catch (reason: unknown) {
      moduleReadingsError = `Could not load module readings: ${toErrorMessage(reason)}`;
    } finally {
      moduleReadingsLoading = false;
    }
  }

  async function handleSelectModule(moduleId: string) {
    selectedModuleId = moduleId;
    moduleReadingsExport = null;
    moduleReadingsExportError = null;
    await refreshModuleReadings(moduleId);
  }

  async function handleAddRadciteModule(input: Parameters<typeof addRadciteModule>[0]) {
    radciteModulesError = null;
    try {
      const added = await addRadciteModule({
        ...input,
        project_id: selectedProjectCommandId(),
      });
      moduleReadingsExport = null;
      await refreshRadciteModules(added.id);
    } catch (reason: unknown) {
      radciteModulesError = `Could not add module: ${toErrorMessage(reason)}`;
    }
  }

  async function handleUpdateRadciteModule(input: Parameters<typeof updateRadciteModule>[0]) {
    radciteModulesError = null;
    try {
      const updated = await updateRadciteModule(input);
      moduleReadingsExport = null;
      await refreshRadciteModules(updated.id);
    } catch (reason: unknown) {
      radciteModulesError = `Could not update module: ${toErrorMessage(reason)}`;
    }
  }

  async function handleArchiveRadciteModule(moduleId: string) {
    radciteModulesError = null;
    moduleReadingsError = null;
    try {
      const archived = await archiveRadciteModule(moduleId);
      moduleReadingsExport = null;
      await refreshRadciteModules(selectedModuleId === archived.id ? null : selectedModuleId);
    } catch (reason: unknown) {
      radciteModulesError = `Could not remove module: ${toErrorMessage(reason)}`;
    }
  }

  async function handleAddModuleReading(input: Parameters<typeof addModuleReading>[0]) {
    moduleReadingsError = null;
    try {
      const added = await addModuleReading(input);
      moduleReadingsExport = null;
      await refreshModuleReadings(added.module_id);
    } catch (reason: unknown) {
      moduleReadingsError = `Could not add reading: ${toErrorMessage(reason)}`;
    }
  }

  async function handleUpdateModuleReading(input: Parameters<typeof updateModuleReading>[0]) {
    moduleReadingsError = null;
    try {
      const updated = await updateModuleReading(input);
      moduleReadingsExport = null;
      await refreshModuleReadings(updated.module_id);
    } catch (reason: unknown) {
      moduleReadingsError = `Could not update reading: ${toErrorMessage(reason)}`;
    }
  }

  async function handleArchiveModuleReading(readingId: string) {
    moduleReadingsError = null;
    try {
      const archived = await archiveModuleReading(readingId);
      moduleReadingsExport = null;
      await refreshModuleReadings(archived.module_id);
    } catch (reason: unknown) {
      moduleReadingsError = `Could not remove reading: ${toErrorMessage(reason)}`;
    }
  }

  async function handlePreviewModuleReadingsImport(
    input: Parameters<typeof previewModuleReadingsImport>[0],
  ) {
    moduleReadingsError = null;
    return previewModuleReadingsImport(input);
  }

  async function handlePreviewModuleReadingsCsvImport(
    input: Parameters<typeof previewModuleReadingsCsvImport>[0],
  ) {
    moduleReadingsError = null;
    return previewModuleReadingsCsvImport(input);
  }

  async function handleSaveModuleReadingsImport(
    input: Parameters<typeof saveModuleReadingsImport>[0],
  ) {
    moduleReadingsError = null;
    try {
      const saved = await saveModuleReadingsImport(input);
      moduleReadingsExport = null;
      await refreshModuleReadings(selectedModuleId ?? saved[0]?.module_id ?? null);
      return saved;
    } catch (reason: unknown) {
      moduleReadingsError = `Could not save imported readings: ${toErrorMessage(reason)}`;
      throw reason;
    }
  }

  async function handleAddCourseReference(apaCitation: string, notes: string | null) {
    courseReferencesError = null;
    try {
      await addCourseReference({
        project_id: selectedProjectCommandId(),
        apa_citation: apaCitation,
        notes,
      });
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
      referencesExport = await exportCourseReferences({
        project_id: selectedProjectCommandId(),
        for_ako_learn: forAkoLearn,
      });
    } catch (reason: unknown) {
      referencesExportError = `Could not export course references: ${toErrorMessage(reason)}`;
    } finally {
      referencesExportLoading = false;
    }
  }

  async function handleExportModuleReadings(moduleId: string, forAkoLearn: boolean) {
    moduleReadingsExportLoading = true;
    moduleReadingsExportError = null;
    try {
      moduleReadingsExport = await exportModuleReadings({
        module_id: moduleId,
        for_ako_learn: forAkoLearn,
      });
    } catch (reason: unknown) {
      moduleReadingsExportError = `Could not export module readings: ${toErrorMessage(reason)}`;
    } finally {
      moduleReadingsExportLoading = false;
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
    void refreshProjects().then(() => {
      void refreshSavedReviews();
      void refreshCourseReferences();
    });
  });
</script>

<main class="app-shell" data-theme={theme}>
  <ProjectSidebar
    {projects}
    {selectedProjectId}
    {activeArea}
    {projectsLoading}
    {projectsError}
    onSelectProject={(projectId) => {
      void handleSelectProject(projectId);
    }}
    onCreateProject={(input) => {
      void handleCreateProject(input);
    }}
    onSelectArea={(area) => {
      activeArea = area;
      selectedParagraphId = null;
      if (area === "references" || area === "exports") {
        void refreshCourseReferences();
      }
      if (area === "readings" || area === "exports") {
        void refreshRadciteModules();
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
        selectedProjectId={selectedProjectCommandId()}
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
    {:else if activeArea === "readings"}
      <RadciteReadingsWorkspace
        modules={radciteModules}
        {selectedModuleId}
        readings={moduleReadings}
        modulesLoading={radciteModulesLoading}
        readingsLoading={moduleReadingsLoading}
        modulesError={radciteModulesError}
        readingsError={moduleReadingsError}
        onRefreshModules={() => {
          void refreshRadciteModules();
        }}
        onSelectModule={(moduleId) => {
          void handleSelectModule(moduleId);
        }}
        onAddModule={(input) => {
          void handleAddRadciteModule(input);
        }}
        onUpdateModule={(input) => {
          void handleUpdateRadciteModule(input);
        }}
        onArchiveModule={(moduleId) => {
          void handleArchiveRadciteModule(moduleId);
        }}
        onAddReading={(input) => {
          void handleAddModuleReading(input);
        }}
        onUpdateReading={(input) => {
          void handleUpdateModuleReading(input);
        }}
        onArchiveReading={(readingId) => {
          void handleArchiveModuleReading(readingId);
        }}
        onPreviewReadingsImport={handlePreviewModuleReadingsImport}
        onPreviewReadingsCsvImport={handlePreviewModuleReadingsCsvImport}
        onSaveReadingsImport={handleSaveModuleReadingsImport}
      />
    {:else if activeArea === "exports"}
      <RadciteExportsWorkspace
        references={courseReferences}
        modules={radciteModules}
        {selectedModuleId}
        moduleReadings={moduleReadings}
        referencesLoading={courseReferencesLoading}
        modulesLoading={radciteModulesLoading}
        readingsLoading={moduleReadingsLoading}
        referenceExportResult={referencesExport}
        referenceExportLoading={referencesExportLoading}
        referenceExportError={referencesExportError}
        moduleExportResult={moduleReadingsExport}
        moduleExportLoading={moduleReadingsExportLoading}
        moduleExportError={moduleReadingsExportError}
        onExportReferences={(forAkoLearn) => {
          void handleExportCourseReferences(forAkoLearn);
        }}
        onExportModuleReadings={(moduleId, forAkoLearn) => {
          void handleExportModuleReadings(moduleId, forAkoLearn);
        }}
        onRefreshReferences={() => {
          void refreshCourseReferences();
        }}
        onRefreshModules={() => {
          void refreshRadciteModules();
        }}
        onSelectModule={(moduleId) => {
          void handleSelectModule(moduleId);
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
