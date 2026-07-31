export type StorageLike = Pick<Storage, "getItem" | "setItem">;

export type ProjectNavStorageState = {
  version: 1;
  expandedProjectIds: string[];
  archivedSectionOpen: boolean;
};

export const defaultProjectNavStorageState: ProjectNavStorageState = {
  version: 1,
  expandedProjectIds: [],
  archivedSectionOpen: false,
};

const projectNavStorageKey = "radciteProjectNavState";
const themeStorageKey = "radciteTheme";

function readRaw(storage: StorageLike | null, key: string): string | null {
  if (!storage) {
    return null;
  }

  try {
    return storage.getItem(key);
  } catch {
    return null;
  }
}

function writeRaw(storage: StorageLike | null, key: string, value: string): void {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(key, value);
  } catch {
    // Local storage is a convenience; navigation must still work when it is blocked.
  }
}

export function browserStorage(): StorageLike | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage;
  } catch {
    return null;
  }
}

export function readThemeStorage(storage: StorageLike | null): "light" | "dark" {
  return readRaw(storage, themeStorageKey) === "dark" ? "dark" : "light";
}

export function writeThemeStorage(storage: StorageLike | null, theme: "light" | "dark"): void {
  writeRaw(storage, themeStorageKey, theme);
}

export function readProjectNavStorageState(
  storage: StorageLike | null,
): ProjectNavStorageState {
  const raw = readRaw(storage, projectNavStorageKey);
  if (!raw) {
    return defaultProjectNavStorageState;
  }

  try {
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object") {
      return defaultProjectNavStorageState;
    }

    const candidate = parsed as {
      version?: unknown;
      expandedProjectIds?: unknown;
      archivedSectionOpen?: unknown;
    };
    if (candidate.version !== 1 || !Array.isArray(candidate.expandedProjectIds)) {
      return defaultProjectNavStorageState;
    }

    return {
      version: 1,
      expandedProjectIds: candidate.expandedProjectIds.filter(
        (value): value is string => typeof value === "string" && value.length > 0,
      ),
      archivedSectionOpen: candidate.archivedSectionOpen === true,
    };
  } catch {
    return defaultProjectNavStorageState;
  }
}

export function writeProjectNavStorageState(
  storage: StorageLike | null,
  state: ProjectNavStorageState,
): void {
  writeRaw(storage, projectNavStorageKey, JSON.stringify(state));
}
