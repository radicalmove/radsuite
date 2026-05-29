import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseReferencesExport, ModuleReadingsExport } from "../types";
import { exportCourseReferences, exportModuleReadings } from "./exportCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const exportResult: CourseReferencesExport = {
  filename: "crju150-course-references.html",
  content_type: "text/html; charset=utf-8",
  html: "<p>Smith, J. (2020). Worked examples in practice. Learning Press.</p>",
  reference_count: 1,
};

const moduleExportResult: ModuleReadingsExport = {
  filename: "crju150-module-1-module-readings.html",
  content_type: "text/html; charset=utf-8",
  html: "<h4>Compulsory readings</h4>",
  module_id: "module-1",
  reading_count: 2,
};

describe("export commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("exports course references with the AKO Learn flag", async () => {
    vi.mocked(invoke).mockResolvedValue(exportResult);

    await expect(
      exportCourseReferences({ project_id: "project-1", for_ako_learn: true }),
    ).resolves.toBe(exportResult);

    expect(invoke).toHaveBeenCalledWith("export_course_references", {
      request: {
        project_id: "project-1",
        for_ako_learn: true,
      },
    });
  });

  test("exports module readings with the module id and AKO Learn flag", async () => {
    vi.mocked(invoke).mockResolvedValue(moduleExportResult);

    await expect(
      exportModuleReadings({ module_id: "module-1", for_ako_learn: false }),
    ).resolves.toBe(moduleExportResult);

    expect(invoke).toHaveBeenCalledWith("export_module_readings", {
      request: {
        module_id: "module-1",
        for_ako_learn: false,
      },
    });
  });
});
