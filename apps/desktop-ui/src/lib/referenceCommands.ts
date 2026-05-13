import { invoke } from "@tauri-apps/api/core";
import type { CourseReferenceSummary } from "../types";

export type AddCourseReferenceInput = {
  apa_citation: string;
  notes?: string | null;
};

export function listCourseReferences(): Promise<CourseReferenceSummary[]> {
  return invoke<CourseReferenceSummary[]>("list_course_references");
}

export function addCourseReference(
  input: AddCourseReferenceInput,
): Promise<CourseReferenceSummary> {
  return invoke<CourseReferenceSummary>("add_course_reference", {
    request: {
      apa_citation: input.apa_citation.trim(),
      notes: input.notes?.trim() || null,
    },
  });
}
