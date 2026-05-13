import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { AnalyseDocxReviewResponse, ReviewParagraph } from "../types";
import {
  persistAddManualCitation,
  persistMarkParagraphResolved,
  persistVerifyParagraphCitations,
} from "./reviewActionCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

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

describe("review action commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("persists resolved paragraph state with the current document id", async () => {
    const source = analysis([paragraph({ id: "paragraph-1", needs_citation: true })]);
    const refreshed = analysis([paragraph({ id: "paragraph-1", needs_citation: false })]);
    vi.mocked(invoke).mockResolvedValue(refreshed);

    await expect(persistMarkParagraphResolved(source, "paragraph-1")).resolves.toBe(refreshed);

    expect(invoke).toHaveBeenCalledWith("mark_radcite_paragraph_resolved", {
      request: {
        document_id: "document-1",
        paragraph_id: "paragraph-1",
      },
    });
  });

  test("persists citation verification with the current document id", async () => {
    const source = analysis([
      paragraph({
        id: "paragraph-1",
        citations: [
          { id: "citation-1", text: "Smith (2020)", start: null, end: null, verified: false },
        ],
      }),
    ]);
    const refreshed = analysis([
      paragraph({
        id: "paragraph-1",
        citations: [
          { id: "citation-1", text: "Smith (2020)", start: null, end: null, verified: true },
        ],
      }),
    ]);
    vi.mocked(invoke).mockResolvedValue(refreshed);

    await expect(persistVerifyParagraphCitations(source, "paragraph-1")).resolves.toBe(refreshed);

    expect(invoke).toHaveBeenCalledWith("verify_radcite_paragraph_citations", {
      request: {
        document_id: "document-1",
        paragraph_id: "paragraph-1",
      },
    });
  });

  test("persists trimmed manual citation text with the current document id", async () => {
    const source = analysis([paragraph({ id: "paragraph-1", needs_citation: true })]);
    const refreshed = analysis([
      paragraph({
        id: "paragraph-1",
        citations: [
          { id: "citation-1", text: "Jones (2024)", start: null, end: null, verified: true },
        ],
      }),
    ]);
    vi.mocked(invoke).mockResolvedValue(refreshed);

    await expect(
      persistAddManualCitation(source, "paragraph-1", " Jones (2024) "),
    ).resolves.toBe(refreshed);

    expect(invoke).toHaveBeenCalledWith("add_radcite_manual_citation", {
      request: {
        document_id: "document-1",
        paragraph_id: "paragraph-1",
        citation_text: "Jones (2024)",
      },
    });
  });
});
