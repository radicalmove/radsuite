import { describe, expect, test } from "vitest";
import {
  clampRadcastSilenceSeconds,
  formatRadcastSilenceSeconds,
} from "./radcastSettings";

describe("RADcast pause settings", () => {
  test("clamps pause limits to quarter-second steps in the supported range", () => {
    expect(clampRadcastSilenceSeconds(-1)).toBe(0);
    expect(clampRadcastSilenceSeconds(0.37)).toBe(0.25);
    expect(clampRadcastSilenceSeconds(3.88)).toBe(4);
    expect(clampRadcastSilenceSeconds(8)).toBe(4);
  });

  test("formats the slider value for the pause label", () => {
    expect(formatRadcastSilenceSeconds(0)).toBe("0 seconds");
    expect(formatRadcastSilenceSeconds(0.25)).toBe("0.25 seconds");
    expect(formatRadcastSilenceSeconds(1)).toBe("1 second");
    expect(formatRadcastSilenceSeconds(4)).toBe("4 seconds");
  });
});
