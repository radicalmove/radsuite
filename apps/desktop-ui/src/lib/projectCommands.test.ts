import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { RadciteProjectSummary } from "../types";
import { createRadciteProject, listRadciteProjects } from "./projectCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const project: RadciteProjectSummary = {
  id: "project-1",
  code: "CRJU201",
  title: "Criminological Theory",
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
});
