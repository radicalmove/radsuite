import { describe, expect, it } from "vitest";
import type { RadtTsCapabilityStatus } from "../types";
import {
  buildRadtTsRequest,
  canStartRadtTs,
  type RadtTsDraft,
} from "./radtTsWorkflow";

const capability: RadtTsCapabilityStatus = {
  available: true,
  executable: "/Users/example/RADTTS/.venv/bin/radtts",
  detail: "Available",
};

const draft: RadtTsDraft = {
  text: "A short script.",
  referenceAudioPath: "/tmp/reference.wav",
  referenceText: "Reference voice transcript.",
  quality: "high",
  chunkMode: "sentence",
  pauseMinSeconds: 0.45,
  pauseMaxSeconds: 1.1,
  outputFormat: "mp3",
  outputName: "lesson-intro",
  acknowledgeVoiceClone: true,
};

describe("RAD TTS workflow", () => {
  it("builds the native request with trimmed text and output name", () => {
    expect(buildRadtTsRequest({ ...draft, text: "  A short script.  ", outputName: "  lesson-intro  " }, "project-1")).toEqual({
      project_id: "project-1",
      text: "A short script.",
      reference_audio_path: "/tmp/reference.wav",
      reference_text: "Reference voice transcript.",
      quality: "high",
      chunk_mode: "sentence",
      pause_min_seconds: 0.45,
      pause_max_seconds: 1.1,
      output_format: "mp3",
      output_name: "lesson-intro",
      acknowledge_voice_clone: true,
    });
  });

  it("trims an optional reference transcript and sends null when blank", () => {
    expect(buildRadtTsRequest({ ...draft, referenceText: "  Hello there.  " }, "project-1").reference_text).toBe(
      "Hello there.",
    );
    expect(buildRadtTsRequest({ ...draft, referenceText: "   " }, "project-1").reference_text).toBeNull();
  });

  it("requires local runtime, script, reference audio, authorization, and valid pauses", () => {
    expect(canStartRadtTs(draft, capability)).toBe(true);
    expect(canStartRadtTs({ ...draft, text: "" }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, referenceAudioPath: "" }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, acknowledgeVoiceClone: false }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, pauseMaxSeconds: 0.2 }, capability)).toBe(false);
    expect(canStartRadtTs(draft, { ...capability, available: false })).toBe(false);
  });
});
