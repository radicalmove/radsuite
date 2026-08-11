import { describe, expect, test } from "vitest";
import { copyHtmlToClipboard, copyTextToClipboard } from "./clipboard";

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

describe("copyHtmlToClipboard", () => {
  test("writes rich HTML and a readable plain-text fallback", async () => {
    const payloads: Array<Record<string, Blob>> = [];
    const writes: unknown[][] = [];

    await copyHtmlToClipboard(
      " <p><strong>Required</strong> &amp; optional</p> ",
      {
        writeText: async () => undefined,
        write: async (items) => {
          writes.push(items);
        },
      },
      (data) => {
        payloads.push(data);
        return data as unknown as ClipboardItem;
      },
    );

    expect(writes).toHaveLength(1);
    expect(payloads).toHaveLength(1);
    expect(Object.keys(payloads[0])).toEqual(["text/html", "text/plain"]);
    expect(await payloads[0]["text/html"].text()).toBe(
      "<p><strong>Required</strong> &amp; optional</p>",
    );
    expect(await payloads[0]["text/plain"].text()).toBe("Required & optional");
  });

  test("falls back to copying HTML source when rich clipboard is unavailable", async () => {
    const copied: string[] = [];

    await copyHtmlToClipboard(
      " <p>Module readings</p> ",
      {
        writeText: async (value) => {
          copied.push(value);
        },
      },
      null,
    );

    expect(copied).toEqual(["<p>Module readings</p>"]);
  });
});
