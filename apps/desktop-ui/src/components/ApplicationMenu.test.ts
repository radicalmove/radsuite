// @vitest-environment happy-dom

import { flushSync, mount, tick, unmount } from "svelte";
import { afterEach, describe, expect, test, vi } from "vitest";
import ApplicationMenu from "./ApplicationMenu.svelte";

const mounted: ReturnType<typeof mount>[] = [];

function render(overrides: Partial<{
  version: string;
  checkingForUpdate: boolean;
  onCheckForUpdates: () => void;
  onOpenHelp: () => void;
}> = {}) {
  const target = document.createElement("div");
  document.body.append(target);
  const component = mount(ApplicationMenu, {
    target,
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
    hamburger: target.querySelector<HTMLButtonElement>(
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

    expect(hamburger.closest(".application-menu")).not.toBeNull();
    expect(hamburger.classList.contains("application-menu-trigger")).toBe(true);
    expect(hamburger.querySelectorAll(".application-menu-trigger-line")).toHaveLength(3);
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
    expect(menu.classList.contains("application-menu-panel")).toBe(true);
    expect(menu.querySelector(".application-menu-actions")).not.toBeNull();
    expect(menu.querySelectorAll(".application-menu-action")).toHaveLength(2);
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

  test("closes before calling the Help callback without restoring trigger focus", async () => {
    let hamburger: HTMLButtonElement;
    const onOpenHelp = vi.fn(() => {
      expect(hamburger.getAttribute("aria-expanded")).toBe("false");
      expect(document.querySelector('[role="menu"]')).toBeNull();
    });
    ({ hamburger } = render({ onOpenHelp }));
    await click(hamburger);
    const helpAction = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Help",
    )!;
    await click(helpAction);

    expect(onOpenHelp).toHaveBeenCalledOnce();
    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).not.toBe(hamburger);
  });

  test("closes before calling the forced update callback without restoring trigger focus", async () => {
    let hamburger: HTMLButtonElement;
    const onCheckForUpdates = vi.fn(() => {
      expect(hamburger.getAttribute("aria-expanded")).toBe("false");
      expect(document.querySelector('[role="menu"]')).toBeNull();
    });
    ({ hamburger } = render({ onCheckForUpdates }));
    await click(hamburger);
    const updateAction = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Check for updates",
    )!;
    await click(updateAction);

    expect(onCheckForUpdates).toHaveBeenCalledOnce();
    expect(hamburger.getAttribute("aria-expanded")).toBe("false");
    expect(document.activeElement).not.toBe(hamburger);
  });

  test("disables the update action while checking", async () => {
    const { hamburger } = render({ checkingForUpdate: true });
    await click(hamburger);
    const updateAction = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Checking for updates…",
    ) as HTMLButtonElement;

    expect(updateAction.disabled).toBe(true);
  });

  test("focuses Help when the update action is disabled", async () => {
    const { hamburger } = render({ checkingForUpdate: true });
    await click(hamburger);
    const helpAction = Array.from(document.querySelectorAll("button")).find(
      (button) => button.textContent?.trim() === "Help",
    );

    expect(document.activeElement).toBe(helpAction);
  });

  test("uses a distinct menu id for each instance", async () => {
    const first = render();
    const second = render();

    expect(first.hamburger.getAttribute("aria-controls")).not.toBe(
      second.hamburger.getAttribute("aria-controls"),
    );
  });

  test("removes window listeners when the menu closes", async () => {
    const removeListener = vi.spyOn(window, "removeEventListener");
    const { hamburger } = render();
    await click(hamburger);
    await click(hamburger);

    expect(removeListener).toHaveBeenCalledWith("click", expect.any(Function));
    expect(removeListener).toHaveBeenCalledWith("keydown", expect.any(Function));
    removeListener.mockRestore();
  });

  test("renders the application version", async () => {
    const { hamburger } = render({ version: "1.4.2" });
    await click(hamburger);

    const footer = document.querySelector('[role="menu"] footer');
    expect(footer?.classList.contains("application-menu-version")).toBe(true);
    expect(footer?.textContent).toContain("Version 1.4.2");
    expect(footer?.getAttribute("role")).toBe("presentation");
  });
});
