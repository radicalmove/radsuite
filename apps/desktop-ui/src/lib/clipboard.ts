export type ClipboardWriter = {
  writeText: (text: string) => Promise<void>;
};

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
