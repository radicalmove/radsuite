import type { Update } from "@tauri-apps/plugin-updater";
import type { StableUpdateCheckResult } from "./updateCheck";

export const UPDATE_CURRENT_NOTICE_MS = 5_000;

export type StableUpdatePresentation = {
  availableUpdate: Update | null;
  currentMessage: string | null;
};

export function presentStableUpdateCheck(
  result: StableUpdateCheckResult,
  manual: boolean,
  installedVersion: string,
): StableUpdatePresentation {
  if (result.kind === "available") {
    return { availableUpdate: result.update, currentMessage: null };
  }
  if (result.kind === "current" && manual) {
    return {
      availableUpdate: null,
      currentMessage: `RADsuite ${installedVersion} is up to date.`,
    };
  }
  return { availableUpdate: null, currentMessage: null };
}

export function formatUpdateCheckError(error: unknown): string {
  const message = error instanceof Error ? error.message : String(error);
  return message.startsWith("Could not check for updates")
    ? message
    : `Could not check for updates: ${message}`;
}
