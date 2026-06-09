import { describe, expect, test } from "vitest";
import type { CourseModuleSummary, ModuleReadingImportCandidate } from "../types";
import {
  applyAutoModuleToCandidates,
  inferModuleDraftForImport,
  moduleMatchesImportDraft,
} from "./readingImportWorkflow";

const candidate: ModuleReadingImportCandidate = {
  module_order: null,
  module_title: null,
  reading_category: "compulsory",
  lesson_code: null,
  apa_citation: "Smith, J. (2024). Worked examples.",
  citation_text: null,
  url: null,
};

const module: CourseModuleSummary = {
  id: "module-6",
  project_id: "project-1",
  code: null,
  title: "Module 6",
  order_index: 6,
  description: null,
};

describe("reading import workflow", () => {
  test("infers a module draft from candidate module metadata", () => {
    expect(
      inferModuleDraftForImport(
        [
          {
            ...candidate,
            module_order: 2,
            module_title: "Module 2 - Campaign planning",
          },
        ],
        "/Users/name/Desktop/COMS432 Module 6.docx",
      ),
    ).toEqual({
      title: "Module 2 - Campaign planning",
      code: null,
      order_index: 2,
      description: null,
    });
  });

  test("infers a module draft from the DOCX filename when candidates lack module metadata", () => {
    expect(inferModuleDraftForImport([candidate], "/Users/name/Desktop/COMS432 Module 6.docx"))
      .toEqual({
        title: "Module 6",
        code: null,
        order_index: 6,
        description: null,
      });
  });

  test("matches existing modules by order or title", () => {
    const draft = {
      title: "Module 6",
      code: null,
      order_index: 6,
      description: null,
    };

    expect(moduleMatchesImportDraft(module, draft)).toBe(true);
    expect(moduleMatchesImportDraft({ ...module, order_index: null }, draft)).toBe(true);
    expect(moduleMatchesImportDraft({ ...module, title: "Module 7", order_index: 7 }, draft)).toBe(
      false,
    );
  });

  test("applies an auto-created module to unassigned selected candidates", () => {
    expect(
      applyAutoModuleToCandidates(
        [
          {
            ...candidate,
            module_id: "",
            selected: true,
          },
          {
            ...candidate,
            module_id: "existing-module",
            selected: true,
          },
          {
            ...candidate,
            module_id: "",
            selected: false,
          },
        ],
        "module-6",
      ).map((item) => item.module_id),
    ).toEqual(["module-6", "existing-module", ""]);
  });
});
