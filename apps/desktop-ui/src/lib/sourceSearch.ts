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
  apaCitation: string;
};

export type AcademicSearchProvider = "crossref" | "openalex" | "hybrid";

type SearchFetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "json">>;

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

type OpenAlexWork = {
  id?: unknown;
  doi?: unknown;
  title?: unknown;
  authorships?: unknown;
  publication_year?: unknown;
  host_venue?: unknown;
  primary_location?: unknown;
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

export function buildOpenAlexWorksApiUrl(query: string, rows = 5): string {
  const url = new URL("https://api.openalex.org/works");
  url.searchParams.set("search", query.trim());
  url.searchParams.set("per-page", rows.toString());
  url.searchParams.set(
    "select",
    "id,doi,title,authorships,publication_year,primary_location,host_venue,abstract_inverted_index",
  );
  return url.toString();
}

export function findCitationMatches(
  citationText: string,
  results: CrossrefSourceResult[],
): CrossrefSourceResult[] {
  const author = citationText
    .trim()
    .match(/^(?:[\(\[\{]\s*)?([A-Z][A-Za-z'\u2019\u2010-]+)/)?.[1];
  const year = citationText.match(/\b(?:19|20)\d{2}\b/)?.[0];
  if (!author || !year) {
    return [];
  }

  const normalizedAuthor = author.toLocaleLowerCase();
  return results.filter(
    (result) =>
      result.year === year && result.authors.toLocaleLowerCase().includes(normalizedAuthor),
  );
}

export async function searchCrossrefWorks(
  query: string,
  fetcher: SearchFetcher = fetch,
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

export async function searchOpenAlexWorks(
  query: string,
  fetcher: SearchFetcher = fetch,
): Promise<CrossrefSourceResult[]> {
  const trimmedQuery = query.trim();
  if (!trimmedQuery) {
    return [];
  }

  const response = await fetcher(buildOpenAlexWorksApiUrl(trimmedQuery));
  if (!response.ok) {
    throw new Error(`OpenAlex search failed with status ${response.status}`);
  }

  const payload = (await response.json()) as { results?: unknown };
  const items = Array.isArray(payload.results) ? payload.results : [];

  return items
    .map(parseOpenAlexWork)
    .filter((result): result is CrossrefSourceResult => Boolean(result));
}

export async function searchAcademicWorks(
  query: string,
  provider: AcademicSearchProvider = "hybrid",
  fetcher: SearchFetcher = fetch,
): Promise<CrossrefSourceResult[]> {
  if (provider === "crossref") {
    return searchCrossrefWorks(query, fetcher);
  }
  if (provider === "openalex") {
    return searchOpenAlexWorks(query, fetcher);
  }

  let crossrefResults: CrossrefSourceResult[] = [];
  try {
    crossrefResults = await searchCrossrefWorks(query, fetcher);
  } catch {
    // OpenAlex remains available when Crossref is unavailable or empty.
  }

  try {
    const openAlexResults = await searchOpenAlexWorks(query, fetcher);
    return mergeAcademicResults(crossrefResults, openAlexResults);
  } catch (reason: unknown) {
    if (crossrefResults.length) {
      return crossrefResults;
    }
    throw reason;
  }
}

function mergeAcademicResults(
  primary: CrossrefSourceResult[],
  secondary: CrossrefSourceResult[],
): CrossrefSourceResult[] {
  const merged: CrossrefSourceResult[] = [];
  const seen = new Set<string>();

  for (const result of [...primary, ...secondary]) {
    const key = academicResultKey(result);
    if (seen.has(key)) {
      continue;
    }
    seen.add(key);
    merged.push(result);
    if (merged.length === 5) {
      break;
    }
  }

  return merged;
}

function academicResultKey(result: CrossrefSourceResult): string {
  return (
    result.doi?.toLocaleLowerCase() ||
    result.url?.toLocaleLowerCase() ||
    `${result.title}|${result.year}|${result.authors}`.toLocaleLowerCase()
  );
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

  const authors = formatCrossrefAuthors(work.author);
  const year = crossrefYear(work);
  const source = firstPlainString(work["container-title"]);
  const url = plainString(work.URL) ?? (doi ? `https://doi.org/${doi}` : null);

  return {
    title: title ?? "Untitled source",
    authors,
    year,
    source,
    doi,
    url,
    apaCitation: crossrefCitationText({
      title: title ?? "Untitled source",
      authors,
      year,
      source,
      url,
    }),
  };
}

function parseOpenAlexWork(item: unknown): CrossrefSourceResult | null {
  const work = isRecord(item) ? (item as OpenAlexWork) : {};
  const doi = normaliseDoi(work.doi);
  const title = plainString(work.title);
  if (!title && !doi && !plainString(work.id)) {
    return null;
  }

  const authors = formatOpenAlexAuthors(work.authorships);
  const year = plainYear(work.publication_year);
  const source = openAlexSource(work);
  const url = doi ? `https://doi.org/${doi}` : openAlexLandingUrl(work);

  return {
    title: title ?? "Untitled source",
    authors,
    year,
    source,
    doi,
    url,
    apaCitation: crossrefCitationText({
      title: title ?? "Untitled source",
      authors,
      year,
      source,
      url,
    }),
  };
}

function crossrefCitationText(
  result: Pick<CrossrefSourceResult, "title" | "authors" | "year" | "source" | "url">,
): string {
  return [
    `${result.authors} (${result.year ?? "n.d."}).`,
    `${result.title}.`,
    result.source ? `${result.source}.` : null,
    result.url,
  ]
    .filter(Boolean)
    .join(" ");
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

function formatOpenAlexAuthors(value: unknown): string {
  if (!Array.isArray(value)) {
    return "Unknown author";
  }

  const authors = value
    .map((item) => {
      if (!isRecord(item) || !isRecord(item.author)) {
        return null;
      }
      return plainString(item.author.display_name);
    })
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

function plainYear(value: unknown): string | null {
  return typeof value === "number" || typeof value === "string" ? value.toString() : null;
}

function normaliseDoi(value: unknown): string | null {
  const doi = plainString(value);
  if (!doi) {
    return null;
  }

  const marker = "doi.org/";
  const markerIndex = doi.toLocaleLowerCase().indexOf(marker);
  if (markerIndex >= 0) {
    return doi.slice(markerIndex + marker.length).trim() || null;
  }

  return doi.replace(/^doi:\s*/i, "").trim() || null;
}

function openAlexSource(work: OpenAlexWork): string | null {
  const hostVenue = isRecord(work.host_venue) ? work.host_venue : null;
  const primaryLocation = isRecord(work.primary_location) ? work.primary_location : null;
  const source = primaryLocation && isRecord(primaryLocation.source) ? primaryLocation.source : null;
  return plainString(hostVenue?.display_name) ?? plainString(source?.display_name);
}

function openAlexLandingUrl(work: OpenAlexWork): string | null {
  const primaryLocation = isRecord(work.primary_location) ? work.primary_location : null;
  return plainString(primaryLocation?.landing_page_url) ?? plainString(work.id);
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
