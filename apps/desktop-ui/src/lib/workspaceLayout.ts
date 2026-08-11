import type { ToolArea } from "../types";

export function showsCitationActions(area: ToolArea): boolean {
  return area === "documents";
}
