import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseModuleSummary, ModuleReadingSummary } from "../types";
import {
  addModuleReading,
  addRadciteModule,
  archiveModuleReading,
  archiveRadciteModule,
  listModuleReadings,
  listRadciteModules,
  previewModuleReadingsCsvImport,
  previewModuleReadingsImport,
  saveModuleReadingsImport,
  updateModuleReading,
  updateRadciteModule,
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

    await expect(listRadciteModules("project-1")).resolves.toEqual([moduleSummary]);

    expect(invoke).toHaveBeenCalledWith("list_radcite_modules", {
      request: {
        project_id: "project-1",
      },
    });
  });

  test("adds a trimmed RADcite module", async () => {
    vi.mocked(invoke).mockResolvedValue(moduleSummary);

    await expect(
      addRadciteModule({
        project_id: " project-1 ",
        title: " Module 1 ",
        code: " M1 ",
        order_index: 1,
        description: " Foundations ",
      }),
    ).resolves.toBe(moduleSummary);

    expect(invoke).toHaveBeenCalledWith("add_radcite_module", {
      request: {
        project_id: "project-1",
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

  test("updates a trimmed RADcite module", async () => {
    vi.mocked(invoke).mockResolvedValue(moduleSummary);

    await expect(
      updateRadciteModule({
        module_id: "module-1",
        title: " Module 1 ",
        code: " M1 ",
        order_index: 2,
        description: " Foundations ",
      }),
    ).resolves.toBe(moduleSummary);

    expect(invoke).toHaveBeenCalledWith("update_radcite_module", {
      request: {
        module_id: "module-1",
        title: "Module 1",
        code: "M1",
        order_index: 2,
        description: "Foundations",
      },
    });
  });

  test("archives a RADcite module", async () => {
    vi.mocked(invoke).mockResolvedValue(moduleSummary);

    await expect(archiveRadciteModule("module-1")).resolves.toBe(moduleSummary);

    expect(invoke).toHaveBeenCalledWith("archive_radcite_module", {
      request: {
        module_id: "module-1",
      },
    });
  });

  test("updates a trimmed module reading", async () => {
    vi.mocked(invoke).mockResolvedValue(readingSummary);

    await expect(
      updateModuleReading({
        reading_id: "reading-1",
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

    expect(invoke).toHaveBeenCalledWith("update_module_reading", {
      request: {
        reading_id: "reading-1",
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

  test("archives a module reading", async () => {
    vi.mocked(invoke).mockResolvedValue(readingSummary);

    await expect(archiveModuleReading("reading-1")).resolves.toBe(readingSummary);

    expect(invoke).toHaveBeenCalledWith("archive_module_reading", {
      request: {
        reading_id: "reading-1",
      },
    });
  });

  test("previews a trimmed module readings import", async () => {
    const candidates = [
      {
        module_order: 1,
        module_title: "Module 1",
        reading_category: "compulsory",
        lesson_code: "1.2",
        apa_citation: "Smith, J. (2024). Worked examples.",
        citation_text: "1.2 Smith, J. (2024). Worked examples.",
        url: "https://example.com/worked",
      },
    ];
    vi.mocked(invoke).mockResolvedValue(candidates);

    await expect(
      previewModuleReadingsImport({
        path: " /tmp/readings.docx ",
        original_filename: " Readings.docx ",
      }),
    ).resolves.toBe(candidates);

    expect(invoke).toHaveBeenCalledWith("preview_module_readings_import", {
      request: {
        path: "/tmp/readings.docx",
        original_filename: "Readings.docx",
      },
    });
  });

  test("previews a trimmed module readings CSV import", async () => {
    const candidates = [
      {
        module_order: 2,
        module_title: "Week 2 - Positivism",
        reading_category: "compulsory",
        lesson_code: "02",
        apa_citation: '"Biosocial Theories of Crime" in Miller, M. (2015).',
        citation_text: null,
        url: null,
      },
    ];
    vi.mocked(invoke).mockResolvedValue(candidates);

    await expect(
      previewModuleReadingsCsvImport({
        path: " /tmp/course_readings.csv ",
        original_filename: " course_readings.csv ",
      }),
    ).resolves.toBe(candidates);

    expect(invoke).toHaveBeenCalledWith("preview_module_readings_csv_import", {
      request: {
        path: "/tmp/course_readings.csv",
        original_filename: "course_readings.csv",
      },
    });
  });

  test("saves trimmed selected module readings import candidates", async () => {
    vi.mocked(invoke).mockResolvedValue([readingSummary]);

    await expect(
      saveModuleReadingsImport({
        candidates: [
          {
            module_id: "module-1",
            reading_category: " optional ",
            lesson_code: " 1.2 ",
            apa_citation: " Smith, J. (2024). Worked examples. ",
            citation_text: "",
            url: " https://example.com/worked ",
            notes: " Imported from DOCX ",
            reading_notes: " Read before class ",
            estimated_reading_time: " 20 minutes ",
          },
        ],
      }),
    ).resolves.toEqual([readingSummary]);

    expect(invoke).toHaveBeenCalledWith("save_module_readings_import", {
      request: {
        candidates: [
          {
            module_id: "module-1",
            reading_category: "optional",
            lesson_code: "1.2",
            apa_citation: "Smith, J. (2024). Worked examples.",
            citation_text: null,
            url: "https://example.com/worked",
            notes: "Imported from DOCX",
            reading_notes: "Read before class",
            estimated_reading_time: "20 minutes",
          },
        ],
      },
    });
  });
});
