import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { RadciteProjectSummary } from "../types";
import {
  archiveRadciteProject,
  createRadciteProject,
  importLegacyRadciteDatabase,
  listRadciteProjects,
  restoreRadciteProject,
  updateRadciteProject,
} from "./projectCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const project: RadciteProjectSummary = {
  id: "project-1",
  code: "CRJU201",
  title: "Criminological Theory",
  description: "A course about criminological theory",
  structure_mode: "modules",
  archived_at: null,
};

describe("project commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists RADcite projects from the Local DB", async () => {
    vi.mocked(invoke).mockResolvedValue([project]);

    await expect(listRadciteProjects()).resolves.toEqual([project]);

    expect(invoke).toHaveBeenCalledWith("list_radcite_projects");
  });

  test("imports a trimmed legacy RADcite database path", async () => {
    const result = {
      source_path: "/tmp/citation_checker.db",
      projects_imported: 1,
      modules_imported: 2,
      documents_imported: 3,
      paragraphs_imported: 4,
      references_imported: 5,
      readings_imported: 2,
      citations_imported: 6,
      warnings: [],
    };
    vi.mocked(invoke).mockResolvedValue(result);

    await expect(importLegacyRadciteDatabase(" /tmp/citation_checker.db ")).resolves.toBe(result);

    expect(invoke).toHaveBeenCalledWith("import_legacy_radcite_database", {
      request: { path: "/tmp/citation_checker.db" },
    });
  });

  test("creates a trimmed RADcite project", async () => {
    vi.mocked(invoke).mockResolvedValue(project);

    await expect(
      createRadciteProject({
        code: " CRJU201 ",
        title: " Criminological Theory ",
      }),
    ).resolves.toBe(project);

    expect(invoke).toHaveBeenCalledWith("create_radcite_project", {
      request: {
        code: "CRJU201",
        title: "Criminological Theory",
      },
    });
  });

  test("archives a trimmed project by ID", async () => {
    vi.mocked(invoke).mockResolvedValue(project);

    await expect(archiveRadciteProject(" project-1 ")).resolves.toBe(project);

    expect(invoke).toHaveBeenCalledWith("archive_radcite_project", {
      request: { project_id: "project-1" },
    });
  });

  test("restores a trimmed project by ID", async () => {
    vi.mocked(invoke).mockResolvedValue(project);

    await expect(restoreRadciteProject(" project-1 ")).resolves.toBe(project);

    expect(invoke).toHaveBeenCalledWith("restore_radcite_project", {
      request: { project_id: "project-1" },
    });
  });

  test("updates a trimmed project by ID", async () => {
    vi.mocked(invoke).mockResolvedValue(project);

    await expect(
      updateRadciteProject(" project-1 ", {
        code: " COMS432 ",
        title: " Strategic Communication ",
        description: " Course foundations and applied practice ",
        structureMode: "weeks",
      }),
    ).resolves.toBe(project);

    expect(invoke).toHaveBeenCalledWith("update_radcite_project", {
      request: {
        project_id: "project-1",
        code: "COMS432",
        title: "Strategic Communication",
        description: "Course foundations and applied practice",
        structure_mode: "weeks",
      },
    });
  });
});
