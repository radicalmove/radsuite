import { describe, expect, test } from "vitest";
import type { ModuleReadingSummary } from "../types";
import type { CrossrefSourceResult } from "./sourceSearch";
import {
  appendCrossrefLookupNote,
  incompleteModuleReadings,
  moduleReadingLookupQuery,
  moduleReadingUpdateFromCrossref,
} from "./readingLookup";

function reading(overrides: Partial<ModuleReadingSummary>): ModuleReadingSummary {
  return {
    id: "reading-1",
    project_id: "project-1",
    module_id: "module-1",
    reading_category: "compulsory",
    lesson_code: "1.1",
    apa_citation: "Smith, J. (2024). Existing title.",
    citation_text: "Existing title",
    title: "Existing title",
    doi: null,
    url: "https://example.test/existing",
    notes: "Imported from course outline.",
    reading_notes: "Read pages 1-10.",
    estimated_reading_time: "20 minutes",
    validation_status: "valid",
    ...overrides,
  };
}

function result(overrides: Partial<CrossrefSourceResult> = {}): CrossrefSourceResult {
  return {
    title: "A better source",
    authors: "Jones, P.",
    year: "2025",
    source: "Teaching Journal",
    doi: "10.1000/example",
    url: "https://doi.org/10.1000/example",
    apaCitation: "Jones, P. (2025). A better source. Teaching Journal. https://doi.org/10.1000/example",
    ...overrides,
  };
}

describe("module reading lookup helpers", () => {
  test("finds readings missing either APA data or a source URL", () => {
    expect(
      incompleteModuleReadings([
        reading({ id: "complete" }),
        reading({ id: "missing-apa", apa_citation: null }),
        reading({ id: "missing-url", url: null }),
      ]).map((item) => item.id),
    ).toEqual(["missing-apa", "missing-url"]);
  });

  test("chooses the most informative saved reading text as the Crossref query", () => {
    expect(moduleReadingLookupQuery(reading({ apa_citation: "  " }))).toBe("Existing title");
    expect(
      moduleReadingLookupQuery(
        reading({ apa_citation: null, citation_text: "Citation text", title: "Title" }),
      ),
    ).toBe("Citation text");
    expect(moduleReadingLookupQuery(reading({ apa_citation: null, citation_text: null, title: null }))).toBeNull();
  });

  test("builds a complete update while preserving unrelated reading fields", () => {
    expect(moduleReadingUpdateFromCrossref(reading({ apa_citation: null, url: null }), result())).toEqual({
      reading_id: "reading-1",
      reading_category: "compulsory",
      lesson_code: "1.1",
      apa_citation: result().apaCitation,
      citation_text: result().apaCitation,
      doi: result().doi,
      url: result().url,
      notes: "Imported from course outline. Imported from Crossref search. DOI: 10.1000/example",
      reading_notes: "Read pages 1-10.",
      estimated_reading_time: "20 minutes",
    });
  });

  test("does not duplicate a Crossref provenance note", () => {
    const note = "Imported from course outline. Imported from Crossref search. DOI: 10.1000/example";
    expect(appendCrossrefLookupNote(note, "10.1000/example")).toBe(note);
  });
});
