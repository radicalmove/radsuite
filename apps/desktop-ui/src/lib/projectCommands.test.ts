import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { RadciteProjectSummary } from "../types";
import {
  archiveRadciteProject,
  createRadciteProject,
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
      }),
    ).resolves.toBe(project);

    expect(invoke).toHaveBeenCalledWith("update_radcite_project", {
      request: {
        project_id: "project-1",
        code: "COMS432",
        title: "Strategic Communication",
      },
    });
  });
});
