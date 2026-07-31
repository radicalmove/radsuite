import { describe, expect, test } from "vitest";
import type { ProjectNavItem } from "../types";
import {
  applyProjectMutationNavigation,
  partitionProjects,
  selectProject,
  type ProjectNavigationState,
} from "./projectNavigation";

const activeProject = (id: string, code: string): ProjectNavItem => ({
  id,
  code,
  title: `${code} course`,
  structureMode: "modules",
  archived_at: null,
});

const archivedProject = (id: string, code: string): ProjectNavItem => ({
  ...activeProject(id, code),
  archived_at: "2026-07-31T00:00:00Z",
});

const initialState: ProjectNavigationState = {
  selectedProjectId: "active-1",
  expandedProjectIds: ["active-1"],
  archivedSectionOpen: false,
};

describe("project navigation", () => {
  test("partitions active and archived projects without changing order", () => {
    const projects = [
      activeProject("active-1", "CRJU201"),
      archivedProject("archived-1", "COMS432"),
      activeProject("active-2", "MBIS622"),
    ];

    expect(partitionProjects(projects)).toEqual({
      active: [projects[0], projects[2]],
      archived: [projects[1]],
    });
  });

  test("selecting an archived project opens its section and card", () => {
    expect(selectProject(initialState, "archived-1", true)).toEqual({
      selectedProjectId: "archived-1",
      expandedProjectIds: ["active-1", "archived-1"],
      archivedSectionOpen: true,
    });
  });

  test("archiving the selected project selects the first active project", () => {
    const projects = [activeProject("active-2", "MBIS622"), archivedProject("active-1", "CRJU201")];

    expect(
      applyProjectMutationNavigation(initialState, projects, "active-1", "archive", true),
    ).toEqual({
      selectedProjectId: "active-2",
      expandedProjectIds: ["active-1", "active-2"],
      archivedSectionOpen: false,
    });
  });

  test("archiving the only active project preserves selection and opens Archived", () => {
    const projects = [archivedProject("active-1", "CRJU201")];

    expect(
      applyProjectMutationNavigation(initialState, projects, "active-1", "archive", true),
    ).toEqual({
      selectedProjectId: "active-1",
      expandedProjectIds: ["active-1"],
      archivedSectionOpen: true,
    });
  });

  test("restoring a project selects and expands it", () => {
    const projects = [activeProject("restored-1", "CRJU201"), archivedProject("archived-2", "COMS432")];

    expect(
      applyProjectMutationNavigation(initialState, projects, "restored-1", "restore", true),
    ).toEqual({
      selectedProjectId: "restored-1",
      expandedProjectIds: ["active-1", "restored-1"],
      archivedSectionOpen: false,
    });
  });

  test("failed archive or restore keeps navigation state unchanged", () => {
    const projects = [activeProject("active-1", "CRJU201")];

    expect(
      applyProjectMutationNavigation(initialState, projects, "active-1", "archive", false),
    ).toEqual(initialState);
  });
});
