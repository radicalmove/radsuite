import type { ParagraphFilter, ReviewParagraph } from "../types";

export function hasLinkedCitation(paragraph: ReviewParagraph): boolean {
  return paragraph.citations.some((citation) => citation.reference_entry_id !== null);
}

export function hasSuggestedCitation(paragraph: ReviewParagraph): boolean {
  return paragraph.citations.some(
    (citation) =>
      citation.reference_entry_id === null && citation.reference_suggestions.length > 0,
  );
}

export function hasUnlinkedCitation(paragraph: ReviewParagraph): boolean {
  return paragraph.citations.some((citation) => citation.reference_entry_id === null);
}

export function paragraphMatchesFilter(
  paragraph: ReviewParagraph,
  filter: ParagraphFilter,
): boolean {
  switch (filter) {
    case "citation-total":
    case "has-citation":
      return paragraph.citations.length > 0;
    case "needs-citation":
      return paragraph.needs_citation;
    case "linked-citation":
      return hasLinkedCitation(paragraph);
    case "suggested-citation":
      return hasSuggestedCitation(paragraph);
    case "unlinked-citation":
      return hasUnlinkedCitation(paragraph);
    case "all":
    default:
      return true;
  }
}

export function filterParagraphs(
  paragraphs: ReviewParagraph[],
  filter: ParagraphFilter,
): ReviewParagraph[] {
  return paragraphs.filter((paragraph) => paragraphMatchesFilter(paragraph, filter));
}
