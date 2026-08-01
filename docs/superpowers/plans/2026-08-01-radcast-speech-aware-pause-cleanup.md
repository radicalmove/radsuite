# RADcast Speech-Aware Pause Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Make RADsuite's RADcast pause slider use the original speech-aware cleanup decisions while preserving local processing and the existing FFmpeg output boundary.

**Architecture:** Add pure cleanup planning beside the existing caption timing types. The desktop workflow will request one fast word-timestamp transcription when pause or filler cleanup is enabled, pass the planner's merged intervals to the existing concat renderer, and persist separate pause/filler counts. The Svelte workspace will use the existing caption capability as the transcription prerequisite and explain the dependency to users.

**Tech Stack:** Rust, `whisper-cli`, FFmpeg, Tauri command contracts, Svelte 5, TypeScript, Vitest.

---

### Task 1: Add the pure speech cleanup planner

**Files:**
- Modify: `crates/radsuite-engines/src/captions.rs`
- Test: `crates/radsuite-engines/tests/captions.rs`

- [ ] **Step 1: Write failing planner tests**

Add tests covering: merged speech words; inter-word gaps longer than 350 ms keeping exactly the configured duration; leading and trailing gaps; gaps at or below 350 ms remaining untouched; filler words excluded from the speech timeline; pause and filler intervals merged; and non-finite/negative timing inputs being rejected or ignored safely.

- [ ] **Step 2: Run the focused tests and confirm the expected failure**

Run: `cargo test -p radsuite-engines --test captions speech_cleanup -- --nocapture`

Expected: FAIL because the planner contract does not yet exist.

- [ ] **Step 3: Implement the minimal planner contract**

Add `SpeechCleanupPlan` and `plan_speech_cleanup(words, total_duration, max_silence_seconds, remove_filler_words, filler_mode)`. Reuse `detect_filler_intervals`, merge speech intervals with a 60 ms tolerance, apply the 350 ms compaction threshold, clamp intervals to the supplied duration, and return merged removal intervals with separate pause and filler counts. Keep the planner deterministic and independent of filesystem or process execution.

- [ ] **Step 4: Run the focused planner tests**

Run: `cargo test -p radsuite-engines --test captions speech_cleanup -- --nocapture`

Expected: all planner tests pass.

- [ ] **Step 5: Commit the engine planner**

Run: `git add crates/radsuite-engines/src/captions.rs crates/radsuite-engines/tests/captions.rs && git commit -m "feat: add speech-aware RADcast cleanup planning"`

### Task 2: Expose one transcription pass for cleanup planning

**Files:**
- Modify: `crates/radsuite-engines/src/captions.rs`
- Test: `crates/radsuite-engines/tests/captions.rs`

- [ ] **Step 1: Write a failing cleanup-plan transcription test**

Use the existing fake `whisper-cli` fixture to assert that a cleanup-plan request returns clip-relative word timestamps and the planner output without running a second transcription command.

- [ ] **Step 2: Run the focused test and confirm it fails**

Run: `cargo test -p radsuite-engines --test captions caption_processor_builds_cleanup_plan -- --nocapture`

Expected: FAIL because the processor method is not yet available.

- [ ] **Step 3: Implement the processor adapter**

Add a `CaptionProcessor::speech_cleanup_plan` method that validates the transcription request, transcribes words using the fast profile, converts timings to the selected clip's coordinate system, and calls the pure planner. Preserve existing `filler_intervals` behavior for other callers.

- [ ] **Step 4: Run caption tests**

Run: `cargo test -p radsuite-engines --test captions`

Expected: all caption tests pass.

- [ ] **Step 5: Commit the processor adapter**

Run: `git add crates/radsuite-engines/src/captions.rs crates/radsuite-engines/tests/captions.rs && git commit -m "feat: build RADcast cleanup plans from local timestamps"`

### Task 3: Replace generic pause filtering in desktop processing

**Files:**
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Test: `crates/radsuite-desktop/tests/radcast_contracts.rs`

- [ ] **Step 1: Add failing desktop contract coverage**

Add a deterministic fake-caption test that requests pause cleanup, asserts the final audio request contains concat removal intervals rather than `silenceremove`, checks the selected pause duration is preserved by the interval plan, and checks the output records separate shortened-pause and filler counts.

- [ ] **Step 2: Run the focused test and confirm it fails**

Run: `cargo test -p radsuite-desktop --test radcast_contracts speech_aware_pause -- --nocapture`

Expected: FAIL because desktop processing currently sends `max_silence_seconds` to FFmpeg and has no pause-count field.

- [ ] **Step 3: Add output metadata and use the shared plan**

Add `removed_pause_count` with a zero serde default to `RadcastAudioOutput`. In the processing pipeline, request one cleanup plan when either pause reduction or filler removal is enabled, pass its merged intervals with `max_silence_seconds: None` to `AudioProcessor`, and persist both counts. Keep cancellation and temporary-file cleanup around the new transcription stage.

- [ ] **Step 4: Run focused desktop tests**

Run: `cargo test -p radsuite-desktop --test radcast_contracts speech_aware_pause -- --nocapture`

Expected: the new contract passes and existing RADcast contracts remain green.

- [ ] **Step 5: Commit desktop integration**

Run: `git add crates/radsuite-desktop/src/radcast.rs crates/radsuite-desktop/src/commands.rs crates/radsuite-desktop/tests/radcast_contracts.rs && git commit -m "feat: use speech-aware pause cleanup in RADcast"`

### Task 4: Surface capability and result details in the UI

**Files:**
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src/types.ts`
- Modify: `apps/desktop-ui/src/components/RadcastWorkspace.svelte`
- Modify: `apps/desktop-ui/src/styles.css`
- Test: `apps/desktop-ui/src/lib/radcastSettings.test.ts`

- [ ] **Step 1: Add a failing frontend contract test**

Test the pure settings/capability helper for disabling pause and filler cleanup when caption support is unavailable, and test that a zero pause count is displayed correctly rather than being hidden by truthiness.

- [ ] **Step 2: Run the focused frontend test and confirm it fails**

Run from `apps/desktop-ui`: `npm test -- --run src/lib/radcastSettings.test.ts`

Expected: FAIL because the capability helper and pause-count display behavior do not yet exist.

- [ ] **Step 3: Implement the UI contract**

Add `removed_pause_count` to the TypeScript output type, disable pause/filler controls when caption support is unavailable with a clear local-runtime note, and show pause reductions in completion status and output metadata using explicit null checks. Keep the existing range slider and settings persistence intact.

- [ ] **Step 4: Run frontend verification**

Run: `npm test -- --run && npm run check && npm run test:style && npm run build`

Expected: all frontend tests, diagnostics, style checks, and build pass.

- [ ] **Step 5: Commit the UI slice**

Run: `git add crates/radsuite-desktop/src/commands.rs apps/desktop-ui/src/types.ts apps/desktop-ui/src/components/RadcastWorkspace.svelte apps/desktop-ui/src/styles.css apps/desktop-ui/src/lib/radcastSettings.test.ts && git commit -m "feat: explain and report speech-aware RADcast cleanup"`

### Task 5: Full verification and integration

**Files:**
- No planned source changes.

- [ ] **Step 1: Run complete Rust verification**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`

Expected: zero formatting, lint, or test failures.

- [ ] **Step 2: Run complete frontend verification**

Run from `apps/desktop-ui`: `npm test -- --run && npm run check && npm run test:style && npm run build`

Expected: all frontend checks pass.

- [ ] **Step 3: Run the real local RADcast smoke**

When the local audio/model assets are present, run the existing `radcast_contracts` smoke with `RADSUITE_REAL_RADCAST_AUDIO` and `RADSUITE_REAL_RADCAST_OPTIMIZED_AUDIO` set. Confirm the output is playable, the manifest contains the pause count, and no temporary prepared files remain.

- [ ] **Step 4: Inspect, push, and open a ready PR**

Run: `git diff main...HEAD --check`, `git status --short`, `git push -u origin codex/radcast-speech-aware-pauses`, then create a ready PR against `main` describing the speech-aware cleanup behavior.

- [ ] **Step 5: Wait for CI, merge, and fast-forward main**

Run: `gh pr checks <number> --watch --interval 10`; after both UI and Rust checks pass, merge using the established non-interactive API workflow and fast-forward `/Users/rcd58/Documents/RADsuite`.
