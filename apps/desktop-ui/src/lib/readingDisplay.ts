import type { ModuleReadingSummary } from "../types";

export function readingListMetadata(reading: ModuleReadingSummary): string[] {
  const metadata = [reading.validation_status.replace("_", " ")];
  const optionalValues = [
    reading.estimated_reading_time,
    reading.reading_notes,
    reading.notes,
    reading.url,
  ];

  for (const value of optionalValues) {
    const trimmed = value?.trim();
    if (trimmed) {
      metadata.push(trimmed);
    }
  }

  return metadata;
}
