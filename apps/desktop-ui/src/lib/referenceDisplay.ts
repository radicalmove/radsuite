import type { CourseReferenceSummary } from "../types";
import type { StorageLike } from "./storage";

const referenceDisplayStorageKey = "radciteReferenceDisplayPreferences";

export function filterCourseReferencesForDisplay(
  references: CourseReferenceSummary[],
  hideApaReady: boolean,
): CourseReferenceSummary[] {
  return hideApaReady
    ? references.filter((reference) => reference.validation_status !== "valid")
    : references;
}

export function readHideApaReady(storage: StorageLike | null): boolean {
  if (!storage) {
    return false;
  }

  try {
    const raw = storage.getItem(referenceDisplayStorageKey);
    if (!raw) {
      return false;
    }

    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return false;
    }

    const candidate = parsed as { version?: unknown; hideApaReady?: unknown };
    return candidate.version === 1 && candidate.hideApaReady === true;
  } catch {
    return false;
  }
}

export function writeHideApaReady(storage: StorageLike | null, hideApaReady: boolean): void {
  if (!storage) {
    return;
  }

  try {
    storage.setItem(
      referenceDisplayStorageKey,
      JSON.stringify({ version: 1, hideApaReady }),
    );
  } catch {
    // Filtering remains available when local preference storage is blocked.
  }
}
