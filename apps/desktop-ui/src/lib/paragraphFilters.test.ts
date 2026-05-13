import { describe, expect, test } from "vitest";
import type { ReviewCitation, ReviewParagraph } from "../types";
import { filterParagraphs } from "./paragraphFilters";

function citation(overrides: Partial<ReviewCitation> = {}): ReviewCitation {
  return {
    id: "citation-1",
    text: "Smith (2020)",
    start: null,
    end: null,
    verified: false,
    reference_entry_id: null,
    reference_suggestions: [],
    ...overrides,
  };
}

function paragraph(overrides: Partial<ReviewParagraph>): ReviewParagraph {
  return {
    id: "paragraph-1",
    order_index: 0,
    page: null,
    text: "A claim that needs support.",
    formatted_text: null,
    is_table: false,
    needs_citation: false,
    citations: [],
    ...overrides,
  };
}

function ids(paragraphs: ReviewParagraph[]): string[] {
  return paragraphs.map((item) => item.id);
}

describe("paragraph filters", () => {
  const paragraphs = [
    paragraph({
      id: "missing-citation",
      needs_citation: true,
    }),
    paragraph({
      id: "unlinked-citation",
      citations: [citation({ id: "unlinked", reference_entry_id: null })],
    }),
    paragraph({
      id: "suggested-citation",
      citations: [
        citation({
          id: "suggested",
          reference_entry_id: null,
          reference_suggestions: [
            {
              reference_entry_id: "reference-1",
              label: "Smith, J. (2020). Worked examples in practice. Learning Press.",
              confidence: "strong",
              reason: "Author and year match",
            },
          ],
        }),
      ],
    }),
    paragraph({
      id: "linked-citation",
      citations: [citation({ id: "linked", reference_entry_id: "reference-1" })],
    }),
  ];

  test("keeps the existing paragraph status filters", () => {
    expect(ids(filterParagraphs(paragraphs, "all"))).toEqual([
      "missing-citation",
      "unlinked-citation",
      "suggested-citation",
      "linked-citation",
    ]);
    expect(ids(filterParagraphs(paragraphs, "citation-total"))).toEqual([
      "unlinked-citation",
      "suggested-citation",
      "linked-citation",
    ]);
    expect(ids(filterParagraphs(paragraphs, "has-citation"))).toEqual([
      "unlinked-citation",
      "suggested-citation",
      "linked-citation",
    ]);
    expect(ids(filterParagraphs(paragraphs, "needs-citation"))).toEqual(["missing-citation"]);
  });

  test("filters the citation review queue by linked, suggested, and unlinked states", () => {
    expect(ids(filterParagraphs(paragraphs, "linked-citation"))).toEqual(["linked-citation"]);
    expect(ids(filterParagraphs(paragraphs, "suggested-citation"))).toEqual([
      "suggested-citation",
    ]);
    expect(ids(filterParagraphs(paragraphs, "unlinked-citation"))).toEqual([
      "unlinked-citation",
      "suggested-citation",
    ]);
  });
});
