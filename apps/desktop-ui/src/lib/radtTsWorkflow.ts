import type { RadtTsCapabilityStatus, RadtTsChunkMode, RadtTsOutputFormat, RadtTsQuality } from "../types";

export const RADTTS_MIN_NEW_TOKENS = 64;
export const RADTTS_MAX_NEW_TOKENS = 8192;
export const RADTTS_DEFAULT_NEW_TOKENS = 1200;

export type RadtTsDraft = {
  text: string;
  referenceAudioPath: string;
  referenceText: string;
  quality: RadtTsQuality;
  chunkMode: RadtTsChunkMode;
  pauseMinSeconds: number;
  pauseMaxSeconds: number;
  maxNewTokens: number;
  outputFormat: RadtTsOutputFormat;
  outputName: string;
  acknowledgeVoiceClone: boolean;
};

export type RadtTsVoicePreferences = Partial<
  Pick<
    RadtTsDraft,
    | "referenceAudioPath"
    | "referenceText"
    | "quality"
    | "chunkMode"
    | "pauseMinSeconds"
    | "pauseMaxSeconds"
    | "maxNewTokens"
    | "outputFormat"
    | "outputName"
  >
>;

export function createDefaultRadtTsDraft(): RadtTsDraft {
  return {
    text: "",
    referenceAudioPath: "",
    referenceText: "",
    quality: "high",
    chunkMode: "sentence",
    pauseMinSeconds: 0.45,
    pauseMaxSeconds: 1.1,
    maxNewTokens: RADTTS_DEFAULT_NEW_TOKENS,
    outputFormat: "mp3",
    outputName: "voice-generation",
    acknowledgeVoiceClone: false,
  };
}

export function mergeRadtTsVoicePreferences(
  draft: RadtTsDraft,
  preferences: RadtTsVoicePreferences | undefined,
): RadtTsDraft {
  return {
    ...draft,
    ...preferences,
    maxNewTokens: clampRadtTsMaxNewTokens(
      preferences?.maxNewTokens ?? draft.maxNewTokens,
    ),
    acknowledgeVoiceClone: false,
  };
}

export type RadtTsRequest = {
  project_id: string | null;
  text: string;
  reference_audio_path: string;
  reference_text: string | null;
  quality: RadtTsQuality;
  chunk_mode: RadtTsChunkMode;
  pause_min_seconds: number;
  pause_max_seconds: number;
  max_new_tokens: number;
  output_format: RadtTsOutputFormat;
  output_name: string;
  acknowledge_voice_clone: boolean;
};

export function canStartRadtTs(
  draft: RadtTsDraft,
  capability: RadtTsCapabilityStatus,
): boolean {
  return (
    capability.available &&
    draft.text.trim().length > 0 &&
    draft.referenceAudioPath.trim().length > 0 &&
    draft.outputName.trim().length > 0 &&
    draft.pauseMinSeconds > 0 &&
    draft.pauseMaxSeconds >= draft.pauseMinSeconds &&
    draft.acknowledgeVoiceClone
  );
}

export function clampRadtTsMaxNewTokens(value: unknown): number {
  const numeric = typeof value === "number" ? value : Number(value);
  if (!Number.isFinite(numeric)) return RADTTS_DEFAULT_NEW_TOKENS;
  return Math.min(
    RADTTS_MAX_NEW_TOKENS,
    Math.max(RADTTS_MIN_NEW_TOKENS, Math.round(numeric)),
  );
}

export function formatRadtTsMaxNewTokens(value: unknown): string {
  return `${clampRadtTsMaxNewTokens(value).toLocaleString("en-US")} tokens`;
}
export function buildRadtTsRequest(
  draft: RadtTsDraft,
  projectId: string | null,
): RadtTsRequest {
  return {
    project_id: projectId,
    text: draft.text.trim(),
    reference_audio_path: draft.referenceAudioPath.trim(),
    reference_text: draft.referenceText.trim() || null,
    quality: draft.quality,
    chunk_mode: draft.chunkMode,
    pause_min_seconds: draft.pauseMinSeconds,
    pause_max_seconds: draft.pauseMaxSeconds,
    max_new_tokens: clampRadtTsMaxNewTokens(draft.maxNewTokens),
    output_format: draft.outputFormat,
    output_name: draft.outputName.trim(),
    acknowledge_voice_clone: draft.acknowledgeVoiceClone,
  };
}
