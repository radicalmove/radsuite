import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const css = readFileSync(resolve(root, "src/styles.css"), "utf8").toLowerCase();
const app = readFileSync(resolve(root, "src/App.svelte"), "utf8");
const sidebar = readFileSync(resolve(root, "src/components/ProjectSidebar.svelte"), "utf8");
const workspace = readFileSync(
  resolve(root, "src/components/RadciteDocumentsWorkspace.svelte"),
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

for (const needle of ["theme-toggle", "radciteTheme", "moonIcon", "data-theme={theme}"]) {
  if (!app.includes(needle)) {
    missing.push(`app includes ${needle}`);
  }
}

if (!workspace.includes('data-filter="needs-citation"')) {
  missing.push("workspace marks needs-citation summary filter");
}

if (missing.length > 0) {
  console.error("RADcite style contract failed:");
  for (const label of missing) {
    console.error(`- ${label}`);
  }
  process.exit(1);
}

console.log("RADcite style contract passed.");
