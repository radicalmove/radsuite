import { describe, expect, test } from "vitest";
import {
  canUseRadcastSpeechCleanup,
  clampRadcastSilenceSeconds,
  formatRadcastPauseRemovalCount,
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

  test("blocks speech cleanup when local caption support is unavailable", () => {
    expect(canUseRadcastSpeechCleanup(false, true, false)).toBe(false);
    expect(canUseRadcastSpeechCleanup(false, false, true)).toBe(false);
    expect(canUseRadcastSpeechCleanup(false, false, false)).toBe(true);
    expect(canUseRadcastSpeechCleanup(true, true, true)).toBe(true);
  });

  test("formats zero and singular pause counts explicitly", () => {
    expect(formatRadcastPauseRemovalCount(0)).toBe("0 pauses shortened");
    expect(formatRadcastPauseRemovalCount(1)).toBe("1 pause shortened");
    expect(formatRadcastPauseRemovalCount(2)).toBe("2 pauses shortened");
  });
});
