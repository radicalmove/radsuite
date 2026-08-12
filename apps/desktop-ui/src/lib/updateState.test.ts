import { describe, expect, test } from "vitest";
import {
  UPDATE_CHECK_INTERVAL_MS,
  defaultUpdateStorageState,
  dismissUpdateVersion,
  readUpdateStorageState,
  recordUpdateCheck,
  shouldCheckForUpdate,
  shouldShowUpdateVersion,
  type UpdateStorageState,
} from "./updateState";
import type { StorageLike } from "./storage";

function memoryStorage(initial: Record<string, string> = {}): StorageLike {
  const values = new Map(Object.entries(initial));
  return {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  };
}

describe("stable update scheduling", () => {
  test("checks on first launch and exactly at the daily interval", () => {
    expect(shouldCheckForUpdate(Date.now(), null)).toBe(true);
    expect(shouldCheckForUpdate(1000, 1000 + UPDATE_CHECK_INTERVAL_MS)).toBe(true);
    expect(shouldCheckForUpdate(1000 + UPDATE_CHECK_INTERVAL_MS - 1, 1000)).toBe(false);
    expect(shouldCheckForUpdate(1000 + UPDATE_CHECK_INTERVAL_MS, 1000)).toBe(true);
  });

  test("retries when the system clock moves backwards", () => {
    expect(shouldCheckForUpdate(9_000, 10_000)).toBe(true);
    expect(shouldCheckForUpdate(Number.NaN, 10_000)).toBe(true);
  });

  test("reads malformed storage as a clean state", () => {
    expect(readUpdateStorageState(memoryStorage({ radsuiteUpdateState: "{bad" }))).toEqual(
      defaultUpdateStorageState,
    );
    expect(
      readUpdateStorageState(
        memoryStorage({
          radsuiteUpdateState: JSON.stringify({ lastCheckedAt: "yesterday", dismissedVersion: 3 }),
        }),
      ),
    ).toEqual(defaultUpdateStorageState);
  });

  test("records checks and allows a dismissed version to be offered later", () => {
    const storage = memoryStorage();
    expect(recordUpdateCheck(storage, 1234)).toEqual({
      lastCheckedAt: 1234,
      dismissedVersion: null,
    });
    expect(dismissUpdateVersion(storage, "v0.2.2")).toEqual({
      lastCheckedAt: 1234,
      dismissedVersion: "v0.2.2",
    });
    expect(shouldShowUpdateVersion("0.2.2", "v0.2.2")).toBe(false);
    expect(shouldShowUpdateVersion("v0.2.2", "v0.2.2")).toBe(false);
    expect(readUpdateStorageState(storage)).toEqual<UpdateStorageState>({
      lastCheckedAt: 1234,
      dismissedVersion: "v0.2.2",
    });
  });
});
