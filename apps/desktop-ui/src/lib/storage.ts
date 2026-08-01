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
const radtTsPreferencesStorageKey = "radsuiteRadtTsPreferences";

export type RadtTsProjectPreferences = {
  voice?: {
    referenceAudioPath?: string;
    referenceText?: string;
    quality?: "fast" | "high";
    chunkMode?: "single" | "sentence";
    pauseMinSeconds?: number;
    pauseMaxSeconds?: number;
    pauseSeed?: string;
    maxNewTokens?: number;
    outputFormat?: "mp3" | "wav";
    outputName?: string;
  };
  transcription?: {
    audioPath?: string;
    name?: string;
    model?: string;
    language?: string;
    beamSize?: number;
  };
  clip?: {
    audioPath?: string;
    segmentsJsonPath?: string;
    outputName?: string;
    boundaryMode?: "phrases" | "times";
    startPhrase?: string;
    endPhrase?: string;
    startTime?: number;
    endTime?: number;
    verificationMode?: "strict" | "lenient";
    outputFormat?: "mp3" | "wav";
  };
};

type RadtTsPreferencesStorageState = {
  version: 1;
  projects: Record<string, RadtTsProjectPreferences>;
};

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

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

export function readRadtTsProjectPreferences(
  storage: StorageLike | null,
  projectId: string | null,
): RadtTsProjectPreferences {
  if (!projectId) return {};
  const raw = readRaw(storage, radtTsPreferencesStorageKey);
  if (!raw) return {};
  try {
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed) || parsed.version !== 1 || !isRecord(parsed.projects)) return {};
    const preference = parsed.projects[projectId];
    return isRecord(preference) ? (preference as RadtTsProjectPreferences) : {};
  } catch {
    return {};
  }
}

export function writeRadtTsProjectPreferences(
  storage: StorageLike | null,
  projectId: string | null,
  preferences: RadtTsProjectPreferences,
): void {
  if (!projectId) return;
  const state: RadtTsPreferencesStorageState = {
    version: 1,
    projects: {},
  };
  const raw = readRaw(storage, radtTsPreferencesStorageKey);
  if (raw) {
    try {
      const parsed: unknown = JSON.parse(raw);
      if (isRecord(parsed) && parsed.version === 1 && isRecord(parsed.projects)) {
        Object.assign(state.projects, parsed.projects);
      }
    } catch {
      // Replace malformed preferences with a clean state.
    }
  }
  state.projects[projectId] = preferences;
  writeRaw(storage, radtTsPreferencesStorageKey, JSON.stringify(state));
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
