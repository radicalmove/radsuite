<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { LegacyRadciteImportResult, ProjectNavItem, ToolArea } from "../types";
  import type {
    CreateRadciteProjectInput,
    UpdateRadciteProjectInput,
  } from "../lib/projectCommands";
  import radciteLogo from "../assets/radcite-logo.svg";
  import {
    browserStorage,
    defaultProjectNavStorageState,
    readProjectNavStorageState,
    writeProjectNavStorageState,
    type StorageLike,
  } from "../lib/storage";

  type Props = {
    projects: ProjectNavItem[];
    selectedProjectId: string;
    activeArea: ToolArea;
    projectsLoading: boolean;
    projectsError: string | null;
    onSelectProject: (projectId: string) => void;
    onCreateProject: (input: CreateRadciteProjectInput) => void | Promise<void>;
    onUpdateProject: (projectId: string, input: UpdateRadciteProjectInput) => void | Promise<void>;
    onArchiveProject: (projectId: string) => void | Promise<void>;
    onRestoreProject: (projectId: string) => void | Promise<void>;
    onImportLegacyDatabase: (path: string) => void | Promise<LegacyRadciteImportResult>;
    onSelectArea: (area: ToolArea) => void;
  };

  let {
    projects,
    selectedProjectId,
    activeArea,
    projectsLoading,
    projectsError,
    onSelectProject,
    onCreateProject,
    onUpdateProject,
    onArchiveProject,
    onRestoreProject,
    onImportLegacyDatabase,
    onSelectArea,
  }: Props = $props();

  let createOpen = $state(false);
  let projectCode = $state("");
  let projectTitle = $state("");
  let createSubmitting = $state(false);
  let editProjectId = $state<string | null>(null);
  let editProjectCode = $state("");
  let editProjectTitle = $state("");
  let editProjectDescription = $state("");
  let editProjectStructureMode = $state<"modules" | "weeks">("modules");
  let editSubmitting = $state(false);
  let actionProjectId = $state<string | null>(null);
  let expandedProjectIds = $state<string[]>(defaultProjectNavStorageState.expandedProjectIds);
  let archivedSectionOpen = $state(defaultProjectNavStorageState.archivedSectionOpen);
  let projectStorage = $state<StorageLike | null>(null);
  let storageReady = $state(false);
  let autoExpandedSelectionKey = $state<string | null>(null);
  let legacyImporting = $state(false);
  let legacyImportError = $state<string | null>(null);
  let legacyImportStatus = $state<string | null>(null);
  let legacyImportWarnings = $state<string[]>([]);

  const radciteAreas: Array<{ id: ToolArea; label: string; disabled?: boolean }> = [
    { id: "documents", label: "Documents" },
    { id: "references", label: "References" },
    { id: "readings", label: "Readings" },
    { id: "exports", label: "Exports" },
    { id: "archive", label: "Archive" },
  ];

  let createDisabled = $derived(createSubmitting || projectTitle.trim().length === 0);
  let editDisabled = $derived(editSubmitting || editProjectTitle.trim().length === 0);
  let activeProjects = $derived(projects.filter((project) => project.archived_at === null));
  let archivedProjects = $derived(projects.filter((project) => project.archived_at !== null));

  function isExpanded(projectId: string): boolean {
    return expandedProjectIds.includes(projectId);
  }

  function projectToolLabel(project: ProjectNavItem): string {
    return (isExpanded(project.id) ? "Collapse " : "Expand ") + project.code + " tools";
  }

  function toggleProject(projectId: string) {
    expandedProjectIds = isExpanded(projectId)
      ? expandedProjectIds.filter((id) => id !== projectId)
      : [...expandedProjectIds, projectId];
  }

  function selectProject(project: ProjectNavItem) {
    if (!isExpanded(project.id)) {
      expandedProjectIds = [...expandedProjectIds, project.id];
    }
    if (project.archived_at !== null) {
      archivedSectionOpen = true;
    }
    onSelectProject(project.id);
  }

  function startProjectEdit(project: ProjectNavItem) {
    editProjectId = project.id;
    editProjectCode = project.code === "RADcite" ? "" : project.code;
    editProjectTitle = project.title;
    editProjectDescription = project.description ?? "";
    editProjectStructureMode = project.structureMode;
    if (!isExpanded(project.id)) {
      expandedProjectIds = [...expandedProjectIds, project.id];
    }
  }

  function cancelProjectEdit() {
    editProjectId = null;
    editProjectCode = "";
    editProjectTitle = "";
    editProjectDescription = "";
    editProjectStructureMode = "modules";
  }

  async function submitProjectEdit(projectId: string) {
    if (editDisabled || editProjectId !== projectId) {
      return;
    }

    editSubmitting = true;
    try {
      await onUpdateProject(projectId, {
        code: editProjectCode.trim() || null,
        title: editProjectTitle.trim(),
        description: editProjectDescription.trim() || null,
        structureMode: editProjectStructureMode,
      });
      cancelProjectEdit();
    } finally {
      editSubmitting = false;
    }
  }

  async function archiveProject(projectId: string) {
    actionProjectId = projectId;
    try {
      await onArchiveProject(projectId);
    } finally {
      actionProjectId = null;
    }
  }

  async function restoreProject(projectId: string) {
    actionProjectId = projectId;
    try {
      await onRestoreProject(projectId);
    } finally {
      actionProjectId = null;
    }
  }

  async function submitProject() {
    if (createDisabled) {
      return;
    }

    createSubmitting = true;
    try {
      await onCreateProject({
        code: projectCode.trim() || null,
        title: projectTitle.trim(),
      });
      projectCode = "";
      projectTitle = "";
      createOpen = false;
    } finally {
      createSubmitting = false;
    }
  }

  async function chooseLegacyDatabase() {
    legacyImportError = null;
    legacyImportStatus = null;
    legacyImportWarnings = [];
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [
          {
            name: "SQLite databases",
            extensions: ["db", "sqlite", "sqlite3"],
          },
        ],
      });
      if (typeof selected !== "string") {
        return;
      }

      legacyImporting = true;
      const result = await onImportLegacyDatabase(selected);
      legacyImportStatus = result
        ? `Imported ${result.projects_imported} project(s), ${result.modules_imported} module(s), ${result.documents_imported} document(s), and ${result.references_imported} reference/reading item(s).`
        : "Legacy RADcite data imported into new local projects.";
      legacyImportWarnings = result?.warnings ?? [];
    } catch (reason: unknown) {
      legacyImportError = reason instanceof Error ? reason.message : String(reason);
    } finally {
      legacyImporting = false;
    }
  }

  onMount(() => {
    projectStorage = browserStorage();
    const stored = readProjectNavStorageState(projectStorage);
    expandedProjectIds = stored.expandedProjectIds;
    archivedSectionOpen = stored.archivedSectionOpen;
    storageReady = true;
  });

  $effect(() => {
    const selectedProject = projects.find((project) => project.id === selectedProjectId);
    if (!selectedProject) {
      return;
    }

    const selectionKey = selectedProject.id + ":" + (selectedProject.archived_at !== null ? "archived" : "active");
    if (selectionKey === autoExpandedSelectionKey) {
      return;
    }

    autoExpandedSelectionKey = selectionKey;
    if (!isExpanded(selectedProject.id)) {
      expandedProjectIds = [...expandedProjectIds, selectedProject.id];
    }
    if (selectedProject.archived_at !== null) {
      archivedSectionOpen = true;
    }
  });

  $effect(() => {
    if (!storageReady) {
      return;
    }

    writeProjectNavStorageState(projectStorage, {
      version: 1,
      expandedProjectIds,
      archivedSectionOpen,
    });
  });
</script>

{#snippet projectCard(project: ProjectNavItem)}
  <section
    class="project-card"
    class:is-active={project.id === selectedProjectId}
    class:is-archived={project.archived_at !== null}
  >
    <div class="project-card-header">
      <button class="project-button" type="button" onclick={() => selectProject(project)}>
        <strong>{project.code}</strong>
        <span>{project.title}</span>
      </button>
      <button
        class="project-expand-button"
        type="button"
        aria-label={projectToolLabel(project)}
        aria-expanded={isExpanded(project.id)}
        title={projectToolLabel(project)}
        onclick={() => toggleProject(project.id)}
      >
        <span class="project-chevron" aria-hidden="true"></span>
      </button>
    </div>

    {#if editProjectId === project.id}
      <form
        class="project-edit-form"
        onsubmit={(event) => {
          event.preventDefault();
          void submitProjectEdit(project.id);
        }}
      >
        <label>
          <span>Code</span>
          <input type="text" bind:value={editProjectCode} autocomplete="off" />
        </label>
        <label>
          <span>Title</span>
          <input type="text" bind:value={editProjectTitle} autocomplete="off" />
        </label>
        <label>
          <span>Description</span>
          <textarea bind:value={editProjectDescription} rows="3" maxlength="500"></textarea>
        </label>
        <label>
          <span>Structure</span>
          <select bind:value={editProjectStructureMode}>
            <option value="modules">Modules</option>
            <option value="weeks">Weeks</option>
          </select>
        </label>
        <div class="project-edit-actions">
          <button class="project-action-button" type="button" onclick={cancelProjectEdit}>
            Cancel
          </button>
          <button class="sidebar-create-button" type="submit" disabled={editDisabled}>
            {editSubmitting ? "Saving" : "Save changes"}
          </button>
        </div>
      </form>
    {/if}

    {#if isExpanded(project.id)}
      <div class="tool-group" aria-label={project.code + " tools"}>
        <div class="tool-heading">RADcite</div>
        <div class="tool-list">
          {#each radciteAreas as area (area.id)}
            <button
              class="tool-area-button"
              class:is-active={project.id === selectedProjectId && activeArea === area.id}
              type="button"
              disabled={area.disabled}
              onclick={() => {
                selectProject(project);
                onSelectArea(area.id);
              }}
            >
              {area.label}
            </button>
          {/each}
        </div>

        <button
          class="tool-area-button tool-area-button-root media-tool-button"
          class:is-active={project.id === selectedProjectId && activeArea === "radcast"}
          type="button"
          onclick={() => {
            selectProject(project);
            onSelectArea("radcast");
          }}
        >
          <span>Audio cleanup</span>
          <small>RADcast</small>
        </button>
        <button
          class="tool-area-button tool-area-button-root media-tool-button"
          class:is-active={project.id === selectedProjectId && activeArea === "radtts"}
          type="button"
          onclick={() => {
            selectProject(project);
            onSelectArea("radtts");
          }}
        >
          <span>Voice generation</span>
          <small>RADTTS</small>
        </button>
        <button
          class="tool-area-button tool-area-button-root media-tool-button"
          class:is-active={project.id === selectedProjectId && activeArea === "radt-tools"}
          type="button"
          onclick={() => {
            selectProject(project);
            onSelectArea("radt-tools");
          }}
        >
          <span>Transcribe &amp; clip</span>
          <small>RADTTS</small>
        </button>

        <div class="project-card-actions">
          {#if project.archived_at === null}
            <button
              class="project-action-button"
              type="button"
              disabled={actionProjectId !== null || editSubmitting}
              onclick={() => startProjectEdit(project)}
            >
              Edit
            </button>
            <button
              class="project-action-button"
              type="button"
              disabled={actionProjectId !== null}
              onclick={() => void archiveProject(project.id)}
            >
              Archive
            </button>
          {:else}
            <button
              class="project-action-button"
              type="button"
              disabled={actionProjectId !== null}
              onclick={() => void restoreProject(project.id)}
            >
              Restore
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </section>
{/snippet}

<aside class="project-sidebar" aria-label="Project navigation">
  <div class="sidebar-header">
    <div class="brand-lockup">
      <span class="brand-mark">
        <img src={radciteLogo} alt="" aria-hidden="true" />
      </span>
      <div>
        <p class="eyebrow">Workspace</p>
        <h1>RADsuite</h1>
      </div>
    </div>
    <div class="active-product">RADcite review</div>
  </div>

  <div class="project-section-heading">
    <span>Active projects</span>
    <button
      class="icon-button"
      type="button"
      aria-label="Create project"
      aria-expanded={createOpen}
      title="Create project"
      onclick={() => {
        createOpen = !createOpen;
      }}
    >
      +
    </button>
  </div>

  {#if createOpen}
    <form
      class="project-create-form"
      onsubmit={(event) => {
        event.preventDefault();
        void submitProject();
      }}
    >
      <label>
        <span>Code</span>
        <input type="text" bind:value={projectCode} placeholder="CRJU201" autocomplete="off" />
      </label>
      <label>
        <span>Title</span>
        <input
          type="text"
          bind:value={projectTitle}
          placeholder="Criminological Theory"
          autocomplete="off"
        />
      </label>
      <button class="sidebar-create-button" type="submit" disabled={createDisabled}>
        {createSubmitting ? "Adding" : "Add project"}
      </button>
    </form>
  {/if}

  {#if projectsError}
    <div class="sidebar-notice">{projectsError}</div>
  {/if}

  <div class="project-list">
    {#if projectsLoading}
      <div class="sidebar-notice">Loading projects</div>
    {/if}
    {#if activeProjects.length === 0 && !projectsLoading}
      <div class="project-empty">No active projects</div>
    {/if}
    {#each activeProjects as project (project.id)}
      {@render projectCard(project)}
    {/each}

    <details class="archived-projects-section" bind:open={archivedSectionOpen}>
      <summary>
        <span>Archived projects</span>
        <span class="archived-project-count">{archivedProjects.length}</span>
      </summary>
      {#if archivedProjects.length === 0}
        <div class="project-empty">No archived projects</div>
      {:else}
        <div class="archived-project-list">
          {#each archivedProjects as project (project.id)}
            {@render projectCard(project)}
          {/each}
        </div>
      {/if}
    </details>
  </div>

  <div class="sidebar-data-actions">
    <button
      class="project-action-button sidebar-data-button"
      type="button"
      disabled={legacyImporting}
      aria-busy={legacyImporting}
      onclick={() => void chooseLegacyDatabase()}
    >
      {legacyImporting ? "Importing old RADcite" : "Import old RADcite data"}
    </button>
    <small>Courses, readings, references, and saved reviews are copied locally.</small>
    {#if legacyImportError}
      <div class="sidebar-notice sidebar-data-notice">{legacyImportError}</div>
    {:else if legacyImportStatus}
      <div class="sidebar-success">{legacyImportStatus}</div>
    {/if}
    {#if legacyImportWarnings.length}
      <details class="sidebar-import-warnings">
        <summary>{legacyImportWarnings.length} import warning(s)</summary>
        <ul>
          {#each legacyImportWarnings as warning}
            <li>{warning}</li>
          {/each}
        </ul>
      </details>
    {/if}
  </div>
</aside>
