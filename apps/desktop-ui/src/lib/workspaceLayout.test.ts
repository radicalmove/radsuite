import { describe, expect, test } from "vitest";
import { showsCitationActions } from "./workspaceLayout";

describe("workspace layout", () => {
  test("shows citation actions only in the document review workspace", () => {
    expect(showsCitationActions("documents")).toBe(true);
    expect(showsCitationActions("radcast")).toBe(false);
    expect(showsCitationActions("radtts")).toBe(false);
    expect(showsCitationActions("radt-tools")).toBe(false);
    expect(showsCitationActions("references")).toBe(false);
    expect(showsCitationActions("readings")).toBe(false);
    expect(showsCitationActions("exports")).toBe(false);
    expect(showsCitationActions("archive")).toBe(false);
  });
});
