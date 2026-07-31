import type { ProjectNavItem } from "../types";

export type ProjectNavigationState = {
  selectedProjectId: string | null;
  expandedProjectIds: string[];
  archivedSectionOpen: boolean;
};

export type ProjectMutation = "archive" | "restore";

export function partitionProjects(projects: ProjectNavItem[]): {
  active: ProjectNavItem[];
  archived: ProjectNavItem[];
} {
  return {
    active: projects.filter((project) => project.archived_at === null),
    archived: projects.filter((project) => project.archived_at !== null),
  };
}

export function selectProject(
  state: ProjectNavigationState,
  projectId: string,
  archived: boolean,
): ProjectNavigationState {
  return {
    selectedProjectId: projectId,
    expandedProjectIds: state.expandedProjectIds.includes(projectId)
      ? state.expandedProjectIds
      : [...state.expandedProjectIds, projectId],
    archivedSectionOpen: archived ? true : state.archivedSectionOpen,
  };
}

export function applyProjectMutationNavigation(
  state: ProjectNavigationState,
  projects: ProjectNavItem[],
  mutatedProjectId: string,
  mutation: ProjectMutation,
  succeeded: boolean,
): ProjectNavigationState {
  if (!succeeded) {
    return state;
  }

  const { active, archived } = partitionProjects(projects);
  if (mutation === "restore") {
    return selectProject(
      {
        ...state,
        archivedSectionOpen: archived.length > 0 ? state.archivedSectionOpen : false,
      },
      mutatedProjectId,
      false,
    );
  }

  if (active.length === 0) {
    return selectProject(state, mutatedProjectId, true);
  }

  if (state.selectedProjectId !== mutatedProjectId) {
    return state;
  }

  return selectProject(
    {
      ...state,
      archivedSectionOpen: false,
    },
    active[0].id,
    false,
  );
}
