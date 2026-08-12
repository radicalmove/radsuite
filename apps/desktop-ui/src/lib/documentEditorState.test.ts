import { describe, expect, test } from "vitest";
import type { SavedRadciteReviewSummary } from "../types";
import {
  applyDocumentSave,
  buildDocumentUpdateInput,
  createDocumentEditorDraft,
  retainDocumentEditorDraftAfterFailure,
  type DocumentEditorDraft,
} from "./documentEditorState";

const savedReview: SavedRadciteReviewSummary = {
  document_id: "document-1",
  project_id: "project-1",
  module_id: null,
  original_filename: "source.docx",
  display_name: "Week 1 source",
  source_path: "/app-data/documents/project-1/source.docx",
  source_file_type: "docx",
  doc_variant: "rise",
  doc_number: 3,
  exclude_from_references: true,
  paragraph_count: 4,
  citation_count: 2,
  missing_citation_count: 0,
};

const draft: DocumentEditorDraft = {
  display_name: "Renamed source",
  doc_number: "4",
  doc_variant: "other",
  exclude_from_references: false,
};

describe("document editor state", () => {
  test("creates an editable draft from saved metadata", () => {
    expect(createDocumentEditorDraft(savedReview)).toEqual({
      display_name: "Week 1 source",
      doc_number: "3",
      doc_variant: "rise",
      exclude_from_references: true,
    });
  });

  test("builds a trimmed update payload and rejects invalid numbers", () => {
    expect(
      buildDocumentUpdateInput(
        savedReview,
        {
          ...draft,
          display_name: "  Renamed source  ",
          doc_number: " 4 ",
        },
        "project-1",
      ),
    ).toEqual({
      project_id: "project-1",
      document_id: "document-1",
      display_name: "Renamed source",
      doc_number: 4,
      doc_variant: "other",
      exclude_from_references: false,
    });

    expect(() =>
      buildDocumentUpdateInput(savedReview, { ...draft, doc_number: "0" }, "project-1"),
    ).toThrow("Document number must be a positive whole number.");
  });

  test("replaces the draft with the saved response after success", () => {
    const result = applyDocumentSave(draft, savedReview);

    expect(result.review).toBe(savedReview);
    expect(result.draft).toBeNull();
    expect(result.error).toBeNull();
  });

  test("retains the draft and records the error after save failure", () => {
    const result = retainDocumentEditorDraftAfterFailure(draft, "Document number is invalid");

    expect(result.draft).toBe(draft);
    expect(result.review).toBeNull();
    expect(result.error).toBe("Document number is invalid");
  });
});
