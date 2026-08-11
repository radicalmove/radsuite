import type { AnalyseDocxReviewResponse, ImportDocumentReadingsResponse } from "../types";

export function shouldAutoImportReadings(
  analysis: AnalyseDocxReviewResponse | null,
  sourcePath: string,
): boolean {
  return analysis !== null && sourcePath.trim().length > 0;
}

export function describeReadingImport(result: ImportDocumentReadingsResponse): string {
  if (result.candidate_count === 0) {
    return result.failed_file_count > 0
      ? `${result.failed_file_count} source ${result.failed_file_count === 1 ? "file could" : "files could"} not be read.`
      : "No readings were detected in this document.";
  }

  const parts = [`Processed ${result.saved_count} of ${result.candidate_count} detected readings.`];
  if (result.created_module_count > 0) {
    parts.push(
      `Created ${result.created_module_count} ${result.created_module_count === 1 ? "module" : "modules"}.`,
    );
  }
  if (result.unassigned_count > 0) {
    parts.push(
      `${result.unassigned_count} ${result.unassigned_count === 1 ? "reading needs" : "readings need"} a module assignment.`,
    );
  }
  if (result.failed_file_count > 0) {
    parts.push(
      `${result.failed_file_count} source ${result.failed_file_count === 1 ? "file was" : "files were"} not readable.`,
    );
  }
  return parts.join(" ");
}
