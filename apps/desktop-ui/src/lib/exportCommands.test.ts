import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseReferencesExport } from "../types";
import { exportCourseReferences } from "./exportCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const exportResult: CourseReferencesExport = {
  filename: "crju150-course-references.html",
  content_type: "text/html; charset=utf-8",
  html: "<p>Smith, J. (2020). Worked examples in practice. Learning Press.</p>",
  reference_count: 1,
};

describe("export commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("exports course references with the AKO Learn flag", async () => {
    vi.mocked(invoke).mockResolvedValue(exportResult);

    await expect(exportCourseReferences({ for_ako_learn: true })).resolves.toBe(exportResult);

    expect(invoke).toHaveBeenCalledWith("export_course_references", {
      request: {
        for_ako_learn: true,
      },
    });
  });
});
