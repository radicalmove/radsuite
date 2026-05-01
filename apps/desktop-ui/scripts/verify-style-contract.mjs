import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const css = readFileSync(resolve(root, "src/styles.css"), "utf8").toLowerCase();
const sidebar = readFileSync(resolve(root, "src/components/ProjectSidebar.svelte"), "utf8");

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
];

const missing = checks
  .filter(([, needle]) => !css.includes(needle))
  .map(([label]) => label);

if (!sidebar.includes("radciteLogo")) {
  missing.push("sidebar imports RADcite logo");
}

if (missing.length > 0) {
  console.error("RADcite style contract failed:");
  for (const label of missing) {
    console.error(`- ${label}`);
  }
  process.exit(1);
}

console.log("RADcite style contract passed.");
