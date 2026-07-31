import type { RadciteDocumentVariant, SavedRadciteReviewSummary } from "../types";
import type { UpdateRadciteDocumentInput } from "./documentCommands";

export type DocumentEditorDraft = {
  display_name: string;
  doc_number: string;
  doc_variant: RadciteDocumentVariant;
  exclude_from_references: boolean;
};

export function buildDocumentUpdateInput(
  review: SavedRadciteReviewSummary,
  draft: DocumentEditorDraft,
  projectId: string | null,
): UpdateRadciteDocumentInput {
  const rawNumber = draft.doc_number.trim();
  const docNumber = rawNumber ? Number(rawNumber) : null;
  if (docNumber !== null && (!Number.isInteger(docNumber) || docNumber < 1)) {
    throw new Error("Document number must be a positive whole number.");
  }

  return {
    project_id: projectId,
    document_id: review.document_id,
    display_name: draft.display_name.trim(),
    doc_number: docNumber,
    doc_variant: draft.doc_variant,
    exclude_from_references: draft.exclude_from_references,
  };
}

export function createDocumentEditorDraft(
  review: SavedRadciteReviewSummary,
): DocumentEditorDraft {
  return {
    display_name: review.display_name,
    doc_number: review.doc_number === null ? "" : String(review.doc_number),
    doc_variant: review.doc_variant,
    exclude_from_references: review.exclude_from_references,
  };
}

export function applyDocumentSave(
  _draft: DocumentEditorDraft,
  review: SavedRadciteReviewSummary,
): { draft: null; review: SavedRadciteReviewSummary; error: null } {
  return { draft: null, review, error: null };
}

export function retainDocumentEditorDraftAfterFailure(
  draft: DocumentEditorDraft,
  error: string,
): { draft: DocumentEditorDraft; review: null; error: string } {
  return { draft, review: null, error };
}
