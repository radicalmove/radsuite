<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CitationActionsPanel from "./components/CitationActionsPanel.svelte";
  import HelpModal from "./components/HelpModal.svelte";
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
    importLegacyRadciteDatabase,
    listRadciteProjects,
    restoreRadciteProject,
    updateRadciteProject,
  } from "./lib/projectCommands";
  import {
    browserStorage,
    readThemeStorage,
    writeThemeStorage,
  } from "./lib/storage";
  import { showsCitationActions } from "./lib/workspaceLayout";
  import { displayAppVersion } from "./lib/appVersion";
  import { setupLocalRuntimes } from "./lib/runtimeSetup";
  import { installUpdate, updaterApi, type UpdateProgress } from "./lib/updateCommands";
  import {
    UPDATE_CHECK_INTERVAL_MS,
    dismissUpdateVersion,
    readUpdateStorageState,
    recordUpdateCheck,
    shouldCheckForUpdate,
    shouldShowUpdateVersion,
  } from "./lib/updateState";
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
    assignCourseReferenceModule,
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
    version: "0.2.2",
    database_ready: false,
    sync_configured: false,
    engines: [],
  };
  const emptyProject: ProjectNavItem = {
    id: "no-project",
    code: "",
    title: "No project selected",
    description: null,
    structureMode: "modules",
    archived_at: null,
  };

  let status = $state<AppStatus>(fallbackStatus);
  let bridgeError = $state<string | null>(null);
  let projects = $state<ProjectNavItem[]>([]);
  let projectsLoading = $state(false);
  let projectsError = $state<string | null>(null);
  let selectedProjectId = $state("");
  let activeArea = $state<ToolArea>("documents");
  let documentSource = $state<"docx" | "pdf">("docx");
  let sharedDocxPath = $state("");
  let analysedReadingsPath = $state("");
  let analysedReadingsSource = $state<"docx" | "pdf" | null>(null);
  let documentModuleId = $state<string | null>(null);
  let analysisResult = $state<AnalyseDocxReviewResponse | null>(null);
  let activeFilter = $state<ParagraphFilter>("all");
  let selectedParagraphId = $state<string | null>(null);
  let theme = $state<"light" | "dark">("light");
  let helpOpen = $state(false);
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
  let runtimeSetupRunning = $state(false);
  let runtimeSetupStatus = $state<string | null>(null);
  let runtimeSetupError = $state<string | null>(null);
  let updateChecking = $state(false);
  let availableUpdate = $state<Awaited<ReturnType<typeof updaterApi.check>>>(null);
  let updateInstalling = $state(false);
  let updateProgress = $state<UpdateProgress | null>(null);
  let updateError = $state<string | null>(null);

  let selectedProject = $derived(
    projects.find((project) => project.id === selectedProjectId) ?? projects[0] ?? emptyProject,
  );
  let selectedParagraph = $derived<ReviewParagraph | null>(
    analysisResult?.paragraphs.find((paragraph) => paragraph.id === selectedParagraphId) ?? null,
  );

  function toErrorMessage(reason: unknown): string {
    return reason instanceof Error ? reason.message : String(reason);
  }

  function handleAnalysisResult(result: AnalyseDocxReviewResponse | null) {
    analysisResult = result;
    documentModuleId = result?.module_id ?? null;
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
    documentModuleId = null;
  }

  function selectedProjectCommandId(): string | null {
    return selectedProjectId || null;
  }

  function projectNavItem(project: {
    id: string;
    code: string | null;
    title: string;
    description: string | null;
    structure_mode: "modules" | "weeks";
    archived_at: string | null;
  }): ProjectNavItem {
    return {
      id: project.id,
      code: project.code ?? "RADcite",
      title: project.title,
      description: project.description,
      structureMode: project.structure_mode,
      archived_at: project.archived_at,
    };
  }

  function resetProjectScopedState() {
    sharedDocxPath = "";
    analysedReadingsPath = "";
    analysedReadingsSource = null;
    analysisResult = null;
    documentModuleId = null;
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
      projects = nextProjects;
      selectedProjectId =
        (preferredProjectId && nextProjects.some((project) => project.id === preferredProjectId)
          ? preferredProjectId
          : nextProjects.find((project) => project.archived_at === null)?.id ??
            nextProjects[0]?.id) ?? "";
    } catch (reason: unknown) {
      projectsError = `Could not load projects: ${toErrorMessage(reason)}`;
      projects = [];
      selectedProjectId = "";
    } finally {
      projectsLoading = false;
    }
  }

  async function handleSetupLocalRuntimes() {
    if (runtimeSetupRunning) {
      return;
    }
    runtimeSetupRunning = true;
    runtimeSetupStatus = null;
    runtimeSetupError = null;
    try {
      runtimeSetupStatus = await setupLocalRuntimes();
      status = await invoke<AppStatus>("get_app_status");
    } catch (reason: unknown) {
      runtimeSetupError = `Could not prepare local audio and voice tools: ${toErrorMessage(reason)}`;
    } finally {
      runtimeSetupRunning = false;
    }
  }

  async function handleCreateProject(input: Parameters<typeof createRadciteProject>[0]) {
    projectsError = null;
    try {
      const created = await createRadciteProject(input);
      await refreshProjects(created.id);
      resetProjectScopedState();
      await refreshProjectScopedData();
    } catch (reason: unknown) {
      projectsError = `Could not create project: ${toErrorMessage(reason)}`;
      throw reason;
    }
  }

  async function handleImportLegacyDatabase(path: string) {
    projectsError = null;
    try {
      const result = await importLegacyRadciteDatabase(path);
      await refreshProjects();
      await refreshProjectScopedData();
      return result;
    } catch (reason: unknown) {
      projectsError = `Could not import old RADcite data: ${toErrorMessage(reason)}`;
      throw reason;
    }
  }

  async function handleUpdateProject(
    projectId: string,
    input: Parameters<typeof updateRadciteProject>[1],
  ) {
    projectsError = null;
    try {
      const updated = await updateRadciteProject(projectId, input);
      await refreshProjects(updated.id);
    } catch (reason: unknown) {
      projectsError = `Could not update project: ${toErrorMessage(reason)}`;
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
    if (activeArea === "documents") {
      await refreshRadciteModules(null);
    }
  }

  async function refreshSavedReviews() {
    if (!selectedProjectId) {
      savedReviews = [];
      return;
    }
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
    if (!selectedProjectId) {
      archiveItems = [];
      return;
    }
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
        module_id: updated.module_id,
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
      documentModuleId = loaded.module_id;
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

  async function handleOpenReferenceCitation(documentId: string, paragraphId: string) {
    activeArea = "documents";
    activeFilter = "all";
    if (analysisResult?.document_id !== documentId) {
      await handleLoadSavedReview(documentId);
    }
    if (analysisResult?.document_id === documentId) {
      selectedParagraphId = paragraphId;
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
    documentModuleId = review.module_id;
    activeArea = "readings";
    selectedParagraphId = null;
    await refreshRadciteModules(documentModuleId);
  }

  async function handleOpenReadingsFromDocument() {
    activeArea = "readings";
    selectedParagraphId = null;
    await refreshRadciteModules(documentModuleId ?? analysisResult?.module_id ?? null);
  }

  async function handleImportDetectedReadings(
    sourcePathOverride?: string,
    sourceFileTypeOverride?: "docx" | "pdf",
  ) {
    const path = sourcePathOverride?.trim() || analysedReadingsPath.trim() || sharedDocxPath.trim();
    const sourceFileType = sourceFileTypeOverride ?? analysedReadingsSource ?? documentSource;
    if (!path) {
      throw new Error("Analyse a document before importing detected readings.");
    }

    const result = await importDocumentReadings({
      project_id: selectedProjectCommandId(),
      module_id: documentModuleId ?? analysisResult?.module_id ?? null,
      path,
      source_file_type: sourceFileType,
    });
    await refreshRadciteModules();
    return result;
  }

  async function refreshCourseReferences() {
    if (!selectedProjectId) {
      courseReferences = [];
      return;
    }
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
    if (!selectedProjectId) {
      radciteModules = [];
      selectedModuleId = null;
      moduleReadings = [];
      return;
    }
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

  async function handleAssignCourseReferenceModule(
    referenceId: string,
    moduleId: string | null,
  ): Promise<boolean> {
    courseReferencesError = null;
    try {
      await assignCourseReferenceModule({
        reference_id: referenceId,
        module_id: moduleId,
      });
      referencesExport = null;
      await refreshCourseReferences();
      return true;
    } catch (reason: unknown) {
      courseReferencesError = `Could not assign course reference module: ${toErrorMessage(reason)}`;
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
      await Promise.all([refreshSavedReviews(), refreshCourseReferences()]);
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

  async function checkForStableUpdate(force = false) {
    const storage = browserStorage();
    const updateState = readUpdateStorageState(storage);
    if (!force && !shouldCheckForUpdate(Date.now(), updateState.lastCheckedAt)) {
      return;
    }

    updateChecking = true;
    updateError = null;
    try {
      const update = await updaterApi.check();
      recordUpdateCheck(storage, Date.now());
      availableUpdate = shouldShowUpdateVersion(update?.version, updateState.dismissedVersion, true)
        ? update
        : null;
    } catch (reason: unknown) {
      updateError = `Could not check for updates: ${toErrorMessage(reason)}`;
    } finally {
      updateChecking = false;
    }
  }

  function deferStableUpdate() {
    if (!availableUpdate) return;
    dismissUpdateVersion(browserStorage(), availableUpdate.version);
    availableUpdate = null;
  }

  async function applyStableUpdate() {
    if (!availableUpdate) return;
    updateInstalling = true;
    updateProgress = null;
    updateError = null;
    try {
      await installUpdate(availableUpdate, (progress) => {
        updateProgress = progress;
      });
    } catch (reason: unknown) {
      updateError = `Could not install the update: ${toErrorMessage(reason)}`;
      updateInstalling = false;
    }
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
    void refreshProjects().then(() => refreshProjectScopedData());
    void checkForStableUpdate();
    const updateTimer = window.setInterval(
      () => void checkForStableUpdate(),
      UPDATE_CHECK_INTERVAL_MS,
    );
    return () => window.clearInterval(updateTimer);
  });
</script>

<main
  class="app-shell"
  class:has-actions-panel={showsCitationActions(activeArea)}
  data-theme={theme}
>
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
    onImportLegacyDatabase={(path) => {
      return handleImportLegacyDatabase(path);
    }}
    onUpdateProject={(projectId, input) => {
      void handleUpdateProject(projectId, input);
    }}
    onSelectArea={(area) => {
      activeArea = area;
      selectedParagraphId = null;
      if (!selectedProjectId) {
        return;
      }
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
        <h2>{selectedProjectId ? `${selectedProject.code} · ${selectedProject.title}` : "Create a project to begin"}</h2>
      </div>
      <div class="status-strip" aria-label="Application status">
        <span
          class="version-chip"
          title={`RADsuite application version ${displayAppVersion(status.version)}`}
          aria-label={`RADsuite application version ${displayAppVersion(status.version)}`}
        >
          {displayAppVersion(status.version)}
        </span>
        <span
          class="status-chip"
          class:is-ready={status.database_ready}
          title={status.database_ready
            ? "Your work is saved locally and remains available offline."
            : "RADsuite cannot currently save your work locally."}
          aria-label={status.database_ready
            ? "Saved locally"
            : "Local saving unavailable"}
        >
          <span class="status-dot"></span>
          <span>{status.database_ready ? "Saved locally" : "Local saving unavailable"}</span>
        </span>
        <span
          class="status-chip"
          class:is-ready={status.sync_configured}
          title={status.sync_configured
            ? "Cloud backup is connected."
            : "Cloud backup is off; your work remains saved locally and is not copied to another device."}
          aria-label={status.sync_configured ? "Cloud backup on" : "Cloud backup off"}
        >
          <span class="status-dot"></span>
          <span>{status.sync_configured ? "Cloud backup on" : "Cloud backup off"}</span>
        </span>
        <button
          class="help-button"
          type="button"
          aria-label="Open help"
          title="Open help"
          onclick={() => (helpOpen = true)}
        >
          Help
        </button>
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

    <HelpModal open={helpOpen} onClose={() => (helpOpen = false)} />

    {#if bridgeError}
      <div class="notice">Command bridge unavailable: {bridgeError}</div>
    {/if}
    {#if reviewActionError}
      <div class="notice">{reviewActionError}</div>
    {/if}
    {#if runtimeSetupError}
      <div class="notice">{runtimeSetupError}</div>
    {/if}
    {#if availableUpdate}
      <div class="update-notice" role="status">
        <div>
          <strong>RADsuite {displayAppVersion(availableUpdate.version)} is available</strong>
          <span>{availableUpdate.body || "A stable update is ready to install."}</span>
          {#if updateProgress}
            <small>
              Downloaded {Math.round(updateProgress.downloadedBytes / 1024 / 1024)} MB
              {#if updateProgress.totalBytes}
                of {Math.round(updateProgress.totalBytes / 1024 / 1024)} MB
              {/if}
            </small>
          {/if}
        </div>
        <div class="update-actions">
          <button class="primary-button compact-button" type="button" disabled={updateInstalling} onclick={() => void applyStableUpdate()}>
            {updateInstalling ? "Installing..." : "Update now"}
          </button>
          <button class="secondary-button compact-button" type="button" disabled={updateInstalling} onclick={deferStableUpdate}>Later</button>
        </div>
      </div>
    {/if}
    {#if updateError}
      <div class="notice">{updateError}</div>
    {/if}

    {#if !selectedProjectId}
      <section class="workspace-placeholder">
        <p class="eyebrow">No project selected</p>
        <h2>Create a project to get started</h2>
        <span>Use the + button in the Projects panel to create a course workspace.</span>
        <div class="runtime-setup-panel">
          <strong>Prepare local audio and voice tools</strong>
          <span>RADcast and RADTTS will be installed on this computer. Larger models download only when first used.</span>
          <button class="primary-button" type="button" disabled={runtimeSetupRunning} onclick={() => void handleSetupLocalRuntimes()}>
            {runtimeSetupRunning ? "Preparing local tools..." : "Prepare audio and voice tools"}
          </button>
          {#if runtimeSetupStatus}
            <small>{runtimeSetupStatus}</small>
          {/if}
        </div>
      </section>
    {:else if activeArea === "documents"}
      <RadciteDocumentsWorkspace
        selectedProjectId={selectedProjectCommandId()}
        modules={radciteModules}
        documentModuleId={documentModuleId}
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
        onDocumentModuleChange={(moduleId) => {
          documentModuleId = moduleId;
        }}
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
        modules={radciteModules}
        references={courseReferences}
        referencesLoading={courseReferencesLoading}
        referencesError={courseReferencesError}
        onAddReference={handleAddCourseReference}
        onUpdateReference={handleUpdateCourseReference}
        onAssignReferenceModule={handleAssignCourseReferenceModule}
        onArchiveReference={handleArchiveCourseReference}
        onMergeReferences={handleMergeCourseReferences}
        onRefreshReferences={() => {
          void refreshCourseReferences();
        }}
        onOpenCitation={(documentId, paragraphId) => {
          void handleOpenReferenceCitation(documentId, paragraphId);
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
      <RadtTsWorkspace
        selectedProjectId={selectedProjectCommandId()}
        onPrepareLocalTools={handleSetupLocalRuntimes}
        preparingLocalTools={runtimeSetupRunning}
      />
    {:else if activeArea === "radt-tools"}
      <RadtTsToolsWorkspace
        selectedProjectId={selectedProjectCommandId()}
        onPrepareLocalTools={handleSetupLocalRuntimes}
        preparingLocalTools={runtimeSetupRunning}
      />
    {:else}
      <section class="workspace-placeholder">
        <p class="eyebrow">Coming later</p>
        <h2>{activeArea}</h2>
        <span>This area will be connected after the document review workspace is stable.</span>
      </section>
    {/if}
  </section>

  {#if showsCitationActions(activeArea)}
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
  {/if}
</main>
