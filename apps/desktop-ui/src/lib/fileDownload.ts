export type LocalArtifactSaveOptions = {
  sourcePath: string;
  defaultPath: string;
  filterName: string;
  extensions: string[];
};

export type LocalTextSaveOptions = {
  contents: string;
  defaultPath: string;
  filterName: string;
  extensions: string[];
};

export type SaveDialogOptions = {
  defaultPath: string;
  filters: Array<{ name: string; extensions: string[] }>;
};

export type ChooseDestination = (
  options: SaveDialogOptions,
) => Promise<string | null>;

export type CopyLocalFile = (sourcePath: string, destinationPath: string) => Promise<void>;
export type WriteLocalTextFile = (destinationPath: string, contents: string) => Promise<void>;

export function filenameFromPath(sourcePath: string, fallback: string): string {
  const filename = sourcePath.replaceAll("\\", "/").split("/").pop()?.trim();
  return filename && filename !== "." && filename !== ".." ? filename : fallback;
}

function pathExtension(sourcePath: string): string {
  const filename = filenameFromPath(sourcePath, "");
  const dotIndex = filename.lastIndexOf(".");
  return dotIndex > 0 ? filename.slice(dotIndex + 1).toLowerCase() : "";
}

function replacePathExtension(sourcePath: string, extension: string): string {
  const separatorIndex = Math.max(sourcePath.lastIndexOf("/"), sourcePath.lastIndexOf("\\"));
  const dotIndex = sourcePath.lastIndexOf(".");
  const normalizedExtension = extension.replace(/^\.+/, "");
  if (dotIndex > separatorIndex && dotIndex > separatorIndex + 1) {
    return `${sourcePath.slice(0, dotIndex)}.${normalizedExtension}`;
  }
  return `${sourcePath}.${normalizedExtension}`;
}

function normalizeArtifactDestination(sourcePath: string, extensions: string[]): string {
  const allowedExtensions = extensions
    .map((extension) => extension.replace(/^\.+/, "").toLowerCase())
    .filter(Boolean);
  if (allowedExtensions.length === 0 || allowedExtensions.includes(pathExtension(sourcePath))) {
    return sourcePath;
  }
  return replacePathExtension(sourcePath, allowedExtensions[0]);
}

export async function saveLocalArtifact(
  options: LocalArtifactSaveOptions,
  chooseDestination: ChooseDestination,
  copyFile: CopyLocalFile,
): Promise<{ destinationPath: string } | null> {
  const destinationPath = await chooseDestination({
    defaultPath: options.defaultPath,
    filters: [{ name: options.filterName, extensions: options.extensions }],
  });

  if (!destinationPath) return null;

  const normalizedDestinationPath = normalizeArtifactDestination(destinationPath, options.extensions);
  await copyFile(options.sourcePath, normalizedDestinationPath);
  return { destinationPath: normalizedDestinationPath };
}

export async function saveLocalTextArtifact(
  options: LocalTextSaveOptions,
  chooseDestination: ChooseDestination,
  writeTextFile: WriteLocalTextFile,
): Promise<{ destinationPath: string } | null> {
  const destinationPath = await chooseDestination({
    defaultPath: options.defaultPath,
    filters: [{ name: options.filterName, extensions: options.extensions }],
  });

  if (!destinationPath) return null;

  const normalizedDestinationPath = normalizeArtifactDestination(destinationPath, options.extensions);
  await writeTextFile(normalizedDestinationPath, options.contents);
  return { destinationPath: normalizedDestinationPath };
}
