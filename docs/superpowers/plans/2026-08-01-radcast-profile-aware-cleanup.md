# RADcast Profile-Aware Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent RADcast enhancement profiles from receiving a second generic cleanup pass while preserving standalone cleanup.

**Architecture:** Add matching pure policy helpers in the Rust command boundary and TypeScript settings layer. Rust remains authoritative for legacy and direct requests; Svelte uses the UI helper to communicate the effective behavior while retaining the user's Standard-cleanup preference.

**Tech Stack:** Rust, Svelte 5, TypeScript, Vitest, Cargo tests, FFmpeg, local RADcast enhancement helper.

---

### Task 1: Lock the Processing Policy with Tests

**Files:**
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `apps/desktop-ui/src/lib/radcastSettings.test.ts`

- [x] Add Rust tests proving generic cleanup is suppressed for every enhanced model and retained for `None`.
- [x] Add TypeScript tests proving the interface policy matches Rust.
- [x] Run the focused tests and confirm they fail because the policy helpers do not exist.

### Task 2: Enforce the Policy

**Files:**
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `apps/desktop-ui/src/lib/radcastSettings.ts`
- Modify: `apps/desktop-ui/src/components/RadcastWorkspace.svelte`

- [x] Implement the Rust effective-cleanup helper.
- [x] Use the effective value for audio processing and output metadata.
- [x] Implement the matching TypeScript helper.
- [x] Use it to show profile-specific cleanup guidance instead of the generic checkbox for enhanced profiles.
- [x] Preserve the saved Standard-cleanup preference while an enhanced profile is selected.
- [x] Run focused Rust and UI tests and confirm they pass.

### Task 3: Verify RADcast End to End

**Files:**
- No production file changes expected.

- [x] Run RADcast-focused Rust tests and the desktop UI test suite.
- [x] Run formatting and static checks.
- [x] Process a representative section of the supplied CRJU201 lecture clip with Studio v1.8.
- [x] Confirm output duration, codec, loudness, and peak values are valid and that only the tuned Studio v1.8 post-filter is applied.
- [x] Commit the verified change.
