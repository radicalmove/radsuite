import type { Update } from "@tauri-apps/plugin-updater";
import type { StorageLike } from "./storage";
import {
  readUpdateStorageState,
  recordUpdateCheck,
  shouldCheckForUpdate,
  shouldShowUpdateVersion,
} from "./updateState";

export type StableUpdateCheckResult =
  | { status: "skipped" }
  | { status: "current" }
  | { status: "available"; update: Update };

type StableUpdateCheckOptions = {
  force: boolean;
  now: number;
  storage: StorageLike | null;
  check: () => Promise<Update | null>;
};

export async function performStableUpdateCheck({
  force,
  now,
  storage,
  check,
}: StableUpdateCheckOptions): Promise<StableUpdateCheckResult> {
  const state = readUpdateStorageState(storage);
  if (!force && !shouldCheckForUpdate(now, state.lastCheckedAt)) {
    return { status: "skipped" };
  }

  const update = await check();
  recordUpdateCheck(storage, now);
  if (update && shouldShowUpdateVersion(update.version, state.dismissedVersion, force)) {
    return { status: "available", update };
  }
  return { status: "current" };
}
