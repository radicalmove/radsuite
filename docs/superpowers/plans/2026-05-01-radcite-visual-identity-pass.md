# RADcite Visual Identity Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the previous RADcite/UC visual identity to the new project-first Svelte/Tauri shell while aligning its app-shell conventions with the colleague Rise course builder app.

**Architecture:** Keep the existing Svelte component split and change only the visual layer plus small branding markup. Use a local CSS token layer inspired by the Rise course builder app’s `app/src/tokens.css`, but keep RADcite colours and semantics. Add one Node-based CSS contract test so palette and status semantics stay intentional without adding a browser test framework.

**Tech Stack:** Svelte 5, Vite, Tauri 2, TypeScript, CSS, Node script verification.

---

## File Structure

- Create: `apps/desktop-ui/src/assets/radcite-logo.svg` for the carried-forward RADcite logo.
- Create: `apps/desktop-ui/scripts/verify-style-contract.mjs` for palette/status CSS checks.
- Modify: `apps/desktop-ui/package.json` to add a `test:style` script.
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte` to render the logo and active tool label.
- Modify: `apps/desktop-ui/src/styles.css` to introduce RADcite tokens and restyle the shell.
- Reference: `/tmp/rise-course-builder-app/app/src/tokens.css`, `/tmp/rise-course-builder-app/app/src/styles.css`, and `/tmp/rise-course-builder-app/app/src/components.css` for token structure, dark sidebar, course navigation density, and card/control treatment.

## Task 1: CSS Contract Test

- [ ] **Step 1: Add the failing style contract**

Create `apps/desktop-ui/scripts/verify-style-contract.mjs` with checks for `--radcite-red: #ce3e2e`, `--radcite-black`, `--font-sans`, a red primary button, green citation badges, and red missing-citation states.

- [ ] **Step 2: Add npm script**

Add `"test:style": "node scripts/verify-style-contract.mjs"` to `apps/desktop-ui/package.json`.

- [ ] **Step 3: Verify RED**

Run: `npm run test:style`

Expected: FAIL because the current stylesheet does not expose the RADcite token contract.

## Task 2: Apply Visual Identity

- [ ] **Step 1: Add logo asset**

Create `apps/desktop-ui/src/assets/radcite-logo.svg` from the previous RADcite app asset.

- [ ] **Step 2: Update sidebar branding**

Modify `ProjectSidebar.svelte` to import and display the logo next to RADsuite/RADcite branding.

- [ ] **Step 3: Restyle CSS**

Modify `styles.css` to use RADcite/UC tokens, Poppins-first typography, neutral surfaces, dark sidebar, red primary actions, green citation badges, and red missing-citation states. Keep CUBE alignment structural only: tokenized CSS, compact spacing, white cards, subtle borders, and course-first sidebar rhythm.

- [ ] **Step 4: Verify GREEN**

Run: `npm run test:style`

Expected: PASS.

## Task 3: Build And Publish

- [ ] **Step 1: Build frontend**

Run: `npm run build`

Expected: `svelte-check found 0 errors and 0 warnings` and Vite build succeeds.

- [ ] **Step 2: Check git diff**

Run: `git diff --check`

Expected: no output.

- [ ] **Step 3: Commit and push**

Commit docs and implementation, then push `codex/project-first-radcite-shell` so PR #6 updates.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so the subagent review step is intentionally skipped.
