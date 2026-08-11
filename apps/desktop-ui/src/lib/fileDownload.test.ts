import { describe, expect, it, vi } from "vitest";
import { filenameFromPath, saveLocalArtifact, saveLocalTextArtifact } from "./fileDownload";

describe("filenameFromPath", () => {
  it("extracts a filename from local or Windows-style paths", () => {
    expect(filenameFromPath("/private/app/outputs/lesson.mp3", "fallback.mp3")).toBe("lesson.mp3");
    expect(filenameFromPath("C:\\outputs\\captions.vtt", "fallback.vtt")).toBe("captions.vtt");
  });

  it("uses the fallback when the path has no filename", () => {
    expect(filenameFromPath("/", "fallback.txt")).toBe("fallback.txt");
  });
});

describe("saveLocalArtifact", () => {
  it("copies the selected artifact to the destination returned by the save dialog", async () => {
    const chooseDestination = vi.fn().mockResolvedValue("/Users/tester/Desktop/lesson.mp3");
    const copyFile = vi.fn().mockResolvedValue(undefined);

    await expect(
      saveLocalArtifact(
        {
          sourcePath: "/private/app/outputs/lesson.mp3",
          defaultPath: "lesson.mp3",
          filterName: "Audio",
          extensions: ["mp3"],
        },
        chooseDestination,
        copyFile,
      ),
    ).resolves.toEqual({ destinationPath: "/Users/tester/Desktop/lesson.mp3" });

    expect(chooseDestination).toHaveBeenCalledWith({
      defaultPath: "lesson.mp3",
      filters: [{ name: "Audio", extensions: ["mp3"] }],
    });
    expect(copyFile).toHaveBeenCalledWith(
      "/private/app/outputs/lesson.mp3",
      "/Users/tester/Desktop/lesson.mp3",
    );
  });

  it("does not copy anything when the save dialog is cancelled", async () => {
    const chooseDestination = vi.fn().mockResolvedValue(null);
    const copyFile = vi.fn();

    await expect(
      saveLocalArtifact(
        {
          sourcePath: "/private/app/outputs/lesson.vtt",
          defaultPath: "lesson.vtt",
          filterName: "WebVTT captions",
          extensions: ["vtt"],
        },
        chooseDestination,
        copyFile,
      ),
    ).resolves.toBeNull();

    expect(copyFile).not.toHaveBeenCalled();
  });
});

describe("saveLocalTextArtifact", () => {
  it("writes generated content to the destination returned by the save dialog", async () => {
    const chooseDestination = vi.fn().mockResolvedValue("/Users/tester/Desktop/readings.html");
    const writeTextFile = vi.fn().mockResolvedValue(undefined);

    await expect(
      saveLocalTextArtifact(
        {
          contents: "<h1>Readings</h1>",
          defaultPath: "readings.html",
          filterName: "HTML export",
          extensions: ["html"],
        },
        chooseDestination,
        writeTextFile,
      ),
    ).resolves.toEqual({ destinationPath: "/Users/tester/Desktop/readings.html" });

    expect(writeTextFile).toHaveBeenCalledWith(
      "/Users/tester/Desktop/readings.html",
      "<h1>Readings</h1>",
    );
  });

  it("does not write generated content when the save dialog is cancelled", async () => {
    const chooseDestination = vi.fn().mockResolvedValue(null);
    const writeTextFile = vi.fn();

    await expect(
      saveLocalTextArtifact(
        {
          contents: "<h1>Readings</h1>",
          defaultPath: "readings.html",
          filterName: "HTML export",
          extensions: ["html"],
        },
        chooseDestination,
        writeTextFile,
      ),
    ).resolves.toBeNull();

    expect(writeTextFile).not.toHaveBeenCalled();
  });
});
