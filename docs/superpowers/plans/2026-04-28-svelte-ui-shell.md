# Svelte UI Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the existing RADsuite desktop shell from React/Vite to Svelte/Vite without changing the Rust/Tauri backend.

**Architecture:** Keep `apps/desktop-ui` as the Tauri frontend package and keep the existing `src-tauri` app. Replace only the frontend framework entrypoint and component files, preserving the `get_app_status` command contract and current CSS classes.

**Tech Stack:** Svelte 5, Vite, TypeScript, Tauri 2, existing Rust workspace.

---

### Task 1: Frontend Dependency Swap

**Files:**
- Modify: `apps/desktop-ui/package.json`
- Modify: `apps/desktop-ui/package-lock.json`
- Modify: `apps/desktop-ui/tsconfig.json`
- Modify: `apps/desktop-ui/vite.config.ts`

- [ ] **Step 1: Replace React packages with Svelte packages**

Remove `@vitejs/plugin-react`, `react`, and `react-dom`. Add `@sveltejs/vite-plugin-svelte` and `svelte`.

- [ ] **Step 2: Update Vite config**

Change the Vite plugin from React to Svelte.

- [ ] **Step 3: Update TypeScript config**

Use a Svelte-compatible config that includes `.svelte` files and does not require JSX.

- [ ] **Step 4: Install dependencies**

Run: `npm install` from `apps/desktop-ui`

Expected: lockfile updates cleanly.

### Task 2: Svelte App Shell

**Files:**
- Delete: `apps/desktop-ui/src/App.tsx`
- Delete: `apps/desktop-ui/src/main.tsx`
- Create: `apps/desktop-ui/src/App.svelte`
- Create: `apps/desktop-ui/src/main.ts`
- Preserve: `apps/desktop-ui/src/styles.css`

- [ ] **Step 1: Create the Svelte entrypoint**

Mount `App.svelte` to `document.getElementById("root")`.

- [ ] **Step 2: Port the app shell component**

Implement the existing `AppStatus`, `EngineStatus`, fallback status, `get_app_status` invocation, error notice, status pills, project panel, and engine list in Svelte.

- [ ] **Step 3: Keep class names unchanged**

Reuse the existing CSS class names so styling behaviour stays stable.

### Task 3: Verification

**Files:**
- Workspace verification only.

- [ ] **Step 1: Build the Svelte UI**

Run: `npm run build` from `apps/desktop-ui`

Expected: PASS.

- [ ] **Step 2: Run desktop Rust contracts**

Run: `cargo test -p radsuite-desktop`

Expected: PASS.

- [ ] **Step 3: Run workspace tests**

Run: `cargo test --workspace`

Expected: PASS.

- [ ] **Step 4: Commit**

Commit message: `chore: convert desktop shell to Svelte`
