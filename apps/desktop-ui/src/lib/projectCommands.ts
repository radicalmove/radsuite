import { invoke } from "@tauri-apps/api/core";
import type { RadciteProjectSummary } from "../types";

export type CreateRadciteProjectInput = {
  code?: string | null;
  title: string;
};

function trimmedOrNull(value: string | null | undefined): string | null {
  return value?.trim() || null;
}

export function listRadciteProjects(): Promise<RadciteProjectSummary[]> {
  return invoke<RadciteProjectSummary[]>("list_radcite_projects");
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
