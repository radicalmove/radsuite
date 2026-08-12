import { invoke } from "@tauri-apps/api/core";

export function setupLocalRuntimes(): Promise<string> {
  return invoke<string>("setup_local_runtimes");
}
