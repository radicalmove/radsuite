# RADcite Shell Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the RADcite shell status, theme, sidebar labels, and summary/action-panel styling before PR #6 is finalized.

**Architecture:** Keep the current Svelte component structure. Extend the existing style contract script to protect markup and CSS expectations, then update `App.svelte`, `ProjectSidebar.svelte`, assets, and `styles.css`.

**Tech Stack:** Svelte 5, Vite, Tauri 2, TypeScript, CSS, Node style contract.

---

## File Structure

- Create: `apps/desktop-ui/src/assets/moon.png`
- Modify: `apps/desktop-ui/scripts/verify-style-contract.mjs`
- Modify: `apps/desktop-ui/src/App.svelte`
- Modify: `apps/desktop-ui/src/components/ProjectSidebar.svelte`
- Modify: `apps/desktop-ui/src/components/RadciteDocumentsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/styles.css`

## Task 1: Contract Test

- [x] Extend `verify-style-contract.mjs` to require:
  - `status-chip` and `status-dot`
  - `theme-toggle`
  - `data-theme="dark"` styling
  - `radciteTheme` localStorage key
  - `Audio cleanup`, `Voice generation`, `RADcast`, `RADTTS`
  - `data-filter="needs-citation"` summary styling
- [x] Run `npm run test:style`
- [x] Expected: FAIL until implementation lands.

## Task 2: Implementation

- [x] Copy the old RADcite `moon.png` asset.
- [x] Add theme state and persistence in `App.svelte`.
- [x] Replace status pills with dot-based status chips.
- [x] Rename sidebar tool entries with task-first labels and product sublabels.
- [x] Add summary `data-filter` attributes.
- [x] Update CSS for status chips, dark mode, theme toggle, summary emphasis, and tighter empty actions panel.

## Task 3: Verification And Publish

- [x] Run `npm run test:style`
- [x] Run `npm run build`
- [x] Run `cargo test --workspace`
- [x] Run `git diff --check`
- [ ] Commit and push to PR #6.

## Review Note

The normal plan workflow asks for a plan-review subagent. This session’s active instructions only allow subagents when the user explicitly asks for them, so the subagent review step is intentionally skipped.
