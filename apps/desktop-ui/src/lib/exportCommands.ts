import { invoke } from "@tauri-apps/api/core";
import type {
  CourseReferencesExport,
  CourseReferencesExportRequest,
  ModuleReadingsExport,
  ModuleReadingsExportRequest,
} from "../types";

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function exportCourseReferences(
  request: CourseReferencesExportRequest,
): Promise<CourseReferencesExport> {
  return invoke<CourseReferencesExport>("export_course_references", {
    request: {
      project_id: trimmedOrNull(request.project_id),
      for_ako_learn: request.for_ako_learn,
    },
  });
}

export function exportModuleReadings(
  request: ModuleReadingsExportRequest,
): Promise<ModuleReadingsExport> {
  return invoke<ModuleReadingsExport>("export_module_readings", { request });
}
