<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CitationActionsPanel from "./components/CitationActionsPanel.svelte";
  import ProjectSidebar from "./components/ProjectSidebar.svelte";
  import RadcastWorkspace from "./components/RadcastWorkspace.svelte";
  import RadtTsWorkspace from "./components/RadtTsWorkspace.svelte";
  import RadtTsToolsWorkspace from "./components/RadtTsToolsWorkspace.svelte";
  import RadciteArchiveWorkspace from "./components/RadciteArchiveWorkspace.svelte";
  import RadciteDocumentsWorkspace from "./components/RadciteDocumentsWorkspace.svelte";
  import RadciteExportsWorkspace from "./components/RadciteExportsWorkspace.svelte";
  import RadciteReferencesWorkspace from "./components/RadciteReferencesWorkspace.svelte";
  import RadciteReadingsWorkspace from "./components/RadciteReadingsWorkspace.svelte";
  import moonIcon from "./assets/moon.png";
  import {
    archiveRadciteDocument,
    listRadciteArchive,
    restoreRadciteArchiveItem,
  } from "./lib/archiveCommands";
  import { exportCourseReferences, exportModuleReadings } from "./lib/exportCommands";
  import {
    updateRadciteDocument,
    type UpdateRadciteDocumentInput,
  } from "./lib/documentCommands";
  import {
    archiveRadciteProject,
    createRadciteProject,
    listRadciteProjects,
    restoreRadciteProject,
  } from "./lib/projectCommands";
  import {
    browserStorage,
    readThemeStorage,
    writeThemeStorage,
  } from "./lib/storage";
  import {
    addModuleReading,
    addRadciteModule,
    archiveModuleReading,
    archiveRadciteModule,
    importDocumentReadings,
    listModuleReadings,
    listRadciteModules,
    previewModuleReadingsCsvImport,
    previewModuleReadingsImport,
    previewModuleReadingsPdfImport,
    saveModuleReadingsImport,
    updateModuleReading,
    updateRadciteModule,
  } from "./lib/readingCommands";
  import {
    addCourseReference,
    archiveCourseReference,
    listCourseReferences,
    mergeCourseReferences,
    updateCourseReference,
    type UpdateCourseReferenceInput,
  } from "./lib/referenceCommands";
  import {
    persistAddManualCitation,
    persistLinkCitationToReference,
    persistMarkParagraphResolved,
    persistVerifyParagraphCitations,
  } from "./lib/reviewActionCommands";
  import {
    canUseSavedReviewForReadings,
    listSavedRadciteReviews,
    loadSavedRadciteReview,
  } from "./lib/savedReviewCommands";
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
    RadciteArchiveItem,
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
  const fallbackProject: ProjectNavItem = {
    id: "radcite-fallback",
    code: "CRJU150",
    title: "RADcite Functional Testing",
    structureMode: "modules",
    archived_at: null,
  };

  let status = $state<AppStatus>(fallbackStatus);
  let bridgeError = $state<string | null>(null);
  let projects = $state<ProjectNavItem[]>([fallbackProject]);
  let projectsLoading = $state(false);
  let projectsError = $state<string | null>(null);
  let selectedProjectId = $state(fallbackProject.id);
  let activeArea = $state<ToolArea>("documents");
  let documentSource = $state<"docx" | "pdf">("docx");
  let sharedDocxPath = $state("");
  let analysedReadingsPath = $state("");
  let analysedReadingsSource = $state<"docx" | "pdf" | null>(null);
  let analysisResult = $state<AnalyseDocxReviewResponse | null>(null);
  let activeFilter = $state<ParagraphFilter>("all");
  let selectedParagraphId = $state<string | null>(null);
  let theme = $state<"light" | "dark">("light");
  let reviewActionError = $state<string | null>(null);
  let savedReviews = $state<SavedRadciteReviewSummary[]>([]);
  let savedReviewsLoading = $state(false);
  let savedReviewsError = $state<string | null>(null);
  let archiveItems = $state<RadciteArchiveItem[]>([]);
  let archiveLoading = $state(false);
  let archiveError = $state<string | null>(null);
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
      analysedReadingsPath = result.source_path?.trim() || sharedDocxPath.trim();
      analysedReadingsSource = result.source_file_type;
    } else {
      analysedReadingsPath = "";
      analysedReadingsSource = null;
    }
    if (result) {
      void refreshSavedReviews();
    }
  }

  function handleDocumentSourceChange(source: "docx" | "pdf") {
    documentSource = source;
    sharedDocxPath = "";
    analysedReadingsPath = "";
    analysedReadingsSource = null;
  }

  function selectedProjectCommandId(): string | null {
    return selectedProjectId === fallbackProject.id ? null : selectedProjectId;
  }

  function projectNavItem(project: {
    id: string;
    code: string | null;
    title: string;
    archived_at: string | null;
  }): ProjectNavItem {
    return {
      id: project.id,
      code: project.code ?? "RADcite",
      title: project.title,
      structureMode: "modules",
      archived_at: project.archived_at,
    };
  }

  function resetProjectScopedState() {
    sharedDocxPath = "";
    analysedReadingsPath = "";
    analysedReadingsSource = null;
    analysisResult = null;
    activeFilter = "all";
    selectedParagraphId = null;
    reviewActionError = null;
    savedReviews = [];
    savedReviewsError = null;
    archiveItems = [];
    archiveError = null;
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
          : nextProjects.find((project) => project.archived_at === null)?.id ??
            nextProjects[0]?.id) ?? fallbackProject.id;
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
      await refreshArchive();
    } catch (reason: unknown) {
      projectsError = `Could not create project: ${toErrorMessage(reason)}`;
      throw reason;
    }
  }

  async function refreshProjectScopedData() {
    resetProjectScopedState();
    await Promise.all([
      refreshSavedReviews(),
      refreshCourseReferences(),
      refreshArchive(),
      refreshRadciteModules(null),
    ]);
  }

  async function handleArchiveProject(projectId: string) {
    const previousSelectedProjectId = selectedProjectId;
    projectsError = null;
    try {
      await archiveRadciteProject(projectId);
      await refreshProjects(previousSelectedProjectId === projectId ? null : previousSelectedProjectId);
      if (selectedProjectId !== previousSelectedProjectId) {
        await refreshProjectScopedData();
      }
    } catch (reason: unknown) {
      projectsError = `Could not archive project: ${toErrorMessage(reason)}`;
    }
  }

  async function handleRestoreProject(projectId: string) {
    const previousSelectedProjectId = selectedProjectId;
    projectsError = null;
    try {
      await restoreRadciteProject(projectId);
      await refreshProjects(projectId);
      if (selectedProjectId !== previousSelectedProjectId) {
        await refreshProjectScopedData();
      }
    } catch (reason: unknown) {
      projectsError = `Could not restore project: ${toErrorMessage(reason)}`;
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
    if (activeArea === "archive") {
      await refreshArchive();
    }
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

  async function refreshArchive() {
    archiveLoading = true;
    archiveError = null;
    try {
      archiveItems = await listRadciteArchive(selectedProjectCommandId());
    } catch (reason: unknown) {
      archiveError = `Could not load archive: ${toErrorMessage(reason)}`;
    } finally {
      archiveLoading = false;
    }
  }

  async function handleArchiveDocument(documentId: string) {
    savedReviewsError = null;
    try {
      await archiveRadciteDocument(documentId, selectedProjectCommandId());
      analysisResult = null;
      selectedParagraphId = null;
      await refreshSavedReviews();
      await refreshArchive();
    } catch (reason: unknown) {
      savedReviewsError = `Could not archive document: ${toErrorMessage(reason)}`;
    }
  }

  async function handleUpdateDocument(
    input: UpdateRadciteDocumentInput,
  ): Promise<SavedRadciteReviewSummary> {
    const updated = await updateRadciteDocument(input);
    if (analysisResult?.document_id === input.document_id) {
      analysisResult = {
        ...analysisResult,
        display_name: updated.display_name,
        doc_variant: updated.doc_variant,
        doc_number: updated.doc_number,
        exclude_from_references: updated.exclude_from_references,
      };
    }
    await Promise.all([
      refreshSavedReviews(),
      refreshCourseReferences(),
      refreshRadciteModules(null),
    ]);
    return updated;
  }

  async function handleRestoreArchiveItem(item: RadciteArchiveItem) {
    archiveError = null;
    try {
      await restoreRadciteArchiveItem({
        project_id: selectedProjectCommandId(),
        kind: item.kind,
        item_id: item.id,
      });
      await Promise.all([
        refreshArchive(),
        refreshSavedReviews(),
        refreshCourseReferences(),
        refreshRadciteModules(null),
      ]);
    } catch (reason: unknown) {
      archiveError = `Could not restore item: ${toErrorMessage(reason)}`;
    }
  }

  async function handleLoadSavedReview(documentId: string) {
    savedReviewsError = null;
    reviewActionError = null;
    selectedParagraphId = null;
    analysedReadingsPath = "";
    analysedReadingsSource = null;
    try {
      const loaded = await loadSavedRadciteReview(documentId);
      analysisResult = loaded;
      activeFilter = "all";
      documentSource = loaded.source_file_type;
      const sourcePath = loaded.source_path?.trim() ?? "";
      sharedDocxPath = sourcePath;
      analysedReadingsPath = sourcePath;
      analysedReadingsSource = loaded.source_file_type;
    } catch (reason: unknown) {
      savedReviewsError = `Could not open saved review: ${toErrorMessage(reason)}`;
    }
  }

  async function handleUseSavedReviewForReadings(review: SavedRadciteReviewSummary) {
    if (!canUseSavedReviewForReadings(review)) {
      return;
    }

    const sourcePath = review.source_path?.trim();
    if (!sourcePath) {
      return;
    }

    sharedDocxPath = sourcePath;
    analysedReadingsPath = sourcePath;
    analysedReadingsSource = review.source_file_type;
    documentSource = review.source_file_type;
    activeArea = "readings";
    selectedParagraphId = null;
    await refreshRadciteModules();
  }

  async function handleOpenReadingsFromDocument() {
    activeArea = "readings";
    selectedParagraphId = null;
    await refreshRadciteModules();
  }

  async function handleImportDetectedReadings() {
    const path = analysedReadingsPath.trim() || sharedDocxPath.trim();
    const sourceFileType = analysedReadingsSource ?? documentSource;
    if (!path) {
      throw new Error("Analyse a document before importing detected readings.");
    }

    const result = await importDocumentReadings({
      project_id: selectedProjectCommandId(),
      path,
      source_file_type: sourceFileType,
    });
    await refreshRadciteModules();
    return result;
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
      return added;
    } catch (reason: unknown) {
      radciteModulesError = `Could not add module: ${toErrorMessage(reason)}`;
      throw reason;
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

  async function handleUpdateModuleReading(
    input: Parameters<typeof updateModuleReading>[0],
  ): Promise<boolean> {
    moduleReadingsError = null;
    try {
      const updated = await updateModuleReading(input);
      moduleReadingsExport = null;
      await refreshModuleReadings(updated.module_id);
      return true;
    } catch (reason: unknown) {
      moduleReadingsError = `Could not update reading: ${toErrorMessage(reason)}`;
      return false;
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

  async function handlePreviewModuleReadingsPdfImport(
    input: Parameters<typeof previewModuleReadingsPdfImport>[0],
  ) {
    moduleReadingsError = null;
    return previewModuleReadingsPdfImport(input);
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
      const added = await addCourseReference({
        project_id: selectedProjectCommandId(),
        apa_citation: apaCitation,
        notes,
      });
      referencesExport = null;
      await refreshCourseReferences();
      return added;
    } catch (reason: unknown) {
      courseReferencesError = `Could not add course reference: ${toErrorMessage(reason)}`;
      return null;
    }
  }

  async function handleUpdateCourseReference(input: UpdateCourseReferenceInput): Promise<boolean> {
    courseReferencesError = null;
    try {
      await updateCourseReference(input);
      referencesExport = null;
      await refreshCourseReferences();
      return true;
    } catch (reason: unknown) {
      courseReferencesError = `Could not update course reference: ${toErrorMessage(reason)}`;
      return false;
    }
  }

  async function handleArchiveCourseReference(referenceId: string) {
    courseReferencesError = null;
    try {
      await archiveCourseReference(referenceId);
      referencesExport = null;
      await refreshCourseReferences();
    } catch (reason: unknown) {
      courseReferencesError = `Could not remove course reference: ${toErrorMessage(reason)}`;
    }
  }

  async function handleMergeCourseReferences(
    primaryReferenceId: string,
    mergeReferenceIds: string[],
  ): Promise<boolean> {
    courseReferencesError = null;
    try {
      await mergeCourseReferences({
        primary_reference_id: primaryReferenceId,
        merge_reference_ids: mergeReferenceIds,
      });
      referencesExport = null;
      await refreshCourseReferences();
      return true;
    } catch (reason: unknown) {
      courseReferencesError = `Could not merge course references: ${toErrorMessage(reason)}`;
      return false;
    }
  }

  async function handleExportCourseReferences(
    forAkoLearn: boolean,
    allowIncomplete: boolean,
    useLibraryLinks: boolean,
  ) {
    referencesExportLoading = true;
    referencesExportError = null;
    try {
      referencesExport = await exportCourseReferences({
        project_id: selectedProjectCommandId(),
        for_ako_learn: forAkoLearn,
        allow_incomplete: allowIncomplete,
        use_library_links: useLibraryLinks,
      });
    } catch (reason: unknown) {
      referencesExportError = `Could not export course references: ${toErrorMessage(reason)}`;
    } finally {
      referencesExportLoading = false;
    }
  }

  async function handleExportModuleReadings(
    moduleId: string,
    forAkoLearn: boolean,
    useLibraryLinks: boolean,
  ) {
    moduleReadingsExportLoading = true;
    moduleReadingsExportError = null;
    try {
      moduleReadingsExport = await exportModuleReadings({
        module_id: moduleId,
        for_ako_learn: forAkoLearn,
        use_library_links: useLibraryLinks,
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
    writeThemeStorage(browserStorage(), nextTheme);
  }

  onMount(() => {
    applyTheme(readThemeStorage(browserStorage()));

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
      void refreshArchive();
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
    onArchiveProject={(projectId) => {
      void handleArchiveProject(projectId);
    }}
    onRestoreProject={(projectId) => {
      void handleRestoreProject(projectId);
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
      if (area === "archive") {
        void refreshArchive();
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
        <span
          class="status-chip"
          class:is-ready={status.database_ready}
          title={status.database_ready
            ? "Your work is saved on this Mac and remains available offline."
            : "RADsuite cannot currently save your work on this Mac."}
          aria-label={status.database_ready
            ? "Saved on this Mac"
            : "Local saving unavailable"}
        >
          <span class="status-dot"></span>
          <span>{status.database_ready ? "Saved on this Mac" : "Local saving unavailable"}</span>
        </span>
        <span
          class="status-chip"
          class:is-ready={status.sync_configured}
          title={status.sync_configured
            ? "Cloud sync is connected."
            : "Cloud sync is not connected; your work remains saved on this Mac and is not copied to another device."}
          aria-label={status.sync_configured ? "Cloud sync on" : "Cloud sync not connected"}
        >
          <span class="status-dot"></span>
          <span>{status.sync_configured ? "Cloud sync on" : "Cloud sync not connected"}</span>
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
        {documentSource}
        docxPath={sharedDocxPath}
        {activeFilter}
        {analysisResult}
        {savedReviews}
        {savedReviewsLoading}
        {savedReviewsError}
        {selectedParagraphId}
        selectedDocumentId={analysisResult?.document_id ?? null}
        readingsPath={analysedReadingsPath}
        onFilterChange={(filter) => {
          activeFilter = filter;
          selectedParagraphId = null;
        }}
        onAnalysisResult={handleAnalysisResult}
        onDocumentSourceChange={handleDocumentSourceChange}
        onDocxPathChange={(path) => {
          sharedDocxPath = path;
        }}
        onOpenReadings={() => {
          void handleOpenReadingsFromDocument();
        }}
        onImportDetectedReadings={() => handleImportDetectedReadings()}
        onLoadSavedReview={(documentId) => {
          void handleLoadSavedReview(documentId);
        }}
        onUseForReadings={(review) => {
          void handleUseSavedReviewForReadings(review);
        }}
        onUpdateDocument={(input) => handleUpdateDocument(input)}
        onArchiveDocument={(documentId) => {
          void handleArchiveDocument(documentId);
        }}
        onRefreshSavedReviews={() => {
          void refreshSavedReviews();
        }}
        onSelectParagraph={(paragraphId) => {
          selectedParagraphId = paragraphId;
        }}
      />
    {:else if activeArea === "archive"}
      <RadciteArchiveWorkspace
        items={archiveItems}
        loading={archiveLoading}
        error={archiveError}
        onRefresh={() => {
          void refreshArchive();
        }}
        onRestore={handleRestoreArchiveItem}
      />
    {:else if activeArea === "references"}
      <RadciteReferencesWorkspace
        references={courseReferences}
        referencesLoading={courseReferencesLoading}
        referencesError={courseReferencesError}
        onAddReference={handleAddCourseReference}
        onUpdateReference={handleUpdateCourseReference}
        onArchiveReference={handleArchiveCourseReference}
        onMergeReferences={handleMergeCourseReferences}
        onRefreshReferences={() => {
          void refreshCourseReferences();
        }}
      />
    {:else if activeArea === "readings"}
      <RadciteReadingsWorkspace
        modules={radciteModules}
        docxPath={sharedDocxPath}
        autoPreviewPath={analysedReadingsPath}
        autoPreviewSource={analysedReadingsSource}
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
        onAddModule={handleAddRadciteModule}
        onUpdateModule={(input) => {
          void handleUpdateRadciteModule(input);
        }}
        onArchiveModule={(moduleId) => {
          void handleArchiveRadciteModule(moduleId);
        }}
        onAddReading={(input) => {
          void handleAddModuleReading(input);
        }}
        onUpdateReading={handleUpdateModuleReading}
        onArchiveReading={(readingId) => {
          void handleArchiveModuleReading(readingId);
        }}
        onPreviewReadingsImport={handlePreviewModuleReadingsImport}
        onPreviewReadingsCsvImport={handlePreviewModuleReadingsCsvImport}
        onPreviewReadingsPdfImport={handlePreviewModuleReadingsPdfImport}
        onSaveReadingsImport={handleSaveModuleReadingsImport}
        onDocxPathChange={(path) => {
          sharedDocxPath = path;
        }}
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
        onExportReferences={(forAkoLearn, allowIncomplete, useLibraryLinks) => {
          void handleExportCourseReferences(forAkoLearn, allowIncomplete, useLibraryLinks);
        }}
        onExportModuleReadings={(moduleId, forAkoLearn, useLibraryLinks) => {
          void handleExportModuleReadings(moduleId, forAkoLearn, useLibraryLinks);
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
    {:else if activeArea === "radcast"}
      <RadcastWorkspace selectedProjectId={selectedProjectCommandId()} />
    {:else if activeArea === "radtts"}
      <RadtTsWorkspace selectedProjectId={selectedProjectCommandId()} />
    {:else if activeArea === "radt-tools"}
      <RadtTsToolsWorkspace selectedProjectId={selectedProjectCommandId()} />
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
    onAddCourseReference={handleAddCourseReference}
    onVerifyCitation={handleVerifyCitation}
    onMarkReviewed={handleVerifyCitation}
    onLinkCitation={handleLinkCitation}
  />
</main>
