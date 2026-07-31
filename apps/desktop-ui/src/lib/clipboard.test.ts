import { describe, expect, test } from "vitest";
import { copyTextToClipboard } from "./clipboard";

describe("copyTextToClipboard", () => {
  test("writes the trimmed search text to the clipboard", async () => {
    const copied: string[] = [];

    await copyTextToClipboard("  Smith 2024  ", {
      writeText: async (value) => {
        copied.push(value);
      },
    });

    expect(copied).toEqual(["Smith 2024"]);
  });

  test("rejects an empty query without touching the clipboard", async () => {
    let copied = false;

    await expect(
      copyTextToClipboard("   ", {
        writeText: async () => {
          copied = true;
        },
      }),
    ).rejects.toThrow("There is no search query to copy.");

    expect(copied).toBe(false);
  });

  test("reports when clipboard access is unavailable", async () => {
    await expect(copyTextToClipboard("Smith 2024", null)).rejects.toThrow(
      "Clipboard access is unavailable.",
    );
  });
});
