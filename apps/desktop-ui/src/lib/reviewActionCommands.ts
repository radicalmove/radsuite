import { invoke } from "@tauri-apps/api/core";
import type { AnalyseDocxReviewResponse } from "../types";

type UpdateParagraphReviewRequest = {
  document_id: string;
  paragraph_id: string;
};

type AddManualCitationRequest = UpdateParagraphReviewRequest & {
  citation_text: string;
};

type LinkCitationReferenceRequest = {
  document_id: string;
  citation_id: string;
  reference_entry_id: string;
};

function paragraphRequest(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
): UpdateParagraphReviewRequest {
  return {
    document_id: analysis.document_id,
    paragraph_id: paragraphId,
  };
}

export function persistMarkParagraphResolved(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
): Promise<AnalyseDocxReviewResponse> {
  return invoke<AnalyseDocxReviewResponse>("mark_radcite_paragraph_resolved", {
    request: paragraphRequest(analysis, paragraphId),
  });
}

export function persistVerifyParagraphCitations(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
): Promise<AnalyseDocxReviewResponse> {
  return invoke<AnalyseDocxReviewResponse>("verify_radcite_paragraph_citations", {
    request: paragraphRequest(analysis, paragraphId),
  });
}

export function persistAddManualCitation(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
  citationText: string,
): Promise<AnalyseDocxReviewResponse> {
  const request: AddManualCitationRequest = {
    ...paragraphRequest(analysis, paragraphId),
    citation_text: citationText.trim(),
  };

  return invoke<AnalyseDocxReviewResponse>("add_radcite_manual_citation", {
    request,
  });
}

export function persistLinkCitationToReference(
  analysis: AnalyseDocxReviewResponse,
  citationId: string,
  referenceEntryId: string,
): Promise<AnalyseDocxReviewResponse> {
  const request: LinkCitationReferenceRequest = {
    document_id: analysis.document_id,
    citation_id: citationId,
    reference_entry_id: referenceEntryId,
  };

  return invoke<AnalyseDocxReviewResponse>("link_radcite_citation_reference", {
    request,
  });
}
