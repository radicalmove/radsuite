import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateProgress = {
  downloadedBytes: number;
  totalBytes: number | null;
};

export type UpdaterApi = {
  check: () => Promise<Update | null>;
  downloadAndInstall: (
    update: Update,
    onProgress?: (progress: UpdateProgress) => void,
  ) => Promise<void>;
  relaunch: () => Promise<void>;
};

export const updaterApi: UpdaterApi = {
  check,
  async downloadAndInstall(update, onProgress) {
    let downloadedBytes = 0;
    let totalBytes: number | null = null;
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        downloadedBytes = 0;
        totalBytes = event.data.contentLength ?? null;
      } else if (event.event === "Progress") {
        downloadedBytes += event.data.chunkLength;
        onProgress?.({ downloadedBytes, totalBytes });
      } else if (event.event === "Finished") {
        onProgress?.({ downloadedBytes, totalBytes });
      }
    });
  },
  relaunch,
};

export async function installUpdate(
  update: Update,
  onProgress?: (progress: UpdateProgress) => void,
  api: UpdaterApi = updaterApi,
): Promise<void> {
  await api.downloadAndInstall(update, onProgress);
  await api.relaunch();
}
