export type ClipboardWriter = {
  writeText: (text: string) => Promise<void>;
  write?: (items: ClipboardItem[]) => Promise<void>;
};

export type ClipboardItemFactory = (data: Record<string, Blob>) => ClipboardItem;

function browserClipboard(): ClipboardWriter | null {
  if (typeof navigator === "undefined" || !navigator.clipboard) {
    return null;
  }

  return navigator.clipboard;
}

export async function copyTextToClipboard(
  text: string,
  clipboard: ClipboardWriter | null = browserClipboard(),
): Promise<void> {
  const trimmedText = text.trim();
  if (!trimmedText) {
    throw new Error("There is no search query to copy.");
  }
  if (!clipboard) {
    throw new Error("Clipboard access is unavailable.");
  }

  await clipboard.writeText(trimmedText);
}

function browserClipboardItemFactory(): ClipboardItemFactory | null {
  if (typeof ClipboardItem === "undefined") {
    return null;
  }

  return (data) => new ClipboardItem(data);
}

function htmlToPlainText(html: string): string {
  if (typeof document !== "undefined") {
    const container = document.createElement("div");
    container.innerHTML = html;
    return container.textContent?.replace(/\s+/g, " ").trim() ?? "";
  }

  return html
    .replace(/<[^>]*>/g, " ")
    .replace(/&nbsp;/gi, " ")
    .replace(/&amp;/gi, "&")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&quot;/gi, '"')
    .replace(/&#39;/gi, "'")
    .replace(/\s+/g, " ")
    .trim();
}

export async function copyHtmlToClipboard(
  html: string,
  clipboard: ClipboardWriter | null = browserClipboard(),
  itemFactory: ClipboardItemFactory | null = browserClipboardItemFactory(),
): Promise<void> {
  const trimmedHtml = html.trim();
  if (!trimmedHtml) {
    throw new Error("There is no HTML export to copy.");
  }
  if (!clipboard) {
    throw new Error("Clipboard access is unavailable.");
  }

  if (clipboard.write && itemFactory) {
    const item = itemFactory({
      "text/html": new Blob([trimmedHtml], { type: "text/html" }),
      "text/plain": new Blob([htmlToPlainText(trimmedHtml)], { type: "text/plain" }),
    });
    await clipboard.write([item]);
    return;
  }

  await clipboard.writeText(trimmedHtml);
}
