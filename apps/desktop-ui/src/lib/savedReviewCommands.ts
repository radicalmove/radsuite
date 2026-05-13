import { invoke } from "@tauri-apps/api/core";
import type { AnalyseDocxReviewResponse, SavedRadciteReviewSummary } from "../types";

export function listSavedRadciteReviews(): Promise<SavedRadciteReviewSummary[]> {
  return invoke<SavedRadciteReviewSummary[]>("list_saved_radcite_reviews");
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
