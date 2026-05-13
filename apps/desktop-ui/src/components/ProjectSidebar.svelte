<script lang="ts">
  import type { ProjectNavItem, ToolArea } from "../types";
  import radciteLogo from "../assets/radcite-logo.svg";

  type Props = {
    projects: ProjectNavItem[];
    selectedProjectId: string;
    activeArea: ToolArea;
    onSelectProject: (projectId: string) => void;
    onSelectArea: (area: ToolArea) => void;
  };

  let { projects, selectedProjectId, activeArea, onSelectProject, onSelectArea }: Props = $props();

  const radciteAreas: Array<{ id: ToolArea; label: string; disabled?: boolean }> = [
    { id: "documents", label: "Documents" },
    { id: "references", label: "References", disabled: true },
    { id: "readings", label: "Readings", disabled: true },
    { id: "exports", label: "Exports", disabled: true },
  ];
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
    <button class="icon-button" type="button" aria-label="Create project" disabled>+</button>
  </div>

  <div class="project-list">
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
