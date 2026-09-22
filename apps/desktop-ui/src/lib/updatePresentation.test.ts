import type { Update } from "@tauri-apps/plugin-updater";
import { describe, expect, test } from "vitest";
import { UPDATE_CURRENT_NOTICE_MS, formatUpdateCheckError, presentStableUpdateCheck } from "./updatePresentation";

const update = { version: "0.3.0" } as Update;

describe("stable update presentation", () => {
  test("keeps the current-version notice visible for five seconds", () => {
    expect(UPDATE_CURRENT_NOTICE_MS).toBe(5_000);
  });

  test("shows the installed version as current only for a manual check", () => {
    expect(presentStableUpdateCheck({ kind: "current" }, true, "0.2.7")).toEqual({ availableUpdate: null, currentMessage: "RADsuite 0.2.7 is up to date." });
    expect(presentStableUpdateCheck({ kind: "current" }, false, "0.2.7").currentMessage).toBeNull();
  });

  test("preserves an available update and clears the current message", () => {
    expect(presentStableUpdateCheck({ kind: "available", update }, true, "0.2.7")).toEqual({ availableUpdate: update, currentMessage: null });
  });

  test("formats update errors with one consistent prefix", () => {
    expect(formatUpdateCheckError(new Error("offline"))).toBe("Could not check for updates: offline");
    expect(formatUpdateCheckError("Could not check for updates: offline")).toBe("Could not check for updates: offline");
  });
});
