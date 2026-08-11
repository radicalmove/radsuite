import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseReferenceSummary } from "../types";
import {
  addCourseReference,
  archiveCourseReference,
  listCourseReferences,
  mergeCourseReferences,
  updateCourseReference,
} from "./referenceCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const reference: CourseReferenceSummary = {
  id: "reference-1",
  project_id: "project-1",
  module_id: null,
  reference_type: "reference",
  apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press.",
  citation_text: null,
  title: "Worked examples in practice",
  authors: ["Smith, J."],
  publication_year: "2020",
  source: "Learning Press",
  doi: null,
  url: null,
  notes: "Core course reference",
  validation_status: "unknown",
  validation_report: null,
};

describe("reference commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists course references from the Local DB", async () => {
    vi.mocked(invoke).mockResolvedValue([reference]);

    await expect(listCourseReferences("project-1")).resolves.toEqual([reference]);

    expect(invoke).toHaveBeenCalledWith("list_course_references", {
      request: {
        project_id: "project-1",
      },
    });
  });

  test("adds a trimmed course reference", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    await expect(
      addCourseReference({
        project_id: " project-1 ",
        apa_citation: "  Smith, J. (2020). Worked examples in practice. Learning Press.  ",
        notes: " Core course reference ",
      }),
    ).resolves.toBe(reference);

    expect(invoke).toHaveBeenCalledWith("add_course_reference", {
      request: {
        project_id: "project-1",
        apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press.",
        notes: "Core course reference",
      },
    });
  });

  test("assigns a course reference to a module", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    const { assignCourseReferenceModule } = await import("./referenceCommands");
    await assignCourseReferenceModule({
      reference_id: "reference-1",
      module_id: "module-1",
    });

    expect(invoke).toHaveBeenCalledWith("assign_course_reference_module", {
      request: {
        reference_id: "reference-1",
        module_id: "module-1",
      },
    });
  });

  test("updates a trimmed course reference", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    await expect(
      updateCourseReference({
        reference_id: "reference-1",
        apa_citation: "  Smith, J. (2020). Worked examples in practice. Learning Press.  ",
        notes: " Updated note ",
        citation_text: " Smith, J. (2020). Worked examples in practice. ",
        url: " https://doi.org/10.1234/example ",
      }),
    ).resolves.toBe(reference);

    expect(invoke).toHaveBeenCalledWith("update_course_reference", {
      request: {
        reference_id: "reference-1",
        apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press.",
        notes: "Updated note",
        citation_text: "Smith, J. (2020). Worked examples in practice.",
        url: "https://doi.org/10.1234/example",
      },
    });
  });

  test("can unassign a course reference from its module", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    const { assignCourseReferenceModule } = await import("./referenceCommands");
    await assignCourseReferenceModule({
      reference_id: "reference-1",
      module_id: null,
    });

    expect(invoke).toHaveBeenCalledWith("assign_course_reference_module", {
      request: {
        reference_id: "reference-1",
        module_id: null,
      },
    });
  });

  test("archives a course reference", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    await expect(archiveCourseReference("reference-1")).resolves.toBe(reference);

    expect(invoke).toHaveBeenCalledWith("archive_course_reference", {
      request: {
        reference_id: "reference-1",
      },
    });
  });

  test("merges selected course references into a primary reference", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    await expect(
      mergeCourseReferences({
        primary_reference_id: "reference-1",
        merge_reference_ids: ["reference-2", "reference-3"],
      }),
    ).resolves.toBe(reference);

    expect(invoke).toHaveBeenCalledWith("merge_course_references", {
      request: {
        primary_reference_id: "reference-1",
        merge_reference_ids: ["reference-2", "reference-3"],
      },
    });
  });
});
