import { describe, expect, test } from "vitest";

import type { ModuleReadingSummary } from "../types";
import { readingListMetadata } from "./readingDisplay";

const baseReading: ModuleReadingSummary = {
  id: "reading-1",
  project_id: "project-1",
  module_id: "module-1",
  reading_category: "compulsory",
  lesson_code: null,
  apa_citation: "Smith, J. (2024). Learning design.",
  citation_text: null,
  title: null,
  url: null,
  notes: null,
  reading_notes: null,
  estimated_reading_time: null,
  validation_status: "needs_fix",
};

describe("reading display metadata", () => {
  test("shows saved reading context in list metadata", () => {
    expect(
      readingListMetadata({
        ...baseReading,
        url: "https://doi.org/10.1000/example",
        notes: "Imported from Module 6 Microlearning 3.pdf",
        reading_notes: "Read before class",
        estimated_reading_time: "20 minutes",
      }),
    ).toEqual([
      "needs fix",
      "20 minutes",
      "Read before class",
      "Imported from Module 6 Microlearning 3.pdf",
      "https://doi.org/10.1000/example",
    ]);
  });

  test("ignores blank optional metadata", () => {
    expect(
      readingListMetadata({
        ...baseReading,
        url: " ",
        notes: "",
        reading_notes: null,
        estimated_reading_time: "  ",
        validation_status: "valid",
      }),
    ).toEqual(["valid"]);
  });
});
