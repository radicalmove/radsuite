import { describe, expect, test } from "vitest";
import type { AnalyseDocxReviewResponse, ImportDocumentReadingsResponse } from "../types";
import { describeReadingImport, shouldAutoImportReadings } from "./documentReadingsWorkflow";

const analysis: AnalyseDocxReviewResponse = {
  project_id: "project-1",
  project_title: "Course",
  document_id: "document-1",
  original_filename: "module-1.docx",
  display_name: "Module 1",
  source_path: "/Users/tester/Desktop/module-1.docx",
  source_file_type: "docx",
  doc_variant: "content",
  doc_number: null,
  exclude_from_references: false,
  summary: {
    paragraph_count: 2,
    citation_count: 0,
    cited_paragraph_count: 0,
    missing_citation_count: 0,
    linked_citation_count: 0,
    suggested_citation_count: 0,
    unlinked_citation_count: 0,
  },
  paragraphs: [],
};

describe("document readings workflow", () => {
  test("auto-imports readings after a successful document analysis", () => {
    expect(shouldAutoImportReadings(analysis, analysis.source_path ?? "")).toBe(true);
  });

  test("does not auto-import when the analysed document has no usable source path", () => {
    expect(shouldAutoImportReadings(analysis, "  ")).toBe(false);
  });

  test("describes a successful readings import for the user", () => {
    const result: ImportDocumentReadingsResponse = {
      candidate_count: 3,
      saved_count: 3,
      created_module_count: 1,
      unassigned_count: 0,
      failed_file_count: 0,
    };

    expect(describeReadingImport(result)).toBe(
      "Processed 3 of 3 detected readings. Created 1 module.",
    );
  });
});
