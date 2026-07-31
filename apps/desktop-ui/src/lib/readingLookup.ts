import type { ModuleReadingSummary } from "../types";
import type { UpdateModuleReadingInput } from "./readingCommands";
import type { CrossrefSourceResult } from "./sourceSearch";

export function incompleteModuleReadings(
  readings: ModuleReadingSummary[],
): ModuleReadingSummary[] {
  return readings.filter(
    (reading) => !reading.apa_citation?.trim() || !reading.url?.trim(),
  );
}

export function moduleReadingLookupQuery(reading: ModuleReadingSummary): string | null {
  return [reading.apa_citation, reading.citation_text, reading.title]
    .map((value) => value?.trim() ?? "")
    .find(Boolean) ?? null;
}

export function appendCrossrefLookupNote(existing: string | null, doi: string | null): string {
  const current = existing?.trim() ?? "";
  if (current.includes("Imported from Crossref search.")) {
    return current;
  }

  const provenance = doi
    ? `Imported from Crossref search. DOI: ${doi}`
    : "Imported from Crossref search.";
  return [current, provenance].filter(Boolean).join(" ");
}

export function moduleReadingUpdateFromCrossref(
  reading: ModuleReadingSummary,
  result: CrossrefSourceResult,
): UpdateModuleReadingInput {
  return {
    reading_id: reading.id,
    reading_category: reading.reading_category,
    lesson_code: reading.lesson_code,
    apa_citation: result.apaCitation,
    citation_text: result.apaCitation,
    doi: result.doi ?? reading.doi,
    url: result.url ?? reading.url,
    notes: appendCrossrefLookupNote(reading.notes, result.doi),
    reading_notes: reading.reading_notes,
    estimated_reading_time: reading.estimated_reading_time,
  };
}
