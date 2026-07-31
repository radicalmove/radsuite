import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { AnalyseDocxReviewResponse, SavedRadciteReviewSummary } from "../types";
import {
  canUseSavedReviewForReadings,
  listSavedRadciteReviews,
  loadSavedRadciteReview,
} from "./savedReviewCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const savedReview: SavedRadciteReviewSummary = {
  document_id: "document-1",
  project_id: "project-1",
  original_filename: "source.docx",
  source_path: "/app-data/documents/project-1/source.docx",
  source_file_type: "docx",
  paragraph_count: 4,
  citation_count: 2,
  missing_citation_count: 0,
};

const loadedReview: AnalyseDocxReviewResponse = {
  project_id: "project-1",
  project_title: "RADcite Functional Testing",
  document_id: "document-1",
  original_filename: "source.docx",
  source_path: "/app-data/documents/project-1/source.docx",
  source_file_type: "docx",
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

    await expect(listSavedRadciteReviews("project-1")).resolves.toEqual([savedReview]);

    expect(invoke).toHaveBeenCalledWith("list_saved_radcite_reviews", {
      request: {
        project_id: "project-1",
      },
    });
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

  test("only managed DOCX reviews can be reused for readings", () => {
    expect(canUseSavedReviewForReadings(savedReview)).toBe(true);
    expect(
      canUseSavedReviewForReadings({
        ...savedReview,
        source_file_type: "pdf",
      }),
    ).toBe(false);
    expect(
      canUseSavedReviewForReadings({
        ...savedReview,
        source_path: null,
      }),
    ).toBe(false);
  });
});
