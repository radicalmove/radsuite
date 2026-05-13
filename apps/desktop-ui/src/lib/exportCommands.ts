import { invoke } from "@tauri-apps/api/core";
import type { CourseReferencesExport, CourseReferencesExportRequest } from "../types";

export function exportCourseReferences(
  request: CourseReferencesExportRequest,
): Promise<CourseReferencesExport> {
  return invoke<CourseReferencesExport>("export_course_references", { request });
}
