import type { ReviewParagraph } from "../types";

export type SourceSearchSuggestion = {
  query: string;
  basis: "detected citation" | "paragraph keywords";
};

const stopWords = new Set([
  "about",
  "after",
  "also",
  "although",
  "among",
  "and",
  "because",
  "been",
  "before",
  "between",
  "both",
  "but",
  "can",
  "could",
  "does",
  "during",
  "each",
  "from",
  "have",
  "help",
  "helps",
  "into",
  "more",
  "most",
  "must",
  "not",
  "often",
  "only",
  "other",
  "over",
  "should",
  "shows",
  "some",
  "such",
  "than",
  "that",
  "their",
  "them",
  "then",
  "there",
  "these",
  "they",
  "this",
  "those",
  "through",
  "under",
  "very",
  "was",
  "were",
  "when",
  "where",
  "which",
  "while",
  "with",
  "within",
  "without",
  "would",
  "your",
]);

export function suggestedSourceSearchQuery(
  paragraph: ReviewParagraph,
): SourceSearchSuggestion | null {
  const citation =
    paragraph.citations.find((item) => !item.reference_entry_id) ?? paragraph.citations[0];
  const citationText = citation?.text.trim();
  if (citationText) {
    return {
      query: citationText,
      basis: "detected citation",
    };
  }

  const keywordQuery = paragraphKeywordQuery(paragraph.text);
  return keywordQuery
    ? {
        query: keywordQuery,
        basis: "paragraph keywords",
      }
    : null;
}

export function buildCrossrefSearchUrl(query: string): string {
  const url = new URL("https://search.crossref.org/");
  url.searchParams.set("q", query.trim());
  return url.toString();
}

function paragraphKeywordQuery(text: string): string | null {
  const keywordStats = new Map<string, { count: number; firstIndex: number }>();
  const words = text.toLowerCase().match(/[a-z][a-z'-]{3,}/g) ?? [];

  for (const [index, word] of words.entries()) {
    const normalized = word.replace(/^'+|'+$/g, "");
    if (!normalized || stopWords.has(normalized)) {
      continue;
    }

    const existing = keywordStats.get(normalized);
    if (existing) {
      existing.count += 1;
    } else {
      keywordStats.set(normalized, { count: 1, firstIndex: index });
    }
  }

  const keywords = [...keywordStats.entries()]
    .sort(
      ([leftWord, left], [rightWord, right]) =>
        right.count - left.count ||
        left.firstIndex - right.firstIndex ||
        leftWord.localeCompare(rightWord),
    )
    .slice(0, 6)
    .map(([word]) => word);

  return keywords.length ? keywords.join(" ") : null;
}
