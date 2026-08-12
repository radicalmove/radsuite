import { describe, expect, it } from "vitest";
import type { RadtTsCapabilityStatus } from "../types";
import {
  buildRadtTsRequest,
  canStartRadtTs,
  clampRadtTsMaxNewTokens,
  createDefaultRadtTsDraft,
  formatRadtTsMaxNewTokens,
  mergeRadtTsVoicePreferences,
  parseRadtTsPauseSeed,
  type RadtTsDraft,
} from "./radtTsWorkflow";

const capability: RadtTsCapabilityStatus = {
  available: true,
  executable: "/Users/example/RADTTS/.venv/bin/radtts",
  detail: "Available",
};

const draft: RadtTsDraft = {
  text: "A short script.",
  voiceSource: "reference",
  referenceAudioPath: "/tmp/reference.wav",
  referenceText: "Reference voice transcript.",
  builtInSpeaker: "Vivian",
  builtInInstruct: "Warm and clear",
  quality: "high",
  chunkMode: "sentence",
  pauseMinSeconds: 0.45,
  pauseMaxSeconds: 1.1,
  pauseSeed: "",
  maxNewTokens: 1200,
  outputFormat: "mp3",
  outputName: "lesson-intro",
  acknowledgeVoiceClone: true,
};

describe("RAD TTS workflow", () => {
  it("builds the native request with trimmed text and output name", () => {
    expect(buildRadtTsRequest({ ...draft, text: "  A short script.  ", outputName: "  lesson-intro  " }, "project-1")).toEqual({
      project_id: "project-1",
      text: "A short script.",
      voice_source: "reference",
      reference_audio_path: "/tmp/reference.wav",
      reference_text: "Reference voice transcript.",
      built_in_speaker: null,
      built_in_instruct: null,
      quality: "high",
      chunk_mode: "sentence",
      pause_min_seconds: 0.45,
      pause_max_seconds: 1.1,
      pause_seed: null,
      max_new_tokens: 1200,
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

  it("starts a new project from blank voice settings instead of carrying prior text", () => {
    const freshDraft = createDefaultRadtTsDraft();
    expect(
      mergeRadtTsVoicePreferences(freshDraft, undefined).referenceText,
    ).toBe("");
    expect(
      mergeRadtTsVoicePreferences(freshDraft, {
        referenceText: "Saved project transcript.",
      }).referenceText,
    ).toBe("Saved project transcript.");
    expect(
      mergeRadtTsVoicePreferences(freshDraft, { quality: "fast" }).maxNewTokens,
    ).toBe(1200);
    expect(mergeRadtTsVoicePreferences(freshDraft, undefined).pauseSeed).toBe("");
    const unsavedDraft = { ...freshDraft, pauseSeed: "42" };
    expect(
      mergeRadtTsVoicePreferences(createDefaultRadtTsDraft(), undefined).pauseSeed,
    ).toBe("");
    expect(unsavedDraft.pauseSeed).toBe("42");
    expect(
      mergeRadtTsVoicePreferences(freshDraft, { pauseSeed: " 42 " }).pauseSeed,
    ).toBe("42");
    expect(
      mergeRadtTsVoicePreferences(freshDraft, { pauseSeed: "not-an-integer" }).pauseSeed,
    ).toBe("");
    expect(
      mergeRadtTsVoicePreferences(freshDraft, { maxNewTokens: 9000 }).maxNewTokens,
    ).toBe(8192);
  });

  it("falls back to reference voice when an old built-in preference is loaded", () => {
    expect(
      mergeRadtTsVoicePreferences(createDefaultRadtTsDraft(), {
        voiceSource: "builtin",
      }).voiceSource,
    ).toBe("reference");
  });

  it("does not enable unsupported built-in voice generation", () => {
    const builtinDraft = {
      ...draft,
      voiceSource: "builtin" as const,
      referenceAudioPath: "",
      referenceText: "",
      builtInSpeaker: "Vivian",
      builtInInstruct: "Warm and clear",
      acknowledgeVoiceClone: false,
    };
    expect(buildRadtTsRequest(builtinDraft, "project-1")).toMatchObject({
      voice_source: "builtin",
      reference_audio_path: null,
      reference_text: null,
      built_in_speaker: "Vivian",
      built_in_instruct: "Warm and clear",
      acknowledge_voice_clone: false,
    });
    expect(canStartRadtTs(builtinDraft, capability)).toBe(false);
    expect(canStartRadtTs({ ...builtinDraft, builtInSpeaker: "" }, capability)).toBe(false);
  });

  it("defaults and formats the generation budget", () => {
    expect(createDefaultRadtTsDraft().maxNewTokens).toBe(1200);
    expect(formatRadtTsMaxNewTokens(1200)).toBe("1,200 tokens");
    expect(formatRadtTsMaxNewTokens(64)).toBe("64 tokens");
  });

  it("clamps generation budgets to the supported RADTTS range", () => {
    expect(clampRadtTsMaxNewTokens(63)).toBe(64);
    expect(clampRadtTsMaxNewTokens(8193)).toBe(8192);
    expect(clampRadtTsMaxNewTokens("not-a-number")).toBe(1200);
  });

  it("normalizes optional pause seeds and forwards only safe integers", () => {
    expect(parseRadtTsPauseSeed("")).toBeNull();
    expect(parseRadtTsPauseSeed("  ")).toBeNull();
    expect(parseRadtTsPauseSeed("42")).toBe(42);
    expect(parseRadtTsPauseSeed("-7")).toBe(-7);
    expect(parseRadtTsPauseSeed("1.5")).toBeNull();
    expect(parseRadtTsPauseSeed("9007199254740992")).toBeNull();
    expect(parseRadtTsPauseSeed("not-a-number")).toBeNull();
    expect(buildRadtTsRequest({ ...draft, pauseSeed: "42" }, "project-1").pause_seed).toBe(42);
  });

  it("requires local runtime, the selected voice source, authorization when cloning, and valid pauses", () => {
    expect(canStartRadtTs(draft, capability)).toBe(true);
    expect(canStartRadtTs({ ...draft, text: "" }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, referenceAudioPath: "" }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, acknowledgeVoiceClone: false }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, voiceSource: "builtin", referenceAudioPath: "", builtInSpeaker: "", acknowledgeVoiceClone: false }, capability)).toBe(false);
    expect(canStartRadtTs({ ...draft, pauseMaxSeconds: 0.2 }, capability)).toBe(false);
    expect(canStartRadtTs(draft, { ...capability, available: false })).toBe(false);
  });
});
