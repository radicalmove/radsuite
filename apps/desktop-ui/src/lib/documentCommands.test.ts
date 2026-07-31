import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import type { SavedRadciteReviewSummary } from "../types";
import { updateRadciteDocument } from "./documentCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const savedReview: SavedRadciteReviewSummary = {
  document_id: "document-1",
  project_id: "project-1",
  original_filename: "source.docx",
  display_name: "Week 1 source",
  source_path: "/app-data/documents/project-1/source.docx",
  source_file_type: "docx",
  doc_variant: "content",
  doc_number: null,
  exclude_from_references: false,
  paragraph_count: 4,
  citation_count: 2,
  missing_citation_count: 0,
};

describe("document commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("updates document metadata with the selected project context", async () => {
    vi.mocked(invoke).mockResolvedValue(savedReview);

    await expect(
      updateRadciteDocument({
        project_id: " project-1 ",
        document_id: "document-1",
        display_name: "Week 1 source",
        doc_number: null,
        doc_variant: "content",
        exclude_from_references: false,
      }),
    ).resolves.toBe(savedReview);

    expect(invoke).toHaveBeenCalledWith("update_radcite_document", {
      request: {
        project_id: "project-1",
        document_id: "document-1",
        display_name: "Week 1 source",
        doc_number: null,
        doc_variant: "content",
        exclude_from_references: false,
      },
    });
  });
});
