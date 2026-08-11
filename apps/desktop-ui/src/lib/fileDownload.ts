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

  await copyFile(options.sourcePath, destinationPath);
  return { destinationPath };
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

  await writeTextFile(destinationPath, options.contents);
  return { destinationPath };
}
