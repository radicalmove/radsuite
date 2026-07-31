import { invoke } from "@tauri-apps/api/core";
import type {
  RadciteArchiveItem,
  RadciteArchiveItemKind,
  SavedRadciteReviewSummary,
} from "../types";

export function listRadciteArchive(
  projectId?: string | null,
): Promise<RadciteArchiveItem[]> {
  return invoke<RadciteArchiveItem[]>("list_radcite_archive", {
    request: {
      project_id: projectId?.trim() || null,
    },
  });
}

export function restoreRadciteArchiveItem(input: {
  project_id?: string | null;
  kind: RadciteArchiveItemKind;
  item_id: string;
}): Promise<RadciteArchiveItem[]> {
  return invoke<RadciteArchiveItem[]>("restore_radcite_archive_item", {
    request: {
      project_id: input.project_id?.trim() || null,
      kind: input.kind,
      item_id: input.item_id,
    },
  });
}

export function archiveRadciteDocument(
  documentId: string,
  projectId?: string | null,
): Promise<SavedRadciteReviewSummary> {
  return invoke<SavedRadciteReviewSummary>("archive_radcite_document", {
    request: {
      project_id: projectId?.trim() || null,
      document_id: documentId,
    },
  });
}
