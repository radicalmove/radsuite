// @vitest-environment happy-dom

import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";
import RadtTsWorkspace from "./RadtTsWorkspace.svelte";

const invoke = vi.hoisted(() => vi.fn(async (command: string) => {
  if (command === "get_radt_ts_capabilities") {
    return {
      available: true,
      executable: "/tmp/radtts",
      detail: "Ready",
      supports_builtin_voices: false,
      builtin_voices: [],
    };
  }
  if (command === "list_radt_ts_outputs") return { outputs: [] };
  if (command === "get_project_presenter_image") return null;
  throw new Error(`Unexpected command: ${command}`);
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
  convertFileSrc: (path: string) => path,
}));

let mounted: ReturnType<typeof mount> | null = null;

afterEach(async () => {
  if (mounted) await unmount(mounted);
  mounted = null;
  document.body.innerHTML = "";
  invoke.mockClear();
});

describe("RadtTsWorkspace", () => {
  test("checks local voice capability once when a project opens", async () => {
    const target = document.createElement("div");
    document.body.append(target);
    mounted = mount(RadtTsWorkspace, {
      target,
      props: { selectedProjectId: "test-project" },
    });

    flushSync();
    await new Promise((resolve) => setTimeout(resolve, 50));
    await tick();

    expect(invoke.mock.calls.filter(([command]) => command === "get_radt_ts_capabilities"))
      .toHaveLength(1);
    expect(target.textContent).not.toContain("Checking local voice generation support...");

    const script = target.querySelector<HTMLTextAreaElement>('textarea[placeholder="Paste the narration script here."]')!;
    const reference = target.querySelector<HTMLInputElement>('input[placeholder="Choose a clear voice sample"]')!;
    const permission = target.querySelector<HTMLInputElement>('.radtts-check input[type="checkbox"]')!;
    flushSync(() => {
      script.value = "A short introduction.";
      script.dispatchEvent(new Event("input", { bubbles: true }));
      reference.value = "/tmp/reference.wav";
      reference.dispatchEvent(new Event("input", { bubbles: true }));
      permission.checked = true;
      permission.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await tick();
    expect(target.querySelector<HTMLButtonElement>(".radtts-process-button")!.disabled).toBe(false);
  });
});
