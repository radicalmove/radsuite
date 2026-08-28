import { readFileSync } from "node:fs";
import { describe, expect, test } from "vitest";

const source = readFileSync(new URL("../App.svelte", import.meta.url), "utf8");

describe("application header source contract", () => {
  test("renders the application menu and keeps the separate theme control", () => {
    expect(source).toContain("<ApplicationMenu");
    expect(source).toContain('class="theme-toggle"');
    expect(source).not.toContain('class="version-chip"');
    expect(source).not.toContain('class="status-chip"');
    expect(source).not.toContain('class="help-button"');
    expect(source).not.toContain('>Saved locally<');
    expect(source).not.toContain('>Cloud backup');
  });

  test("keeps update availability, actions, and errors visible", () => {
    expect(source).toContain("{#if availableUpdate}");
    expect(source).toContain("Update now");
    expect(source).toContain("Later");
    expect(source).toContain("{#if updateError}");
  });

  test("uses the stable presentation helper and current-notice duration", () => {
    expect(source).toContain("presentStableUpdateCheck");
    expect(source).toContain("UPDATE_CURRENT_NOTICE_MS");
    expect(source).toContain("currentUpdateMessage");
  });

  test("uses the current release as the fallback display version", () => {
    expect(source).toContain('version: "0.2.8"');
  });
});
