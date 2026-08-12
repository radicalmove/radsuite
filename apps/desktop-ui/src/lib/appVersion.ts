export function displayAppVersion(version: string | null | undefined): string {
  const trimmed = version?.trim();
  return trimmed ? `v${trimmed.replace(/^v/i, "")}` : "Version unavailable";
}
