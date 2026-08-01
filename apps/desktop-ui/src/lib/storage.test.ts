import { describe, expect, test } from "vitest";
import {
  defaultProjectNavStorageState,
  readProjectNavStorageState,
  readRadtTsProjectPreferences,
  readThemeStorage,
  writeProjectNavStorageState,
  writeRadtTsProjectPreferences,
  writeThemeStorage,
  type StorageLike,
} from "./storage";

function memoryStorage(initial: Record<string, string> = {}): StorageLike {
  const values = new Map(Object.entries(initial));
  return {
    getItem(key) {
      return values.get(key) ?? null;
    },
    setItem(key, value) {
      values.set(key, value);
    },
  };
}

describe("safe local storage helpers", () => {
  test("fall back when storage is unavailable or malformed", () => {
    expect(readThemeStorage(null)).toBe("light");
    expect(
      readProjectNavStorageState(
        memoryStorage({ radciteProjectNavState: "{not-json" }),
      ),
    ).toEqual(defaultProjectNavStorageState);
  });

  test("filters invalid project IDs from a stored navigation state", () => {
    const storage = memoryStorage({
      radciteProjectNavState: JSON.stringify({
        version: 1,
        expandedProjectIds: ["project-1", 42, "", "project-2"],
        archivedSectionOpen: true,
      }),
    });

    expect(readProjectNavStorageState(storage)).toEqual({
      version: 1,
      expandedProjectIds: ["project-1", "project-2"],
      archivedSectionOpen: true,
    });
  });

  test("writes theme and navigation state without exposing JSON details to callers", () => {
    const storage = memoryStorage();

    writeThemeStorage(storage, "dark");
    writeProjectNavStorageState(storage, {
      version: 1,
      expandedProjectIds: ["project-1"],
      archivedSectionOpen: false,
    });

    expect(readThemeStorage(storage)).toBe("dark");
    expect(readProjectNavStorageState(storage).expandedProjectIds).toEqual(["project-1"]);
  });

  test("swallows storage read and write failures", () => {
    const brokenStorage: StorageLike = {
      getItem() {
        throw new Error("blocked");
      },
      setItem() {
        throw new Error("quota");
      },
    };

    expect(readThemeStorage(brokenStorage)).toBe("light");
    expect(readProjectNavStorageState(brokenStorage)).toEqual(defaultProjectNavStorageState);
    expect(() => writeThemeStorage(brokenStorage, "dark")).not.toThrow();
    expect(() => writeProjectNavStorageState(brokenStorage, defaultProjectNavStorageState)).not.toThrow();
    expect(() => writeRadtTsProjectPreferences(brokenStorage, "project-1", {})).not.toThrow();
  });

  test("stores RADTTS preferences separately for each project", () => {
    const storage = memoryStorage();
    writeRadtTsProjectPreferences(storage, "project-1", {
      voice: {
        voiceSource: "builtin",
        builtInSpeaker: "Vivian",
        builtInInstruct: "Warm and clear",
        quality: "high",
        pauseSeed: "42",
        maxNewTokens: 1400,
        outputName: "intro",
        referenceText: "Reference voice transcript.",
      },
    });
    writeRadtTsProjectPreferences(storage, "project-2", {
      transcription: { model: "medium" },
    });

    expect(readRadtTsProjectPreferences(storage, "project-1")).toEqual({
      voice: {
        voiceSource: "builtin",
        builtInSpeaker: "Vivian",
        builtInInstruct: "Warm and clear",
        quality: "high",
        pauseSeed: "42",
        maxNewTokens: 1400,
        outputName: "intro",
        referenceText: "Reference voice transcript.",
      },
    });
    expect(readRadtTsProjectPreferences(storage, "project-2")).toEqual({
      transcription: { model: "medium" },
    });
  });

  test("falls back when RADTTS preferences are malformed", () => {
    const storage = memoryStorage({ radsuiteRadtTsPreferences: "{not-json" });
    expect(readRadtTsProjectPreferences(storage, "project-1")).toEqual({});
    expect(readRadtTsProjectPreferences(storage, null)).toEqual({});
  });
});
