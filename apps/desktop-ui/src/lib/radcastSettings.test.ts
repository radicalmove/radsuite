import { describe, expect, test } from "vitest";
import {
  canUseRadcastSpeechCleanup,
  clampRadcastSilenceSeconds,
  effectiveRadcastCleanupEnabled,
  formatRadcastPauseRemovalCount,
  formatRadcastSilenceSeconds,
  formatRadcastTrimSeconds,
  isRadcastFullTrimRange,
  clampRadcastPlaybackTime,
  shouldRestartRadcastPlayback,
  normalizeRadcastTrimRange,
} from "./radcastSettings";

describe("RADcast cleanup policy", () => {
  test("uses generic cleanup only for standard processing", () => {
    expect(effectiveRadcastCleanupEnabled("none", true)).toBe(true);
    expect(effectiveRadcastCleanupEnabled("none", false)).toBe(false);
    expect(effectiveRadcastCleanupEnabled("resemble", true)).toBe(false);
    expect(effectiveRadcastCleanupEnabled("deepfilternet", true)).toBe(false);
    expect(effectiveRadcastCleanupEnabled("studio", true)).toBe(false);
    expect(effectiveRadcastCleanupEnabled("studio_v18", true)).toBe(false);
  });
});

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

describe("RADcast trim settings", () => {
  test("normalizes trim boundaries against the source duration", () => {
    expect(normalizeRadcastTrimRange(-3, 14, 10)).toEqual({
      clip_start_seconds: 0,
      clip_end_seconds: 10,
    });
    expect(normalizeRadcastTrimRange(2, 6, 10)).toEqual({
      clip_start_seconds: 2,
      clip_end_seconds: 6,
    });
  });

  test("keeps a trim selection at least half a second long", () => {
    expect(normalizeRadcastTrimRange(4, 4.1, 10)).toEqual({
      clip_start_seconds: 4,
      clip_end_seconds: 4.5,
    });
    expect(normalizeRadcastTrimRange(9.9, 10, 10)).toEqual({
      clip_start_seconds: 9.5,
      clip_end_seconds: 10,
    });
  });

  test("uses the complete source for short recordings", () => {
    expect(normalizeRadcastTrimRange(0.1, 0.2, 0.2)).toEqual({
      clip_start_seconds: 0,
      clip_end_seconds: 0.2,
    });
  });

  test("recognizes full-source ranges and formats trim metrics", () => {
    const fullRange = normalizeRadcastTrimRange(0, 10, 10);
    expect(fullRange).not.toBeNull();
    expect(isRadcastFullTrimRange(fullRange, 10)).toBe(true);
    expect(isRadcastFullTrimRange({ clip_start_seconds: 1, clip_end_seconds: 10 }, 10)).toBe(false);
    expect(formatRadcastTrimSeconds(2.345)).toBe("2.345s");
    expect(formatRadcastTrimSeconds(2.3456)).toBe("2.346s");
  });

  test("keeps the source player playhead inside the selected range", () => {
    const range = { clip_start_seconds: 10, clip_end_seconds: 20 };
    expect(clampRadcastPlaybackTime(4, range)).toBe(10);
    expect(clampRadcastPlaybackTime(15, range)).toBe(15);
    expect(clampRadcastPlaybackTime(24, range)).toBe(20);
  });

  test("restarts playback at the trim start after reaching the trim end", () => {
    const range = { clip_start_seconds: 10, clip_end_seconds: 20 };
    expect(shouldRestartRadcastPlayback(9, range)).toBe(true);
    expect(shouldRestartRadcastPlayback(10, range)).toBe(false);
    expect(shouldRestartRadcastPlayback(19.9, range)).toBe(false);
    expect(shouldRestartRadcastPlayback(20, range)).toBe(true);
  });
});
