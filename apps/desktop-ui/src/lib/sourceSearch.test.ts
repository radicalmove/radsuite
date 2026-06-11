import { describe, expect, test } from "vitest";
import type { ReviewParagraph } from "../types";
import {
  buildCrossrefSearchUrl,
  buildCrossrefWorksApiUrl,
  searchCrossrefWorks,
  suggestedSourceSearchQuery,
} from "./sourceSearch";

function paragraph(overrides: Partial<ReviewParagraph>): ReviewParagraph {
  return {
    id: "paragraph-1",
    order_index: 0,
    page: null,
    text:
      "Research shows that students learn more effectively when practice tasks are connected " +
      "to authentic problems.",
    formatted_text: null,
    is_table: false,
    needs_citation: true,
    citations: [],
    ...overrides,
  };
}

describe("source search", () => {
  test("prefers the first unlinked detected citation as the search query", () => {
    const query = suggestedSourceSearchQuery(
      paragraph({
        citations: [
          {
            id: "citation-1",
            text: "Smith (2020)",
            start: null,
            end: null,
            verified: true,
            reference_entry_id: "reference-1",
            reference_suggestions: [],
          },
          {
            id: "citation-2",
            text: "Jones & Patel, 2021",
            start: null,
            end: null,
            verified: false,
            reference_entry_id: null,
            reference_suggestions: [],
          },
        ],
      }),
    );

    expect(query).toEqual({
      query: "Jones & Patel, 2021",
      basis: "detected citation",
    });
  });

  test("falls back to any citation when all citations are already linked", () => {
    const query = suggestedSourceSearchQuery(
      paragraph({
        citations: [
          {
            id: "citation-1",
            text: "Smith (2020)",
            start: null,
            end: null,
            verified: true,
            reference_entry_id: "reference-1",
            reference_suggestions: [],
          },
        ],
      }),
    );

    expect(query).toEqual({
      query: "Smith (2020)",
      basis: "detected citation",
    });
  });

  test("falls back to paragraph keywords when no citation was detected", () => {
    const query = suggestedSourceSearchQuery(
      paragraph({
        text: "Authentic practice problems help students connect theory to applied learning tasks.",
      }),
    );

    expect(query).toEqual({
      query: "authentic practice problems students connect theory",
      basis: "paragraph keywords",
    });
  });

  test("builds a Crossref metadata search URL", () => {
    expect(buildCrossrefSearchUrl(" Jones & Patel, 2021 ")).toBe(
      "https://search.crossref.org/?q=Jones+%26+Patel%2C+2021",
    );
  });

  test("builds a Crossref works API URL", () => {
    expect(buildCrossrefWorksApiUrl(" Jones & Patel, 2021 ", 3)).toBe(
      "https://api.crossref.org/works?query.bibliographic=Jones+%26+Patel%2C+2021&rows=3&select=DOI%2Ctitle%2Cauthor%2Cissued%2Cpublished-print%2Cpublished-online%2Ccontainer-title%2CURL",
    );
  });

  test("searches Crossref works and formats compact results", async () => {
    const fetcher = async () =>
      ({
        ok: true,
        status: 200,
        json: async () => ({
          message: {
            items: [
              {
                DOI: "10.1000/example",
                title: ["Worked examples in practice"],
                author: [
                  { given: "Jane", family: "Smith" },
                  { given: "Priya", family: "Jones" },
                ],
                issued: { "date-parts": [[2024, 3, 1]] },
                "container-title": ["Teaching Journal"],
                URL: "https://doi.org/10.1000/example",
              },
            ],
          },
        }),
      }) as Response;

    await expect(searchCrossrefWorks("Smith 2024 worked examples", fetcher)).resolves.toEqual([
      {
        title: "Worked examples in practice",
        authors: "Smith, J.; Jones, P.",
        year: "2024",
        source: "Teaching Journal",
        doi: "10.1000/example",
        url: "https://doi.org/10.1000/example",
      },
    ]);
  });

  test("throws a useful error when Crossref search fails", async () => {
    const fetcher = async () =>
      ({
        ok: false,
        status: 503,
        json: async () => ({}),
      }) as Response;

    await expect(searchCrossrefWorks("Smith 2024", fetcher)).rejects.toThrow(
      "Crossref search failed with status 503",
    );
  });

  test("drops Crossref works that have neither title nor DOI", async () => {
    const fetcher = async () =>
      ({
        ok: true,
        status: 200,
        json: async () => ({
          message: {
            items: [{ author: [{ family: "Smith" }] }],
          },
        }),
      }) as Response;

    await expect(searchCrossrefWorks("Smith", fetcher)).resolves.toEqual([]);
  });

  test("returns null when there is nothing meaningful to search", () => {
    expect(suggestedSourceSearchQuery(paragraph({ text: "and the or but" }))).toBeNull();
  });
});
