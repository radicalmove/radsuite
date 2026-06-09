import type { AddRadciteModuleInput } from "./readingCommands";
import type { CourseModuleSummary, ModuleReadingImportCandidate } from "../types";

export type ImportModuleDraft = Required<
  Pick<AddRadciteModuleInput, "title" | "code" | "order_index" | "description">
>;

type CandidateWithModuleSelection = ModuleReadingImportCandidate & {
  module_id: string;
  selected: boolean;
};

export function inferModuleDraftForImport(
  candidates: ModuleReadingImportCandidate[],
  sourcePath: string,
): ImportModuleDraft | null {
  const candidateDraft = inferModuleDraftFromCandidates(candidates);
  if (candidateDraft) {
    return candidateDraft;
  }

  return inferModuleDraftFromPath(sourcePath);
}

export function moduleMatchesImportDraft(
  module: CourseModuleSummary,
  draft: ImportModuleDraft,
): boolean {
  if (draft.order_index !== null && module.order_index === draft.order_index) {
    return true;
  }

  return normalizedLabel(module.title) === normalizedLabel(draft.title);
}

export function applyAutoModuleToCandidates<T extends CandidateWithModuleSelection>(
  candidates: T[],
  moduleId: string,
): T[] {
  return candidates.map((candidate) =>
    candidate.selected && !candidate.module_id ? { ...candidate, module_id: moduleId } : candidate,
  );
}

function inferModuleDraftFromCandidates(
  candidates: ModuleReadingImportCandidate[],
): ImportModuleDraft | null {
  const candidatesWithModule = candidates.filter(
    (candidate) => candidate.module_order !== null || candidate.module_title,
  );
  if (!candidatesWithModule.length) {
    return null;
  }

  const first = candidatesWithModule[0];
  const consistent = candidatesWithModule.every(
    (candidate) =>
      candidate.module_order === first.module_order &&
      normalizedLabel(candidate.module_title ?? "") === normalizedLabel(first.module_title ?? ""),
  );
  if (!consistent) {
    return null;
  }

  const title = first.module_title?.trim() || moduleTitleFromOrder(first.module_order);
  if (!title) {
    return null;
  }

  return {
    title,
    code: null,
    order_index: first.module_order,
    description: null,
  };
}

function inferModuleDraftFromPath(sourcePath: string): ImportModuleDraft | null {
  const filename = sourcePath.split(/[\\/]/).at(-1) ?? sourcePath;
  const match = filename.match(/\bmodule\s+(\d{1,2})\b/i);
  if (!match) {
    return null;
  }

  const order = Number(match[1]);
  if (!Number.isInteger(order) || order < 1) {
    return null;
  }

  return {
    title: moduleTitleFromOrder(order) ?? `Module ${order}`,
    code: null,
    order_index: order,
    description: null,
  };
}

function moduleTitleFromOrder(order: number | null): string | null {
  return order === null ? null : `Module ${order}`;
}

function normalizedLabel(value: string): string {
  return value.trim().toLowerCase();
}
