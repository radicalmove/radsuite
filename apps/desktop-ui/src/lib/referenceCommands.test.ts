import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { CourseReferenceSummary } from "../types";
import { addCourseReference, listCourseReferences } from "./referenceCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const reference: CourseReferenceSummary = {
  id: "reference-1",
  project_id: "project-1",
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
};

describe("reference commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists course references from the Local DB", async () => {
    vi.mocked(invoke).mockResolvedValue([reference]);

    await expect(listCourseReferences()).resolves.toEqual([reference]);

    expect(invoke).toHaveBeenCalledWith("list_course_references");
  });

  test("adds a trimmed course reference", async () => {
    vi.mocked(invoke).mockResolvedValue(reference);

    await expect(
      addCourseReference({
        apa_citation: "  Smith, J. (2020). Worked examples in practice. Learning Press.  ",
        notes: " Core course reference ",
      }),
    ).resolves.toBe(reference);

    expect(invoke).toHaveBeenCalledWith("add_course_reference", {
      request: {
        apa_citation: "Smith, J. (2020). Worked examples in practice. Learning Press.",
        notes: "Core course reference",
      },
    });
  });
});
