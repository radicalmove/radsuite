import { describe, expect, test } from "vitest";
import type { RadtTsCapabilityStatus } from "../types";
import {
  buildClipRequest,
  buildTranscriptionRequest,
  canStartClip,
  canStartTranscription,
  type RadtTsClipDraft,
  type RadtTsTranscriptionDraft,
} from "./radtTsToolsWorkflow";

const capability: RadtTsCapabilityStatus = {
  available: true,
  executable: "/usr/local/bin/radtts",
  detail: "available",
  supports_builtin_voices: true,
  builtin_voices: ["Ryan", "Vivian"],
};

const transcription: RadtTsTranscriptionDraft = {
  audioPath: " /tmp/lecture.mp3 ",
  name: " lecture-1 ",
  model: "small",
  language: " en ",
  beamSize: 5,
};

const clip: RadtTsClipDraft = {
  audioPath: " /tmp/lecture.mp3 ",
  segmentsJsonPath: " /tmp/lecture.segments.json ",
  outputName: " opening ",
  boundaryMode: "phrases",
  startPhrase: "Welcome",
  endPhrase: "Goodbye",
  startTime: 0,
  endTime: 10,
  verificationMode: "strict",
  outputFormat: "mp3",
};

describe("RADTTS transcription and clip workflow", () => {
  test("trims transcription requests and requires a usable audio path", () => {
    expect(canStartTranscription(transcription, capability)).toBe(true);
    expect(buildTranscriptionRequest(transcription, "project-1")).toMatchObject({
      project_id: "project-1",
      audio_path: "/tmp/lecture.mp3",
      name: "lecture-1",
      language: "en",
    });
    expect(
      canStartTranscription({ ...transcription, audioPath: "" }, capability),
    ).toBe(false);
  });

  test("builds phrase and time clip boundaries", () => {
    expect(canStartClip(clip, capability)).toBe(true);
    expect(buildClipRequest(clip, "project-1")).toMatchObject({
      start_phrase: "Welcome",
      end_phrase: "Goodbye",
      start_time: null,
      end_time: null,
    });
    const timed = { ...clip, boundaryMode: "times" as const };
    expect(buildClipRequest(timed, "project-1")).toMatchObject({
      start_phrase: null,
      end_phrase: null,
      start_time: 0,
      end_time: 10,
    });
    expect(canStartClip({ ...timed, endTime: 0 }, capability)).toBe(false);
  });
});
