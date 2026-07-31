import { describe, expect, test } from "vitest";

import type { CourseReferenceSummary } from "../types";
import {
  filterCourseReferencesForDisplay,
  readHideApaReady,
  writeHideApaReady,
} from "./referenceDisplay";

const references: CourseReferenceSummary[] = [
  {
    id: "reference-valid",
    project_id: "project-1",
    reference_type: "reference",
    citation_text: "Smith (2024)",
    apa_citation: "Smith, J. (2024). A valid source.",
    authors: ["Smith, J."],
    publication_year: "2024",
    source: null,
    title: null,
    doi: null,
    url: null,
    notes: null,
    validation_status: "valid",
    validation_report: null,
  },
  {
    id: "reference-warning",
    project_id: "project-1",
    reference_type: "reference",
    citation_text: "Jones (2023)",
    apa_citation: "Jones, P. (2023). A source needing review.",
    authors: ["Jones, P."],
    publication_year: "2023",
    source: null,
    title: null,
    doi: null,
    url: null,
    notes: null,
    validation_status: "needs_fix",
    validation_report: "Check the publisher details.",
  },
  {
    id: "reference-unknown",
    project_id: "project-1",
    reference_type: "reference",
    citation_text: "Brown (2022)",
    apa_citation: "Brown, A. (2022). An unchecked source.",
    authors: ["Brown, A."],
    publication_year: "2022",
    source: null,
    title: null,
    doi: null,
    url: null,
    notes: null,
    validation_status: "unknown",
    validation_report: null,
  },
];

function memoryStorage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial));
  return {
    getItem(key: string) {
      return values.get(key) ?? null;
    },
    setItem(key: string, value: string) {
      values.set(key, value);
    },
  };
}

describe("course reference display preferences", () => {
  test("hides only APA-valid references when the filter is enabled", () => {
    expect(filterCourseReferencesForDisplay(references, true).map((reference) => reference.id)).toEqual([
      "reference-warning",
      "reference-unknown",
    ]);
  });

  test("shows every reference when the filter is disabled", () => {
    expect(filterCourseReferencesForDisplay(references, false)).toEqual(references);
  });

  test("persists the hide APA-ready preference", () => {
    const storage = memoryStorage();

    expect(readHideApaReady(storage)).toBe(false);
    writeHideApaReady(storage, true);
    expect(readHideApaReady(storage)).toBe(true);
    writeHideApaReady(storage, false);
    expect(readHideApaReady(storage)).toBe(false);
  });

  test("falls back safely when the stored preference is malformed", () => {
    const storage = memoryStorage({ radciteReferenceDisplayPreferences: "{not-json" });
    expect(readHideApaReady(storage)).toBe(false);
  });
});
