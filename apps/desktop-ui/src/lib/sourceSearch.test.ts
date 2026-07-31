import { describe, expect, test } from "vitest";
import type { ReviewParagraph } from "../types";
import {
  buildCrossrefSearchUrl,
  buildCrossrefWorksApiUrl,
  buildOpenAlexWorksApiUrl,
  findCitationMatches,
  searchAcademicWorks,
  searchCrossrefWorks,
  searchOpenAlexWorks,
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

  test("builds an OpenAlex works API URL", () => {
    expect(buildOpenAlexWorksApiUrl(" Smith 2024 ", 3)).toBe(
      "https://api.openalex.org/works?search=Smith+2024&per-page=3&select=id%2Cdoi%2Ctitle%2Cauthorships%2Cpublication_year%2Cprimary_location%2Chost_venue%2Cabstract_inverted_index",
    );
  });

  test("finds Crossref results matching a citation author and year", () => {
    const matchingResult = {
      title: "Worked examples in practice",
      authors: "Smith, J.; Jones, P.",
      year: "2024",
      source: "Teaching Journal",
      doi: "10.1000/example",
      url: "https://doi.org/10.1000/example",
      apaCitation: "Smith, J.; Jones, P. (2024). Worked examples in practice.",
    };

    const expected = [matchingResult];

    expect(
      findCitationMatches("Smith (2024)", [
        matchingResult,
        { ...matchingResult, authors: "Taylor, R.", doi: "10.1000/other" },
        { ...matchingResult, year: "2023", doi: "10.1000/older" },
      ]),
    ).toEqual(expected);
    expect(findCitationMatches("(Smith, 2024)", [matchingResult])).toEqual(expected);
  });

  test("returns no verification matches when a citation has no author-year pair", () => {
    expect(
      findCitationMatches("(see the discussion)", [
        {
          title: "Untitled source",
          authors: "Smith, J.",
          year: "2024",
          source: null,
          doi: null,
          url: null,
          apaCitation: "Smith, J. (2024). Untitled source.",
        },
      ]),
    ).toEqual([]);
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
        apaCitation:
          "Smith, J.; Jones, P. (2024). Worked examples in practice. Teaching Journal. https://doi.org/10.1000/example",
      },
    ]);
  });

  test("searches OpenAlex works and formats compact results", async () => {
    const fetcher = async () =>
      ({
        ok: true,
        status: 200,
        json: async () => ({
          results: [
            {
              id: "https://openalex.org/W123",
              doi: "https://doi.org/10.1000/example",
              title: "Worked examples in practice",
              authorships: [
                { author: { display_name: "Jane Smith" } },
                { author: { display_name: "Priya Jones" } },
              ],
              publication_year: 2024,
              host_venue: { display_name: "Teaching Journal" },
              primary_location: { landing_page_url: "https://example.org/worked" },
            },
          ],
        }),
      }) as Response;

    await expect(searchOpenAlexWorks("Smith 2024", fetcher)).resolves.toEqual([
      {
        title: "Worked examples in practice",
        authors: "Jane Smith; Priya Jones",
        year: "2024",
        source: "Teaching Journal",
        doi: "10.1000/example",
        url: "https://doi.org/10.1000/example",
        apaCitation:
          "Jane Smith; Priya Jones (2024). Worked examples in practice. Teaching Journal. https://doi.org/10.1000/example",
      },
    ]);
  });

  test("falls back to OpenAlex when Crossref returns no results", async () => {
    const fetcher = async (input: string) =>
      ({
        ok: true,
        status: 200,
        json: async () =>
          input.includes("api.crossref.org")
            ? { message: { items: [] } }
            : {
                results: [
                  {
                    id: "https://openalex.org/W123",
                    title: "Open source result",
                    authorships: [{ author: { display_name: "Taylor Smith" } }],
                    publication_year: 2023,
                  },
                ],
              },
      }) as Response;

    await expect(searchAcademicWorks("Taylor 2023", "hybrid", fetcher)).resolves.toHaveLength(1);
  });

  test("merges and de-duplicates Crossref and OpenAlex results in hybrid mode", async () => {
    const fetcher = async (input: string) =>
      ({
        ok: true,
        status: 200,
        json: async () =>
          input.includes("api.crossref.org")
            ? {
                message: {
                  items: [
                    {
                      DOI: "10.1000/shared",
                      title: ["Shared result"],
                      author: [{ family: "Smith", given: "Jane" }],
                      issued: { "date-parts": [[2024]] },
                      URL: "https://doi.org/10.1000/shared",
                    },
                  ],
                },
              }
            : {
                results: [
                  {
                    id: "https://openalex.org/W123",
                    doi: "https://doi.org/10.1000/shared",
                    title: "Shared result",
                    authorships: [{ author: { display_name: "Jane Smith" } }],
                    publication_year: 2024,
                  },
                  {
                    id: "https://openalex.org/W456",
                    title: "OpenAlex-only result",
                    authorships: [{ author: { display_name: "Taylor Jones" } }],
                    publication_year: 2023,
                  },
                ],
              },
      }) as Response;

    await expect(searchAcademicWorks("Smith", "hybrid", fetcher)).resolves.toEqual([
      expect.objectContaining({ title: "Shared result" }),
      expect.objectContaining({ title: "OpenAlex-only result" }),
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
