import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const css = readFileSync(resolve(root, "src/styles.css"), "utf8").toLowerCase();
const packageJson = readFileSync(resolve(root, "package.json"), "utf8");
const tauriMain = readFileSync(resolve(root, "src-tauri/src/main.rs"), "utf8");
const app = readFileSync(resolve(root, "src/App.svelte"), "utf8");
const sidebar = readFileSync(resolve(root, "src/components/ProjectSidebar.svelte"), "utf8");
const storage = readFileSync(resolve(root, "src/lib/storage.ts"), "utf8");
const workspace = readFileSync(
  resolve(root, "src/components/RadciteDocumentsWorkspace.svelte"),
  "utf8",
);
const actionsPanel = readFileSync(
  resolve(root, "src/components/CitationActionsPanel.svelte"),
  "utf8",
);
const referencesWorkspace = readFileSync(
  resolve(root, "src/components/RadciteReferencesWorkspace.svelte"),
  "utf8",
);
const exportsWorkspace = readFileSync(
  resolve(root, "src/components/RadciteExportsWorkspace.svelte"),
  "utf8",
);
const readingsWorkspace = readFileSync(
  resolve(root, "src/components/RadciteReadingsWorkspace.svelte"),
  "utf8",
);

const checks = [
  ["RADcite red token", "--radcite-red: #ce3e2e"],
  ["RADcite black token", "--radcite-black:"],
  ["Poppins-first font token", "--font-sans:"],
  ["primary button uses RADcite red", ".primary-button"],
  ["primary button references RADcite red", "background: var(--radcite-red)"],
  ["citation badges use success green", ".citation-badge"],
  ["citation badges reference success token", "color: var(--success-deep)"],
  ["missing-citation warning uses red tint", ".status-warning"],
  ["warning references danger tint", "background: var(--danger-tint)"],
  ["selected paragraph has red edge", "border-left-color: var(--radcite-red)"],
  ["status chip styling", ".status-chip"],
  ["status dot styling", ".status-dot"],
  ["status chips use square radius", "border-radius: var(--r-sm)"],
  ["theme toggle styling", ".theme-toggle"],
  ["dark theme selector", '[data-theme="dark"]'],
  ["needs-citation summary emphasis", '[data-filter="needs-citation"].is-active'],
  ["linked-citation summary emphasis", '[data-filter="linked-citation"].is-active'],
  ["suggested-citation summary emphasis", '[data-filter="suggested-citation"].is-active'],
  ["unlinked-citation summary emphasis", '[data-filter="unlinked-citation"].is-active'],
  ["review queue status styling", ".queue-status"],
  ["source search panel styling", ".source-search-panel"],
  ["source search row styling", ".source-search-row"],
  ["source result list styling", ".source-result-list"],
  ["source result card styling", ".source-result-card"],
  ["export panel styling", ".export-panel"],
  ["export mode toggle styling", ".export-mode-toggle"],
  ["module export controls styling", ".module-export-controls"],
  ["export preview styling", ".export-preview"],
  ["readings workspace styling", ".readings-workspace"],
  ["module selector styling", ".module-selector"],
  ["reading list panel styling", ".reading-list-panel"],
  ["module card action styling", ".module-card-actions"],
  ["reading row action styling", ".reading-row-actions"],
  ["project create form styling", ".project-create-form"],
  ["project card header layout", ".project-card-header"],
  ["project expand button styling", ".project-expand-button"],
  ["project archive action styling", ".project-action-button"],
  ["archived project section styling", ".archived-projects-section"],
  ["import source selector styling", ".import-source-toggle"],
  ["danger button styling", ".danger-button"],
];

const missing = checks
  .filter(([, needle]) => !css.includes(needle))
  .map(([label]) => label);

if (!sidebar.includes("radciteLogo")) {
  missing.push("sidebar imports RADcite logo");
}

for (const needle of ["Audio cleanup", "Voice generation", "RADcast", "RADTTS"]) {
  if (!sidebar.includes(needle)) {
    missing.push(`sidebar includes ${needle}`);
  }
}

for (const needle of [
  "Active projects",
  "Archived projects",
  "Archive",
  "Restore",
  "project-expand-button",
  "aria-expanded",
]) {
  if (!sidebar.includes(needle)) {
    missing.push(`sidebar includes ${needle}`);
  }
}

for (const needle of ["radciteProjectNavState", "radciteTheme", "browserStorage"]) {
  if (!storage.includes(needle)) {
    missing.push(`storage helper includes ${needle}`);
  }
}

for (const needle of ["reference-add-form", "reference-list-panel", "Course References"]) {
  if (!referencesWorkspace.includes(needle) && !css.includes(needle)) {
    missing.push(`references workspace includes ${needle}`);
  }
}

for (const needle of ["listCourseReferences", "addCourseReference"]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

for (const needle of [
  "RadciteExportsWorkspace",
  'activeArea === "exports"',
  "exportCourseReferences",
  "exportModuleReadings",
]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

for (const needle of [
  "listRadciteProjects",
  "createRadciteProject",
  "archiveRadciteProject",
  "restoreRadciteProject",
  "selectedProjectId",
  "handleCreateProject",
  "handleOpenReadingsFromDocument",
  "RadciteReadingsWorkspace",
  'activeArea === "readings"',
  "listRadciteModules",
  "listModuleReadings",
  "updateRadciteModule",
  "archiveRadciteModule",
  "updateModuleReading",
  "archiveModuleReading",
  "previewModuleReadingsImport",
  "previewModuleReadingsCsvImport",
  "saveModuleReadingsImport",
]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

for (const needle of [
  "Saved on this Mac",
  "Local saving unavailable",
  "Cloud sync on",
  "Cloud sync not connected",
  "title={",
  "aria-label=",
]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

for (const obsolete of ["Local DB ready", "Local DB offline", "Sync configured", "Sync off"]) {
  if (app.includes(obsolete)) {
    missing.push(`app removes obsolete status copy: ${obsolete}`);
  }
}

if (sidebar.includes('{ id: "readings", label: "Readings", disabled: true }')) {
  missing.push("sidebar enables Readings");
}

for (const needle of ["theme-toggle", "moonIcon", "data-theme={theme}"]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

if (!workspace.includes('data-filter="needs-citation"')) {
  missing.push("workspace marks needs-citation summary filter");
}

for (const needle of [
  'data-filter="linked-citation"',
  'data-filter="suggested-citation"',
  'data-filter="unlinked-citation"',
  "Linked citations",
  "Suggested matches",
  "Unlinked citations",
  "Suggested match",
  "Unlinked citation",
]) {
  if (!workspace.includes(needle)) {
    missing.push(`workspace includes ${needle}`);
  }
}

for (const needle of [
  "@tauri-apps/plugin-dialog",
  "choose-docx-button",
  "onChooseDocx",
  "Review readings",
  "onOpenReadings",
]) {
  if (!workspace.includes(needle) && !packageJson.includes(needle)) {
    missing.push(`document workspace includes ${needle}`);
  }
}

for (const needle of [
  "Course References Export",
  "Course references",
  "Module readings",
  "Module readings export",
  "Module selector",
  "AKO | LEARN",
  "Generate HTML",
  "Copy HTML",
  "Download HTML",
  "export-preview",
]) {
  if (!exportsWorkspace.includes(needle)) {
    missing.push(`exports workspace includes ${needle}`);
  }
}

for (const needle of [
  "Module readings",
  "Required",
  "Optional",
  "module-selector",
  "reading-list-panel",
  "reading-import-panel",
  "reading-import-candidate",
  "import-source-toggle",
  "DOCX",
  "CSV",
  "Choose CSV",
  "Preview readings",
  "Save selected readings",
  "Edit module",
  "Remove module",
  "Update module",
  "Cancel edit",
  "Edit reading",
  "Remove reading",
  "Update reading",
]) {
  if (!readingsWorkspace.includes(needle)) {
    missing.push(`readings workspace includes ${needle}`);
  }
}

for (const needle of [
  "export_course_references",
  "ExportCourseReferencesRequest",
  "export_module_readings",
  "ExportModuleReadingsRequest",
  "list_radcite_projects",
  "CreateRadciteProjectRequest",
  "create_radcite_project",
  "ArchiveRadciteProjectRequest",
  "archive_radcite_project",
  "RestoreRadciteProjectRequest",
  "restore_radcite_project",
  "update_radcite_module",
  "UpdateRadciteModuleRequest",
  "archive_radcite_module",
  "ArchiveRadciteModuleRequest",
  "update_module_reading",
  "UpdateModuleReadingRequest",
  "archive_module_reading",
  "ArchiveModuleReadingRequest",
  "preview_module_readings_import",
  "PreviewModuleReadingsImportRequest",
  "preview_module_readings_csv_import",
  "PreviewModuleReadingsCsvImportRequest",
  "save_module_readings_import",
  "SaveModuleReadingsImportRequest",
]) {
  if (!tauriMain.includes(needle)) {
    missing.push(`Tauri main includes ${needle}`);
  }
}

for (const needle of [
  "review-action-form",
  "citation-link-form",
  "manualCitationText",
  "onMarkResolved",
  "onAddManualCitation",
  "onAddCourseReference",
  "onVerifyCitation",
  "onLinkCitation",
  "reference_entry_id",
  "reference_suggestions",
  "Suggested references",
  "suggestion-card",
  "Accept",
  "Search sources",
  "sourceSearchQuery",
  "sourceSearchResults",
  "searchCrossrefWorks",
  "Open Crossref",
  "Crossref results",
  "Open DOI",
  "Add reference",
  "Add & link",
  "Mark citations reviewed",
  "Not required",
  "Reviewed",
]) {
  if (!actionsPanel.includes(needle)) {
    missing.push(`citation actions include ${needle}`);
  }
}

if (!tauriMain.includes("tauri_plugin_dialog::init()")) {
  missing.push("Tauri registers dialog plugin");
}

if (missing.length > 0) {
  console.error("RADcite style contract failed:");
  for (const label of missing) {
    console.error(`- ${label}`);
  }
  process.exit(1);
}

console.log("RADcite style contract passed.");
