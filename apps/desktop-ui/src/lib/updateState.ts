import type { StorageLike } from "./storage";

export const UPDATE_CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

const updateStateStorageKey = "radsuiteUpdateState";

export type UpdateStorageState = {
  lastCheckedAt: number | null;
  dismissedVersion: string | null;
};

export const defaultUpdateStorageState: UpdateStorageState = {
  lastCheckedAt: null,
  dismissedVersion: null,
};

function readRaw(storage: StorageLike | null): string | null {
  try {
    return storage?.getItem(updateStateStorageKey) ?? null;
  } catch {
    return null;
  }
}

function writeRaw(storage: StorageLike | null, state: UpdateStorageState): void {
  try {
    storage?.setItem(updateStateStorageKey, JSON.stringify(state));
  } catch {
    // Update reminders are best-effort and must never block the application.
  }
}

export function readUpdateStorageState(storage: StorageLike | null): UpdateStorageState {
  const raw = readRaw(storage);
  if (!raw) return defaultUpdateStorageState;

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") return defaultUpdateStorageState;
    const candidate = parsed as { lastCheckedAt?: unknown; dismissedVersion?: unknown };
    return {
      lastCheckedAt:
        typeof candidate.lastCheckedAt === "number" && Number.isFinite(candidate.lastCheckedAt)
          ? candidate.lastCheckedAt
          : null,
      dismissedVersion:
        typeof candidate.dismissedVersion === "string" && candidate.dismissedVersion.trim()
          ? candidate.dismissedVersion.trim()
          : null,
    };
  } catch {
    return defaultUpdateStorageState;
  }
}

export function writeUpdateStorageState(
  storage: StorageLike | null,
  state: UpdateStorageState,
): void {
  writeRaw(storage, state);
}

export function shouldCheckForUpdate(now: number, lastCheckedAt: number | null): boolean {
  if (!Number.isFinite(now) || lastCheckedAt === null || !Number.isFinite(lastCheckedAt)) {
    return true;
  }
  return now < lastCheckedAt || now - lastCheckedAt >= UPDATE_CHECK_INTERVAL_MS;
}

export function recordUpdateCheck(
  storage: StorageLike | null,
  checkedAt: number,
): UpdateStorageState {
  const current = readUpdateStorageState(storage);
  const next = {
    ...current,
    lastCheckedAt: Number.isFinite(checkedAt) ? checkedAt : current.lastCheckedAt,
  };
  writeUpdateStorageState(storage, next);
  return next;
}

export function dismissUpdateVersion(
  storage: StorageLike | null,
  version: string,
): UpdateStorageState {
  const current = readUpdateStorageState(storage);
  const next = { ...current, dismissedVersion: version.trim() || current.dismissedVersion };
  writeUpdateStorageState(storage, next);
  return next;
}

export function shouldShowUpdateVersion(
  version: string | null | undefined,
  dismissedVersion: string | null,
  allowDismissed = false,
): boolean {
  const normalized = version?.trim();
  const normalizedDismissed = dismissedVersion?.trim().replace(/^v/i, "") ?? null;
  return Boolean(
    normalized &&
      (allowDismissed || normalized.replace(/^v/i, "") !== normalizedDismissed),
  );
}
