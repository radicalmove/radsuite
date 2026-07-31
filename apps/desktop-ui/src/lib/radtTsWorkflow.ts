import type { RadtTsCapabilityStatus, RadtTsChunkMode, RadtTsOutputFormat, RadtTsQuality } from "../types";

export type RadtTsDraft = {
  text: string;
  referenceAudioPath: string;
  quality: RadtTsQuality;
  chunkMode: RadtTsChunkMode;
  pauseMinSeconds: number;
  pauseMaxSeconds: number;
  outputFormat: RadtTsOutputFormat;
  outputName: string;
  acknowledgeVoiceClone: boolean;
};

export type RadtTsRequest = {
  project_id: string | null;
  text: string;
  reference_audio_path: string;
  quality: RadtTsQuality;
  chunk_mode: RadtTsChunkMode;
  pause_min_seconds: number;
  pause_max_seconds: number;
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
export function buildRadtTsRequest(
  draft: RadtTsDraft,
  projectId: string | null,
): RadtTsRequest {
  return {
    project_id: projectId,
    text: draft.text.trim(),
    reference_audio_path: draft.referenceAudioPath.trim(),
    quality: draft.quality,
    chunk_mode: draft.chunkMode,
    pause_min_seconds: draft.pauseMinSeconds,
    pause_max_seconds: draft.pauseMaxSeconds,
    output_format: draft.outputFormat,
    output_name: draft.outputName.trim(),
    acknowledge_voice_clone: draft.acknowledgeVoiceClone,
  };
}
