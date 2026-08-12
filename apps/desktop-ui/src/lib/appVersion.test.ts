import { describe, expect, test } from "vitest";
import { displayAppVersion } from "./appVersion";

describe("application version display", () => {
  test("renders a compact version label for the header", () => {
    expect(displayAppVersion("0.2.1")).toBe("v0.2.1");
  });

  test("does not show an empty version label", () => {
    expect(displayAppVersion(null)).toBe("Version unavailable");
  });

  test("normalizes a version that already includes the v prefix", () => {
    expect(displayAppVersion("v0.2.1")).toBe("v0.2.1");
  });
});
