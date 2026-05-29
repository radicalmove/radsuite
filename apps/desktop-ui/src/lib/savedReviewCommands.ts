import { invoke } from "@tauri-apps/api/core";
import type { AnalyseDocxReviewResponse, SavedRadciteReviewSummary } from "../types";

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function listSavedRadciteReviews(
  projectId?: string | null,
): Promise<SavedRadciteReviewSummary[]> {
  return invoke<SavedRadciteReviewSummary[]>("list_saved_radcite_reviews", {
    request: {
      project_id: trimmedOrNull(projectId),
    },
  });
}

export function loadSavedRadciteReview(
  documentId: string,
): Promise<AnalyseDocxReviewResponse> {
  return invoke<AnalyseDocxReviewResponse>("load_saved_radcite_review", {
    request: {
      document_id: documentId,
    },
  });
}
