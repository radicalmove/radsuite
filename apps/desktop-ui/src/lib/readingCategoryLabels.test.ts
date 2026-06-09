import { describe, expect, test } from "vitest";
import { readingCategoryLabel } from "./readingCategoryLabels";

describe("reading category labels", () => {
  test("shows compulsory readings as Required", () => {
    expect(readingCategoryLabel("compulsory")).toBe("Required");
  });

  test("shows optional readings as Optional", () => {
    expect(readingCategoryLabel("optional")).toBe("Optional");
  });
});
