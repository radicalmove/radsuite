const SILENCE_MIN_SECONDS = 0;
const SILENCE_MAX_SECONDS = 4;
const SILENCE_STEP_SECONDS = 0.25;
const DEFAULT_SILENCE_SECONDS = 1;

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
