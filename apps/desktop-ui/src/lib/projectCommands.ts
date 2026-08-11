import { invoke } from "@tauri-apps/api/core";
import type { LegacyRadciteImportResult, RadciteProjectSummary } from "../types";

export type CreateRadciteProjectInput = {
  code?: string | null;
  title: string;
};

export type UpdateRadciteProjectInput = {
  code?: string | null;
  title: string;
  description?: string | null;
  structureMode?: "modules" | "weeks";
};

export type ProjectMutationInput = {
  project_id: string;
};

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function listRadciteProjects(): Promise<RadciteProjectSummary[]> {
  return invoke<RadciteProjectSummary[]>("list_radcite_projects");
}

export function importLegacyRadciteDatabase(path: string): Promise<LegacyRadciteImportResult> {
  const normalizedPath = path.trim();
  if (!normalizedPath) {
    throw new Error("legacy RADcite database path is required");
  }
  return invoke<LegacyRadciteImportResult>("import_legacy_radcite_database", {
    request: { path: normalizedPath },
  });
}

export function createRadciteProject(
  input: CreateRadciteProjectInput,
): Promise<RadciteProjectSummary> {
  return invoke<RadciteProjectSummary>("create_radcite_project", {
    request: {
      code: trimmedOrNull(input.code),
      title: input.title.trim(),
    },
  });
}

export function updateRadciteProject(
  projectId: string,
  input: UpdateRadciteProjectInput,
): Promise<RadciteProjectSummary> {
  return invoke<RadciteProjectSummary>("update_radcite_project", {
    request: {
      ...projectMutationRequest(projectId),
      code: trimmedOrNull(input.code),
      title: input.title.trim(),
      description: trimmedOrNull(input.description),
      structure_mode: input.structureMode ?? "modules",
    },
  });
}

function projectMutationRequest(projectId: string): ProjectMutationInput {
  const normalized = projectId.trim();
  if (!normalized) {
    throw new Error("project ID is required");
  }
  return { project_id: normalized };
}

export function archiveRadciteProject(projectId: string): Promise<RadciteProjectSummary> {
  return invoke<RadciteProjectSummary>("archive_radcite_project", {
    request: projectMutationRequest(projectId),
  });
}

export function restoreRadciteProject(projectId: string): Promise<RadciteProjectSummary> {
  return invoke<RadciteProjectSummary>("restore_radcite_project", {
    request: projectMutationRequest(projectId),
  });
}
