import { describe, expect, test } from "vitest";

import { helpFaqs, helpSections } from "./helpContent";

describe("RADsuite help content", () => {
  test("covers the main RADcite workflow from import to export", () => {
    expect(helpSections.map((section) => section.id)).toEqual([
      "getting-started",
      "citation-review",
      "readings-and-exports",
      "local-media-tools",
    ]);

    const guideText = helpSections
      .flatMap((section) => [section.title, section.summary, ...section.steps])
      .join(" ");

    expect(guideText).toContain("Analyse");
    expect(guideText).toContain("Use for readings");
    expect(guideText).toContain("Copy search query");
    expect(guideText).toContain("Export");
    expect(guideText).toContain("RADcast");
    expect(guideText).toContain("RADTTS");
  });

  test("answers the local-first and recovery questions users need", () => {
    expect(helpFaqs).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ question: "Where is my work saved?" }),
        expect.objectContaining({ question: "What does cloud sync mean?" }),
        expect.objectContaining({ question: "How do I reuse an analysed document?" }),
        expect.objectContaining({ question: "How do I restore something I archived?" }),
        expect.objectContaining({ question: "Why is voice generation unavailable?" }),
      ]),
    );
  });
});
