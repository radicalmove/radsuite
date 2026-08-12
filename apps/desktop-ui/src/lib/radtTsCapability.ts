import type { RadtTsCapabilityStatus } from "../types";

export function capabilityNotice(
  checking: boolean,
  capability: RadtTsCapabilityStatus,
): string {
  return checking
    ? "Checking local voice generation support..."
    : capability.detail;
}
