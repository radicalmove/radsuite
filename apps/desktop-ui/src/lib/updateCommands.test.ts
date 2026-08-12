import { describe, expect, test, vi } from "vitest";
import { installUpdate, type UpdaterApi } from "./updateCommands";
import type { Update } from "@tauri-apps/plugin-updater";

describe("stable update installation", () => {
  test("downloads, reports progress, and relaunches only after installation", async () => {
    const progress = vi.fn();
    const order: string[] = [];
    const api: UpdaterApi = {
      check: vi.fn(),
      downloadAndInstall: vi.fn(async (_update, onProgress) => {
        order.push("download");
        onProgress?.({ downloadedBytes: 1024, totalBytes: 2048 });
      }),
      relaunch: vi.fn(async () => {
        order.push("relaunch");
      }),
    };

    await installUpdate({ version: "0.2.3" } as Update, progress, api);

    expect(progress).toHaveBeenCalledWith({ downloadedBytes: 1024, totalBytes: 2048 });
    expect(order).toEqual(["download", "relaunch"]);
  });

  test("does not relaunch when the download or install fails", async () => {
    const api: UpdaterApi = {
      check: vi.fn(),
      downloadAndInstall: vi.fn(async () => {
        throw new Error("disk full");
      }),
      relaunch: vi.fn(),
    };

    await expect(installUpdate({ version: "0.2.3" } as Update, undefined, api)).rejects.toThrow("disk full");
    expect(api.relaunch).not.toHaveBeenCalled();
  });
});
