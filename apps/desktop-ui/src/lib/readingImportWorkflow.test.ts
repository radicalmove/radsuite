import { describe, expect, test } from "vitest";
import type { CourseModuleSummary, ModuleReadingImportCandidate } from "../types";
import {
  applyAutoModuleToCandidates,
  defaultModuleIdForImportCandidate,
  defaultReadingImportNotes,
  inferModuleDraftForImport,
  moduleMatchesImportDraft,
  selectedImportHasUsableModuleAssignments,
} from "./readingImportWorkflow";

const candidate: ModuleReadingImportCandidate = {
  source_path: null,
  source_filename: null,
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

  test("uses an existing module matching the import draft before the selected module", () => {
    const selectedModule = {
      ...module,
      id: "module-1",
      title: "Module 1",
      order_index: 1,
    };
    const importDraft = inferModuleDraftForImport(
      [candidate],
      "/Users/name/Desktop/COMS432 Module 6.docx",
    );

    expect(
      defaultModuleIdForImportCandidate(
        candidate,
        [selectedModule, module],
        selectedModule.id,
        importDraft,
      ),
    ).toBe("module-6");
  });

  test("leaves candidates unassigned when the import draft needs a new module", () => {
    const selectedModule = {
      ...module,
      id: "module-1",
      title: "Module 1",
      order_index: 1,
    };
    const importDraft = inferModuleDraftForImport(
      [candidate],
      "/Users/name/Desktop/COMS432 Module 6.docx",
    );

    expect(
      defaultModuleIdForImportCandidate(
        candidate,
        [selectedModule],
        selectedModule.id,
        importDraft,
      ),
    ).toBe("");
  });

  test("falls back to the selected module when the import has no module signal", () => {
    expect(
      defaultModuleIdForImportCandidate(candidate, [module], module.id, null),
    ).toBe("module-6");
  });

  test("allows saving selected candidates assigned to multiple existing modules", () => {
    expect(
      selectedImportHasUsableModuleAssignments([
        { ...candidate, selected: true, module_id: "module-1" },
        { ...candidate, selected: true, module_id: "module-2" },
      ]),
    ).toBe(true);
  });

  test("does not allow saving selected candidates with missing module assignments", () => {
    expect(
      selectedImportHasUsableModuleAssignments([
        { ...candidate, selected: true, module_id: "module-1" },
        { ...candidate, selected: true, module_id: "" },
      ]),
    ).toBe(false);
  });

  test("uses source filenames as default import notes", () => {
    expect(
      defaultReadingImportNotes({
        ...candidate,
        source_filename: "Module 6 Microlearning 3.pdf",
      }),
    ).toBe("Imported from Module 6 Microlearning 3.pdf");
  });

  test("leaves default import notes blank without source filename", () => {
    expect(defaultReadingImportNotes(candidate)).toBe("");
  });
});
