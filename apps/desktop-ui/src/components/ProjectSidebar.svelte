<script lang="ts">
  import type { ProjectNavItem, ToolArea } from "../types";
  import type { CreateRadciteProjectInput } from "../lib/projectCommands";
  import radciteLogo from "../assets/radcite-logo.svg";

  type Props = {
    projects: ProjectNavItem[];
    selectedProjectId: string;
    activeArea: ToolArea;
    projectsLoading: boolean;
    projectsError: string | null;
    onSelectProject: (projectId: string) => void;
    onCreateProject: (input: CreateRadciteProjectInput) => void | Promise<void>;
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
    onSelectArea,
  }: Props = $props();

  let createOpen = $state(false);
  let projectCode = $state("");
  let projectTitle = $state("");
  let createSubmitting = $state(false);

  const radciteAreas: Array<{ id: ToolArea; label: string; disabled?: boolean }> = [
    { id: "documents", label: "Documents" },
    { id: "references", label: "References" },
    { id: "readings", label: "Readings" },
    { id: "exports", label: "Exports" },
  ];

  let createDisabled = $derived(createSubmitting || projectTitle.trim().length === 0);

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
</script>

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
    <span>Projects</span>
    <button
      class="icon-button"
      type="button"
      aria-label="Create project"
      aria-expanded={createOpen}
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
    {#each projects as project (project.id)}
      <section class="project-card" class:is-active={project.id === selectedProjectId}>
        <button class="project-button" type="button" onclick={() => onSelectProject(project.id)}>
          <strong>{project.code}</strong>
          <span>{project.title}</span>
        </button>

        <div class="tool-group" aria-label={`${project.code} tools`}>
          <div class="tool-heading">RADcite</div>
          <div class="tool-list">
            {#each radciteAreas as area (area.id)}
              <button
                class="tool-area-button"
                class:is-active={project.id === selectedProjectId && activeArea === area.id}
                type="button"
                disabled={area.disabled}
                onclick={() => {
                  onSelectProject(project.id);
                  onSelectArea(area.id);
                }}
              >
                {area.label}
              </button>
            {/each}
          </div>

          <button class="tool-area-button tool-area-button-root media-tool-button" type="button" disabled>
            <span>Audio cleanup</span>
            <small>RADcast</small>
          </button>
          <button class="tool-area-button tool-area-button-root media-tool-button" type="button" disabled>
            <span>Voice generation</span>
            <small>RADTTS</small>
          </button>
        </div>
      </section>
    {/each}
  </div>
</aside>
