import type {
  RadtTsCapabilityStatus,
  RadtTsOutputFormat,
  RadtTsVerificationMode,
} from "../types";

export type RadtTsTranscriptionDraft = {
  audioPath: string;
  name: string;
  model: string;
  language: string;
  beamSize: number;
};

export type RadtTsClipDraft = {
  audioPath: string;
  segmentsJsonPath: string;
  outputName: string;
  boundaryMode: "phrases" | "times";
  startPhrase: string;
  endPhrase: string;
  startTime: number;
  endTime: number;
  verificationMode: RadtTsVerificationMode;
  outputFormat: RadtTsOutputFormat;
};

export function canStartTranscription(
  draft: RadtTsTranscriptionDraft,
  capability: RadtTsCapabilityStatus,
): boolean {
  return (
    capability.available &&
    draft.audioPath.trim().length > 0 &&
    draft.name.trim().length > 0 &&
    draft.model.trim().length > 0 &&
    Number.isInteger(draft.beamSize) &&
    draft.beamSize >= 1 &&
    draft.beamSize <= 10
  );
}

export function canStartClip(
  draft: RadtTsClipDraft,
  capability: RadtTsCapabilityStatus,
): boolean {
  const hasBoundaries =
    draft.boundaryMode === "phrases"
      ? draft.startPhrase.trim().length > 0 && draft.endPhrase.trim().length > 0
      : draft.startTime >= 0 && draft.endTime > draft.startTime;
  return (
    capability.available &&
    draft.audioPath.trim().length > 0 &&
    draft.segmentsJsonPath.trim().length > 0 &&
    draft.outputName.trim().length > 0 &&
    hasBoundaries
  );
}

export function buildTranscriptionRequest(
  draft: RadtTsTranscriptionDraft,
  projectId: string | null,
) {
  return {
    project_id: projectId,
    audio_path: draft.audioPath.trim(),
    name: draft.name.trim(),
    model: draft.model,
    language: draft.language.trim() || null,
    beam_size: draft.beamSize,
  };
}

export function buildClipRequest(draft: RadtTsClipDraft, projectId: string | null) {
  return {
    project_id: projectId,
    audio_path: draft.audioPath.trim(),
    segments_json_path: draft.segmentsJsonPath.trim(),
    output_name: draft.outputName.trim(),
    start_time: draft.boundaryMode === "times" ? draft.startTime : null,
    end_time: draft.boundaryMode === "times" ? draft.endTime : null,
    start_phrase: draft.boundaryMode === "phrases" ? draft.startPhrase.trim() : null,
    end_phrase: draft.boundaryMode === "phrases" ? draft.endPhrase.trim() : null,
    verification_mode: draft.verificationMode,
    output_format: draft.outputFormat,
  };
}
