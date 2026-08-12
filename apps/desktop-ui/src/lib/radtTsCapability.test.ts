import { describe, expect, it } from "vitest";
import type { RadtTsCapabilityStatus } from "../types";
import { capabilityNotice } from "./radtTsCapability";

const unavailable: RadtTsCapabilityStatus = {
  available: false,
  executable: null,
  detail: "Install RADTTS locally or set RADSUITE_RADTTS_CLI to its executable.",
  supports_builtin_voices: false,
  builtin_voices: [],
};

describe("RADTTS capability notice", () => {
  it("does not present the initial capability check as a setup failure", () => {
    expect(capabilityNotice(true, unavailable)).toBe(
      "Checking local voice generation support...",
    );
  });

  it("uses the runtime detail after the check has completed", () => {
    expect(capabilityNotice(false, unavailable)).toBe(unavailable.detail);
  });
});
