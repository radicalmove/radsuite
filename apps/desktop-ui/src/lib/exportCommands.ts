import { invoke } from "@tauri-apps/api/core";
import type {
  CourseReferencesExport,
  CourseReferencesExportRequest,
  ModuleReadingsExport,
  ModuleReadingsExportRequest,
} from "../types";

export function exportCourseReferences(
  request: CourseReferencesExportRequest,
): Promise<CourseReferencesExport> {
  return invoke<CourseReferencesExport>("export_course_references", { request });
}

export function exportModuleReadings(
  request: ModuleReadingsExportRequest,
): Promise<ModuleReadingsExport> {
  return invoke<ModuleReadingsExport>("export_module_readings", { request });
}
