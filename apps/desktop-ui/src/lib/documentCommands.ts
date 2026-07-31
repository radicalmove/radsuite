import { invoke } from "@tauri-apps/api/core";
import type {
  RadciteDocumentVariant,
  RadciteReviewReportExport,
  SavedRadciteReviewSummary,
} from "../types";

export type UpdateRadciteDocumentInput = {
  project_id?: string | null;
  document_id: string;
  display_name: string;
  doc_number: number | null;
  doc_variant: RadciteDocumentVariant;
  exclude_from_references: boolean;
};

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}
export function updateRadciteDocument(
  input: UpdateRadciteDocumentInput,
): Promise<SavedRadciteReviewSummary> {
  return invoke<SavedRadciteReviewSummary>("update_radcite_document", {
    request: {
      project_id: trimmedOrNull(input.project_id),
      document_id: input.document_id,
      display_name: input.display_name,
      doc_number: input.doc_number,
      doc_variant: input.doc_variant,
      exclude_from_references: input.exclude_from_references,
    },
  });
}

export function exportRadciteReviewReport(
  documentId: string,
): Promise<RadciteReviewReportExport> {
  return invoke<RadciteReviewReportExport>("export_radcite_review_report", {
    request: { document_id: documentId },
  });
}
