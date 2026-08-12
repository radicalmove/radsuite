import { invoke } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, test, vi } from "vitest";
import { setupLocalRuntimes } from "./runtimeSetup";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("local runtime setup", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
  });

  test("starts the bundled local runtime setup", async () => {
    vi.mocked(invoke).mockResolvedValue("Local runtimes are ready.");

    await expect(setupLocalRuntimes()).resolves.toBe("Local runtimes are ready.");
    expect(invoke).toHaveBeenCalledWith("setup_local_runtimes");
  });
});
