import type { EnhancementModel, RadcastTrimRange } from "../types";

const SILENCE_MIN_SECONDS = 0;
const SILENCE_MAX_SECONDS = 4;
const SILENCE_STEP_SECONDS = 0.25;
const DEFAULT_SILENCE_SECONDS = 1;
const MIN_TRIM_OUTPUT_SECONDS = 0.5;

export function effectiveRadcastCleanupEnabled(
  enhancementModel: EnhancementModel,
  cleanupEnabled: boolean,
): boolean {
  return enhancementModel === "none" && cleanupEnabled;
}

export function canUseRadcastSpeechCleanup(
  captionAvailable: boolean,
  shortenPauses: boolean,
  removeFillerWords: boolean,
): boolean {
  return captionAvailable || (!shortenPauses && !removeFillerWords);
}

export function formatRadcastPauseRemovalCount(value: unknown): string {
  const count = typeof value === "number" && Number.isFinite(value)
    ? Math.max(0, Math.floor(value))
    : 0;
  return `${count} pause${count === 1 ? "" : "s"} shortened`;
}

export function clampRadcastSilenceSeconds(value: unknown): number {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return DEFAULT_SILENCE_SECONDS;

  const bounded = Math.min(SILENCE_MAX_SECONDS, Math.max(SILENCE_MIN_SECONDS, numeric));
  return Math.round(bounded / SILENCE_STEP_SECONDS) * SILENCE_STEP_SECONDS;
}

export function formatRadcastSilenceSeconds(value: number): string {
  const seconds = clampRadcastSilenceSeconds(value);
  const display = Number.isInteger(seconds)
    ? String(seconds)
    : seconds.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
  return `${display} second${seconds === 1 ? "" : "s"}`;
}

function effectiveRadcastTrimMinimum(durationSeconds: number): number {
  return Math.max(0.05, Math.min(MIN_TRIM_OUTPUT_SECONDS, durationSeconds));
}

export function normalizeRadcastTrimRange(
  startValue: unknown,
  endValue: unknown,
  durationValue: unknown,
): RadcastTrimRange | null {
  const duration = typeof durationValue === "number" ? durationValue : Number(durationValue);
  if (!Number.isFinite(duration) || duration <= 0) return null;

  const minimumOutput = effectiveRadcastTrimMinimum(duration);
  const startInput = typeof startValue === "number" ? startValue : Number(startValue);
  const endInput = typeof endValue === "number" ? endValue : Number(endValue);
  let start = Number.isFinite(startInput) ? startInput : 0;
  let end = Number.isFinite(endInput) ? endInput : duration;

  start = Math.max(0, Math.min(start, duration - minimumOutput));
  end = Math.max(minimumOutput, Math.min(end, duration));

  if (end - start < minimumOutput) {
    end = Math.min(duration, start + minimumOutput);
  }
  if (end - start < minimumOutput) {
    start = Math.max(0, end - minimumOutput);
  }

  return {
    clip_start_seconds: start,
    clip_end_seconds: end,
  };
}

export function isRadcastFullTrimRange(
  range: RadcastTrimRange | null | undefined,
  durationValue: unknown,
): boolean {
  const duration = typeof durationValue === "number" ? durationValue : Number(durationValue);
  if (!range || !Number.isFinite(duration) || duration <= 0) return true;
  return range.clip_start_seconds <= 0.05 && range.clip_end_seconds >= duration - 0.05;
}

export function formatRadcastTrimSeconds(value: unknown): string {
  const numeric = typeof value === "number" ? value : Number(value);
  const safe = Number.isFinite(numeric) ? Math.max(0, numeric) : 0;
  return `${safe.toFixed(3)}s`;
}

export function clampRadcastPlaybackTime(
  value: unknown,
  range: RadcastTrimRange,
): number {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return range.clip_start_seconds;
  return Math.max(
    range.clip_start_seconds,
    Math.min(range.clip_end_seconds, numeric),
  );
}

export function shouldRestartRadcastPlayback(
  value: unknown,
  range: RadcastTrimRange,
): boolean {
  const numeric = typeof value === "number" ? value : Number(value);
  return !Number.isFinite(numeric)
    || numeric < range.clip_start_seconds
    || numeric >= range.clip_end_seconds;
}
