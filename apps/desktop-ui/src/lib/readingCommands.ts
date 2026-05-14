import { invoke } from "@tauri-apps/api/core";
import type { CourseModuleSummary, ModuleReadingSummary } from "../types";

export type AddRadciteModuleInput = {
  title: string;
  code?: string | null;
  order_index?: number | null;
  description?: string | null;
};

export type AddModuleReadingInput = {
  module_id: string;
  reading_category: string;
  lesson_code?: string | null;
  apa_citation?: string | null;
  citation_text?: string | null;
  url?: string | null;
  notes?: string | null;
  reading_notes?: string | null;
  estimated_reading_time?: string | null;
};

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function listRadciteModules(): Promise<CourseModuleSummary[]> {
  return invoke<CourseModuleSummary[]>("list_radcite_modules");
}

export function addRadciteModule(
  input: AddRadciteModuleInput,
): Promise<CourseModuleSummary> {
  return invoke<CourseModuleSummary>("add_radcite_module", {
    request: {
      title: input.title.trim(),
      code: trimmedOrNull(input.code),
      order_index: input.order_index ?? null,
      description: trimmedOrNull(input.description),
    },
  });
}

export function listModuleReadings(moduleId: string): Promise<ModuleReadingSummary[]> {
  return invoke<ModuleReadingSummary[]>("list_module_readings", {
    request: {
      module_id: moduleId,
    },
  });
}

export function addModuleReading(
  input: AddModuleReadingInput,
): Promise<ModuleReadingSummary> {
  return invoke<ModuleReadingSummary>("add_module_reading", {
    request: {
      module_id: input.module_id,
      reading_category: input.reading_category.trim(),
      lesson_code: trimmedOrNull(input.lesson_code),
      apa_citation: trimmedOrNull(input.apa_citation),
      citation_text: trimmedOrNull(input.citation_text),
      url: trimmedOrNull(input.url),
      notes: trimmedOrNull(input.notes),
      reading_notes: trimmedOrNull(input.reading_notes),
      estimated_reading_time: trimmedOrNull(input.estimated_reading_time),
    },
  });
}
