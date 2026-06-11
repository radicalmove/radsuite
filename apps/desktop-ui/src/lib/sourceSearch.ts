import type { ReviewParagraph } from "../types";

export type SourceSearchSuggestion = {
  query: string;
  basis: "detected citation" | "paragraph keywords";
};

export type CrossrefSourceResult = {
  title: string;
  authors: string;
  year: string | null;
  source: string | null;
  doi: string | null;
  url: string | null;
};

type CrossrefFetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "json">>;

type CrossrefAuthor = {
  given?: unknown;
  family?: unknown;
};

type CrossrefWork = {
  DOI?: unknown;
  title?: unknown;
  author?: unknown;
  issued?: unknown;
  "published-print"?: unknown;
  "published-online"?: unknown;
  "container-title"?: unknown;
  URL?: unknown;
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

export function buildCrossrefWorksApiUrl(query: string, rows = 5): string {
  const url = new URL("https://api.crossref.org/works");
  url.searchParams.set("query.bibliographic", query.trim());
  url.searchParams.set("rows", rows.toString());
  url.searchParams.set(
    "select",
    "DOI,title,author,issued,published-print,published-online,container-title,URL",
  );
  return url.toString();
}

export async function searchCrossrefWorks(
  query: string,
  fetcher: CrossrefFetcher = fetch,
): Promise<CrossrefSourceResult[]> {
  const trimmedQuery = query.trim();
  if (!trimmedQuery) {
    return [];
  }

  const response = await fetcher(buildCrossrefWorksApiUrl(trimmedQuery));
  if (!response.ok) {
    throw new Error(`Crossref search failed with status ${response.status}`);
  }

  const payload = (await response.json()) as {
    message?: {
      items?: unknown;
    };
  };
  const items = Array.isArray(payload.message?.items) ? payload.message.items : [];

  return items
    .map(parseCrossrefWork)
    .filter((result): result is CrossrefSourceResult => Boolean(result));
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

function parseCrossrefWork(item: unknown): CrossrefSourceResult | null {
  const work = isRecord(item) ? (item as CrossrefWork) : {};
  const doi = plainString(work.DOI);
  const title = firstPlainString(work.title);
  if (!title && !doi) {
    return null;
  }

  const url = plainString(work.URL) ?? (doi ? `https://doi.org/${doi}` : null);

  return {
    title: title ?? "Untitled source",
    authors: formatCrossrefAuthors(work.author),
    year: crossrefYear(work),
    source: firstPlainString(work["container-title"]),
    doi,
    url,
  };
}

function formatCrossrefAuthors(value: unknown): string {
  if (!Array.isArray(value)) {
    return "Unknown author";
  }

  const authors = value
    .map(formatCrossrefAuthor)
    .filter((author): author is string => Boolean(author))
    .slice(0, 3);

  if (!authors.length) {
    return "Unknown author";
  }

  return value.length > 3 ? `${authors.join("; ")}; et al.` : authors.join("; ");
}

function formatCrossrefAuthor(value: unknown): string | null {
  if (!isRecord(value)) {
    return null;
  }

  const author = value as CrossrefAuthor;
  const family = plainString(author.family);
  const given = plainString(author.given);
  if (!family) {
    return given;
  }

  const initials = given
    ?.split(/\s+/)
    .filter(Boolean)
    .map((part) => `${part.charAt(0).toUpperCase()}.`)
    .join(" ");

  return initials ? `${family}, ${initials}` : family;
}

function crossrefYear(work: CrossrefWork): string | null {
  return (
    yearFromDateParts(work.issued) ??
    yearFromDateParts(work["published-print"]) ??
    yearFromDateParts(work["published-online"])
  );
}

function yearFromDateParts(value: unknown): string | null {
  if (!isRecord(value)) {
    return null;
  }

  const dateParts = value["date-parts"];
  if (!Array.isArray(dateParts) || !Array.isArray(dateParts[0])) {
    return null;
  }

  const year = dateParts[0][0];
  return typeof year === "number" || typeof year === "string" ? year.toString() : null;
}

function firstPlainString(value: unknown): string | null {
  if (Array.isArray(value)) {
    return plainString(value[0]);
  }

  return plainString(value);
}

function plainString(value: unknown): string | null {
  return typeof value === "string" && value.trim() ? value.trim() : null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
