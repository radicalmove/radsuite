import { invoke } from "@tauri-apps/api/core";
import type {
  CourseModuleSummary,
  ModuleReadingImportCandidate,
  ModuleReadingSummary,
} from "../types";

export type AddRadciteModuleInput = {
  title: string;
  code?: string | null;
  order_index?: number | null;
  description?: string | null;
};

export type UpdateRadciteModuleInput = AddRadciteModuleInput & {
  module_id: string;
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

export type UpdateModuleReadingInput = Omit<AddModuleReadingInput, "module_id"> & {
  reading_id: string;
};

export type PreviewModuleReadingsImportInput = {
  path: string;
  original_filename?: string | null;
};

export type SaveModuleReadingsImportInput = {
  candidates: AddModuleReadingInput[];
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

export function updateRadciteModule(
  input: UpdateRadciteModuleInput,
): Promise<CourseModuleSummary> {
  return invoke<CourseModuleSummary>("update_radcite_module", {
    request: {
      module_id: input.module_id,
      title: input.title.trim(),
      code: trimmedOrNull(input.code),
      order_index: input.order_index ?? null,
      description: trimmedOrNull(input.description),
    },
  });
}

export function archiveRadciteModule(moduleId: string): Promise<CourseModuleSummary> {
  return invoke<CourseModuleSummary>("archive_radcite_module", {
    request: {
      module_id: moduleId,
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

export function updateModuleReading(
  input: UpdateModuleReadingInput,
): Promise<ModuleReadingSummary> {
  return invoke<ModuleReadingSummary>("update_module_reading", {
    request: {
      reading_id: input.reading_id,
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

export function archiveModuleReading(readingId: string): Promise<ModuleReadingSummary> {
  return invoke<ModuleReadingSummary>("archive_module_reading", {
    request: {
      reading_id: readingId,
    },
  });
}

export function previewModuleReadingsImport(
  input: PreviewModuleReadingsImportInput,
): Promise<ModuleReadingImportCandidate[]> {
  return invoke<ModuleReadingImportCandidate[]>("preview_module_readings_import", {
    request: {
      path: input.path.trim(),
      original_filename: trimmedOrNull(input.original_filename),
    },
  });
}

export function saveModuleReadingsImport(
  input: SaveModuleReadingsImportInput,
): Promise<ModuleReadingSummary[]> {
  return invoke<ModuleReadingSummary[]>("save_module_readings_import", {
    request: {
      candidates: input.candidates.map((candidate) => ({
        module_id: candidate.module_id,
        reading_category: candidate.reading_category.trim(),
        lesson_code: trimmedOrNull(candidate.lesson_code),
        apa_citation: trimmedOrNull(candidate.apa_citation),
        citation_text: trimmedOrNull(candidate.citation_text),
        url: trimmedOrNull(candidate.url),
        notes: trimmedOrNull(candidate.notes),
        reading_notes: trimmedOrNull(candidate.reading_notes),
        estimated_reading_time: trimmedOrNull(candidate.estimated_reading_time),
      })),
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
