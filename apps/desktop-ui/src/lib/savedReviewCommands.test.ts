import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { AnalyseDocxReviewResponse, SavedRadciteReviewSummary } from "../types";
import { listSavedRadciteReviews, loadSavedRadciteReview } from "./savedReviewCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const savedReview: SavedRadciteReviewSummary = {
  document_id: "document-1",
  project_id: "project-1",
  original_filename: "source.docx",
  paragraph_count: 4,
  citation_count: 2,
  missing_citation_count: 0,
};

const loadedReview: AnalyseDocxReviewResponse = {
  project_id: "project-1",
  project_title: "RADcite Functional Testing",
  document_id: "document-1",
  original_filename: "source.docx",
  summary: {
    paragraph_count: 4,
    citation_count: 2,
    cited_paragraph_count: 2,
    missing_citation_count: 0,
    linked_citation_count: 1,
    suggested_citation_count: 1,
    unlinked_citation_count: 1,
  },
  paragraphs: [],
};

describe("saved review commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists saved RADcite reviews from the Local DB", async () => {
    vi.mocked(invoke).mockResolvedValue([savedReview]);

    await expect(listSavedRadciteReviews()).resolves.toEqual([savedReview]);

    expect(invoke).toHaveBeenCalledWith("list_saved_radcite_reviews");
  });

  test("loads a saved RADcite review by document id", async () => {
    vi.mocked(invoke).mockResolvedValue(loadedReview);

    await expect(loadSavedRadciteReview("document-1")).resolves.toBe(loadedReview);

    expect(invoke).toHaveBeenCalledWith("load_saved_radcite_review", {
      request: {
        document_id: "document-1",
      },
    });
  });
});
