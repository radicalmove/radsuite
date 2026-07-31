import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import {
  archiveRadciteDocument,
  listRadciteArchive,
  restoreRadciteArchiveItem,
} from "./archiveCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("archive commands", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("lists archive items for the selected project", async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    await expect(listRadciteArchive(" project-1 ")).resolves.toEqual([]);

    expect(invoke).toHaveBeenCalledWith("list_radcite_archive", {
      request: { project_id: "project-1" },
    });
  });

  test("restores the requested archive item kind and id", async () => {
    vi.mocked(invoke).mockResolvedValue([]);

    await restoreRadciteArchiveItem({
      project_id: "project-1",
      kind: "module",
      item_id: "module-1",
    });

    expect(invoke).toHaveBeenCalledWith("restore_radcite_archive_item", {
      request: {
        project_id: "project-1",
        kind: "module",
        item_id: "module-1",
      },
    });
  });

  test("archives a saved document by id", async () => {
    vi.mocked(invoke).mockResolvedValue({});

    await archiveRadciteDocument("document-1", "project-1");

    expect(invoke).toHaveBeenCalledWith("archive_radcite_document", {
      request: { project_id: "project-1", document_id: "document-1" },
    });
  });
});
