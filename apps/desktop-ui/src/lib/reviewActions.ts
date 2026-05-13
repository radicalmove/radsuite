import type { AnalyseDocxReviewResponse, ReviewCitation, ReviewParagraph } from "../types";

function recomputeSummary(
  analysis: AnalyseDocxReviewResponse,
  paragraphs: ReviewParagraph[],
): AnalyseDocxReviewResponse {
  return {
    ...analysis,
    summary: {
      paragraph_count: paragraphs.length,
      citation_count: paragraphs.reduce((count, paragraph) => count + paragraph.citations.length, 0),
      cited_paragraph_count: paragraphs.filter((paragraph) => paragraph.citations.length > 0).length,
      missing_citation_count: paragraphs.filter((paragraph) => paragraph.needs_citation).length,
    },
    paragraphs,
  };
}

function updateParagraph(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
  update: (paragraph: ReviewParagraph) => ReviewParagraph,
): AnalyseDocxReviewResponse {
  let didUpdate = false;
  const paragraphs = analysis.paragraphs.map((paragraph) => {
    if (paragraph.id !== paragraphId) {
      return paragraph;
    }
    didUpdate = true;
    return update(paragraph);
  });

  return didUpdate ? recomputeSummary(analysis, paragraphs) : analysis;
}

export function markParagraphResolved(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
): AnalyseDocxReviewResponse {
  return updateParagraph(analysis, paragraphId, (paragraph) => ({
    ...paragraph,
    needs_citation: false,
  }));
}

export function addManualCitation(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
  citationText: string,
): AnalyseDocxReviewResponse {
  const text = citationText.trim();
  if (!text) {
    return analysis;
  }

  return updateParagraph(analysis, paragraphId, (paragraph) => {
    const citation: ReviewCitation = {
      id: `manual-${paragraph.id}-${paragraph.citations.length + 1}`,
      text,
      start: null,
      end: null,
      verified: true,
    };

    return {
      ...paragraph,
      needs_citation: false,
      citations: [...paragraph.citations, citation],
    };
  });
}

export function verifyParagraphCitations(
  analysis: AnalyseDocxReviewResponse,
  paragraphId: string,
): AnalyseDocxReviewResponse {
  return updateParagraph(analysis, paragraphId, (paragraph) => ({
    ...paragraph,
    citations: paragraph.citations.map((citation) => ({
      ...citation,
      verified: true,
    })),
  }));
}
