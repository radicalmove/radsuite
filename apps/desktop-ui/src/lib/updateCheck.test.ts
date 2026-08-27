import type { Update } from "@tauri-apps/plugin-updater";
import { describe, expect, test, vi } from "vitest";
import type { StorageLike } from "./storage";
import { performStableUpdateCheck } from "./updateCheck";
import { UPDATE_CHECK_INTERVAL_MS, readUpdateStorageState } from "./updateState";

function memoryStorage(lastCheckedAt: number | null, dismissedVersion: string | null = null): StorageLike {
  const values = new Map<string, string>();
  values.set("radsuiteUpdateState", JSON.stringify({ lastCheckedAt, dismissedVersion }));
  return { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) };
}

const update = { version: "0.3.0" } as Update;

describe("performStableUpdateCheck", () => {
  test("skips a non-forced check inside the daily interval without calling the API", async () => {
    const now = 10_000;
    const check = vi.fn(async () => update);
    const result = await performStableUpdateCheck({ force: false, now, storage: memoryStorage(now - UPDATE_CHECK_INTERVAL_MS + 1), check });
    expect(result).toEqual({ status: "skipped" });
    expect(check).not.toHaveBeenCalled();
  });

  test("a forced check bypasses the daily interval", async () => {
    const check = vi.fn(async () => null);
    await expect(performStableUpdateCheck({ force: true, now: 10_000, storage: memoryStorage(9_999), check })).resolves.toEqual({ status: "current" });
    expect(check).toHaveBeenCalledOnce();
  });

  test("returns current when the API reports no update", async () => {
    await expect(performStableUpdateCheck({ force: false, now: 10_000, storage: memoryStorage(null), check: async () => null })).resolves.toEqual({ status: "current" });
  });

  test("returns an available dismissed update when the check is forced", async () => {
    await expect(performStableUpdateCheck({ force: true, now: 10_000, storage: memoryStorage(null, "0.3.0"), check: async () => update })).resolves.toEqual({ status: "available", update });
  });

  test("rejects a failed check without recording a successful timestamp", async () => {
    const storage = memoryStorage(100);
    await expect(performStableUpdateCheck({ force: true, now: 10_000, storage, check: async () => { throw new Error("offline"); } })).rejects.toThrow("offline");
    expect(readUpdateStorageState(storage).lastCheckedAt).toBe(100);
  });
});
