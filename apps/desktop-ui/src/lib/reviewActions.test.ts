import { describe, expect, test } from "vitest";
import type { AnalyseDocxReviewResponse, ReviewParagraph } from "../types";
import { addManualCitation, markParagraphResolved, verifyParagraphCitations } from "./reviewActions";

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

function analysis(paragraphs: ReviewParagraph[]): AnalyseDocxReviewResponse {
  return {
    project_id: "project-1",
    project_title: "RADcite Functional Testing",
    document_id: "document-1",
    original_filename: "source.docx",
    summary: {
      paragraph_count: paragraphs.length,
      citation_count: paragraphs.reduce((count, item) => count + item.citations.length, 0),
      cited_paragraph_count: paragraphs.filter((item) => item.citations.length > 0).length,
      missing_citation_count: paragraphs.filter((item) => item.needs_citation).length,
    },
    paragraphs,
  };
}

describe("review actions", () => {
  test("marking a paragraph as resolved clears missing-citation state and summary count", () => {
    const source = analysis([
      paragraph({ id: "paragraph-1", needs_citation: true }),
      paragraph({ id: "paragraph-2", needs_citation: true }),
    ]);

    const updated = markParagraphResolved(source, "paragraph-1");

    expect(updated.paragraphs[0].needs_citation).toBe(false);
    expect(updated.paragraphs[1].needs_citation).toBe(true);
    expect(updated.summary.missing_citation_count).toBe(1);
  });

  test("adding a manual citation appends a verified citation and resolves the paragraph", () => {
    const source = analysis([paragraph({ needs_citation: true })]);

    const updated = addManualCitation(source, "paragraph-1", "Smith (2024)");

    expect(updated.paragraphs[0].needs_citation).toBe(false);
    expect(updated.paragraphs[0].citations).toEqual([
      expect.objectContaining({
        text: "Smith (2024)",
        verified: true,
      }),
    ]);
    expect(updated.summary.citation_count).toBe(1);
    expect(updated.summary.cited_paragraph_count).toBe(1);
    expect(updated.summary.missing_citation_count).toBe(0);
  });

  test("verifying citations marks every selected paragraph citation as verified", () => {
    const source = analysis([
      paragraph({
        citations: [
          { id: "citation-1", text: "Smith (2020)", start: null, end: null, verified: false },
          { id: "citation-2", text: "(Jones, 2021)", start: null, end: null, verified: true },
        ],
      }),
    ]);

    const updated = verifyParagraphCitations(source, "paragraph-1");

    expect(updated.paragraphs[0].citations.every((citation) => citation.verified)).toBe(true);
    expect(updated.summary.citation_count).toBe(2);
  });

  test("empty manual citation text leaves analysis unchanged", () => {
    const source = analysis([paragraph({ needs_citation: true })]);

    const updated = addManualCitation(source, "paragraph-1", "   ");

    expect(updated).toBe(source);
  });
});
