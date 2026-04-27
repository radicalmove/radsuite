# Svelte UI Shell Design

## Status

Approved in conversation on 2026-04-28.

## Context

RADsuite currently uses a small React/Vite/Tauri desktop shell. The UI only calls the `get_app_status` Tauri command and renders local database, sync, and engine status. The Rust workspace now contains the RADcite domain model, persistence layer, citation analysis, and DOCX ingestion.

The user wants to align frontend technology with a close collaborator who is using Svelte in a Rust app. Because the current UI surface is still small, this is the lowest-risk point to test a Svelte frontend without disturbing the Rust/Tauri backend.

## Goal

Convert the existing desktop UI shell from React to Svelte while preserving the same user-visible behaviour and Tauri command bridge.

## Non-Goals

- Add new RADcite upload or analysis UI.
- Change Rust crates, Tauri commands, database schema, or RADcite logic.
- Adopt SvelteKit.
- Redesign the product UI beyond preserving the existing shell.

## Design

Use Svelte with Vite and TypeScript inside the existing `apps/desktop-ui` package. Keep the Tauri app structure, `src-tauri` configuration, CSS file, and `get_app_status` command unchanged.

Replace:

- `apps/desktop-ui/src/App.tsx`
- `apps/desktop-ui/src/main.tsx`
- React package dependencies and TypeScript settings

With:

- `apps/desktop-ui/src/App.svelte`
- `apps/desktop-ui/src/main.ts`
- Svelte/Vite package dependencies and TypeScript settings

The Svelte app should load status on mount with `invoke<AppStatus>("get_app_status")`, show the same topbar/status pills, list engines, and show the same command bridge error notice if invocation fails.

## Testing

Verify that:

- `npm run build` succeeds in `apps/desktop-ui`.
- `cargo test -p radsuite-desktop` still passes.
- `cargo test --workspace` still passes after dependency/config changes.

Plan/spec reviewer subagent steps were not run because this Codex session only permits subagents when the user explicitly asks for delegation.
