// @vitest-environment happy-dom

// Vitest resolves Svelte's public entry to its server build unless the whole Vite app is
// configured for browser-only tests. Import the client runtime directly for this Happy DOM test.
// @ts-expect-error Svelte does not publish declarations for its concrete client entry point.
import { flushSync, mount, tick, unmount } from "../../node_modules/svelte/src/index-client.js";
import { afterEach, describe, expect, test, vi } from "vitest";
import ApplicationMenu from "./ApplicationMenu.svelte";

const mounted: ReturnType<typeof mount>[] = [];

function render(overrides: Partial<{
  version: string;
  checkingForUpdate: boolean;
  onCheckForUpdates: () => void;
  onOpenHelp: () => void;
}> = {}) {
  const component = mount(ApplicationMenu, {
    target: document.body,
    props: {
      version: "0.2.7",
      checkingForUpdate: false,
      onCheckForUpdates: vi.fn(),
      onOpenHelp: vi.fn(),
      ...overrides,
    },
  });
  mounted.push(component);
  return {
    hamburger: document.querySelector<HTMLButtonElement>(
      'button[aria-label="Open application menu"]',
    )!,
  };
}

async function click(element: HTMLElement) {
  flushSync(() => element.click());
  await tick();
  flushSync();
}

afterEach(async () => {
  while (mounted.length) await unmount(mounted.pop()!);
  document.body.innerHTML = "";
});

describe("ApplicationMenu", () => {
  test("starts closed with menu button accessibility attributes", () => {
    const { hamburger } = render();

    expect(hamburger.getAttribute("aria-haspopup")).toBe("menu");
    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
    expect(hamburger.getAttribute("aria-controls")).toBeTruthy();
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });

  test("opens the menu and focuses the first action", async () => {
    const { hamburger } = render();
    await click(hamburger);

    const menu = document.querySelector<HTMLElement>('[role="menu"]')!;
    const firstAction = menu.querySelector<HTMLButtonElement>("button")!;
    expect(hamburger.getAttribute("aria-expanded")).toBe("true");
    expect(menu.id).toBe(hamburger.getAttribute("aria-controls"));
    expect(firstAction.textContent).toContain("Check for updates");
    expect(document.activeElement).toBe(firstAction);
  });

  test("closes when the hamburger is toggled", async () => {
    const { hamburger } = render();
    await click(hamburger);
    await click(hamburger);

    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
    expect(document.querySelector('[role="menu"]')).toBeNull();
  });

  test("closes on an outside click", async () => {
    const { hamburger } = render();
    await click(hamburger);
    await click(document.body);

    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
  });

  test("closes on Escape and returns focus to the hamburger", async () => {
    const { hamburger } = render();
    await click(hamburger);
    flushSync(() => window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" })));
    await tick();
    flushSync();

    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).toBe(hamburger);
  });

  test("calls the Help callback", async () => {
    const onOpenHelp = vi.fn();
    const { hamburger } = render({ onOpenHelp });
    await click(hamburger);
    await click(Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Help",
    )!);

    expect(onOpenHelp).toHaveBeenCalledOnce();
  });

  test("calls the forced update callback", async () => {
    const onCheckForUpdates = vi.fn();
    const { hamburger } = render({ onCheckForUpdates });
    await click(hamburger);
    await click(Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Check for updates",
    )!);

    expect(onCheckForUpdates).toHaveBeenCalledOnce();
  });

  test("disables the update action while checking", async () => {
    const { hamburger } = render({ checkingForUpdate: true });
    await click(hamburger);
    const updateAction = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Checking for updates…",
    ) as HTMLButtonElement;

    expect(updateAction.disabled).toBe(true);
  });

  test("renders the application version", async () => {
    const { hamburger } = render({ version: "1.4.2" });
    await click(hamburger);

    expect(document.querySelector('[role="menu"]')?.textContent).toContain("Version 1.4.2");
  });
});
