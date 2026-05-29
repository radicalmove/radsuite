import { invoke } from "@tauri-apps/api/core";
import type { CourseReferenceSummary } from "../types";

export type AddCourseReferenceInput = {
  project_id?: string | null;
  apa_citation: string;
  notes?: string | null;
};

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function listCourseReferences(
  projectId?: string | null,
): Promise<CourseReferenceSummary[]> {
  return invoke<CourseReferenceSummary[]>("list_course_references", {
    request: {
      project_id: trimmedOrNull(projectId),
    },
  });
}

export function addCourseReference(
  input: AddCourseReferenceInput,
): Promise<CourseReferenceSummary> {
  return invoke<CourseReferenceSummary>("add_course_reference", {
    request: {
      project_id: trimmedOrNull(input.project_id),
      apa_citation: input.apa_citation.trim(),
      notes: trimmedOrNull(input.notes),
    },
  });
}
