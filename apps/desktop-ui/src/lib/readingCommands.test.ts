import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseModuleSummary, ModuleReadingSummary } from "../types";
import {
  addModuleReading,
  addRadciteModule,
  listModuleReadings,
  listRadciteModules,
} from "./readingCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const moduleSummary: CourseModuleSummary = {
  id: "module-1",
  project_id: "project-1",
  code: "M1",
  title: "Module 1",
  order_index: 1,
  description: "Foundations",
};

const readingSummary: ModuleReadingSummary = {
  id: "reading-1",
  project_id: "project-1",
  module_id: "module-1",
  reading_category: "optional",
  lesson_code: "1.2",
  apa_citation: "Smith, J. (2024). Optional reading.",
  citation_text: null,
  title: null,
  url: "https://example.com/reading",
  notes: "Manual entry",
  reading_notes: "Skim before class",
  estimated_reading_time: "15 minutes",
  validation_status: "unknown",
};

describe("reading commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists RADcite modules from the Local DB", async () => {
    vi.mocked(invoke).mockResolvedValue([moduleSummary]);

    await expect(listRadciteModules()).resolves.toEqual([moduleSummary]);

    expect(invoke).toHaveBeenCalledWith("list_radcite_modules");
  });

  test("adds a trimmed RADcite module", async () => {
    vi.mocked(invoke).mockResolvedValue(moduleSummary);

    await expect(
      addRadciteModule({
        title: " Module 1 ",
        code: " M1 ",
        order_index: 1,
        description: " Foundations ",
      }),
    ).resolves.toBe(moduleSummary);

    expect(invoke).toHaveBeenCalledWith("add_radcite_module", {
      request: {
        title: "Module 1",
        code: "M1",
        order_index: 1,
        description: "Foundations",
      },
    });
  });

  test("lists readings for a module", async () => {
    vi.mocked(invoke).mockResolvedValue([readingSummary]);

    await expect(listModuleReadings("module-1")).resolves.toEqual([readingSummary]);

    expect(invoke).toHaveBeenCalledWith("list_module_readings", {
      request: {
        module_id: "module-1",
      },
    });
  });

  test("adds a trimmed module reading", async () => {
    vi.mocked(invoke).mockResolvedValue(readingSummary);

    await expect(
      addModuleReading({
        module_id: "module-1",
        reading_category: " optional ",
        lesson_code: " 1.2 ",
        apa_citation: " Smith, J. (2024). Optional reading. ",
        citation_text: "",
        url: " https://example.com/reading ",
        notes: " Manual entry ",
        reading_notes: " Skim before class ",
        estimated_reading_time: " 15 minutes ",
      }),
    ).resolves.toBe(readingSummary);

    expect(invoke).toHaveBeenCalledWith("add_module_reading", {
      request: {
        module_id: "module-1",
        reading_category: "optional",
        lesson_code: "1.2",
        apa_citation: "Smith, J. (2024). Optional reading.",
        citation_text: null,
        url: "https://example.com/reading",
        notes: "Manual entry",
        reading_notes: "Skim before class",
        estimated_reading_time: "15 minutes",
      },
    });
  });
});
