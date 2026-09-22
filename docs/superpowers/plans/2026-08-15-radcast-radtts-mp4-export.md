# RADcast and RADTTS MP4 Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional MP4 exports to RADcast and both RADTTS output paths, using a reusable presenter image on the left and an audio-responsive neutral waveform on the right, without changing MP3/WAV behavior.

**Architecture:** Preserve the audio-only engine and CLI contracts. MP4 requests render their exact final audio into an isolated WAV, then pass that WAV and a managed project image to the existing cancellable Rust video exporter. Shared desktop storage owns project covers and output-scoped overrides; each workflow persists only a validated MP4 and durable artifacts.

**Tech Stack:** Rust 2024, `radsuite-engines`, `radsuite-desktop`, FFmpeg/FFprobe, serde, Tauri 2, Svelte 5, TypeScript, Vitest.

---

## Current Foundation

Already committed: `MediaOutputFormat::{Mp3, Wav, Mp4}` and legacy normalization; media-format propagation through RADcast/RADTTS types; the cancellable process runner; and the transactional FFmpeg/FFprobe exporter with progress, validation, cleanup, and tests. Extend these rather than recreating them. Do not alter RADcast enhancement filters or RADTTS synthesis models.

## File Map

- Modify `crates/radsuite-engines/src/video.rs` and `crates/radsuite-engines/tests/video.rs`: approved 540px presenter/740px waveform composition.
- Create `crates/radsuite-desktop/src/media_assets.rs`: static-image validation, project covers, output overrides, rollback, and deletion.
- Modify `crates/radsuite-desktop/src/media_export.rs` and `lib.rs`: shared records and exports.
- Modify `crates/radsuite-desktop/src/state.rs`: invoke partial/orphan recovery during startup.
- Modify `radcast.rs`, `radt_ts.rs`, `radt_ts_tools.rs`, `commands.rs`, and Tauri `main.rs`: workflow orchestration and commands.
- Modify desktop contract tests: lifecycle, compatibility, rollback, and deletion.
- Create `apps/desktop-ui/src/lib/mediaExport.ts` and its test; modify types, three audio workspaces, and styles.

### Task 1: Change the Existing Exporter to the Approved Split Layout

**Files:**
- Modify: `crates/radsuite-engines/src/video.rs`
- Modify: `crates/radsuite-engines/tests/video.rs`

- [ ] **Step 1: Replace the old expectation with a failing split-layout test.** Assert `PRESENTER_WIDTH=540`, `WAVEFORM_PANEL_WIDTH=740`, `WAVEFORM_WIDTH=660`, `WAVEFORM_HEIGHT=240`, and this graph:

```text
[0:v]scale=540:720:force_original_aspect_ratio=increase,crop=540:720:(iw-540)/2:(ih-720)/2[presenter];color=c=0x101214:s=740x720:r=30[panel];[1:a]showwaves=s=660x240:mode=cline:rate=30:colors=white,format=rgba,colorkey=black:0.01:0.0[waveform];[panel][waveform]overlay=40:240[wave_panel];[presenter][wave_panel]hstack=inputs=2,fps=30,format=yuv420p[video]
```

- [ ] **Step 2: Run `cargo test -p radsuite-engines --test video ffmpeg_arguments -- --nocapture`.** Expect failure because the old graph uses a full-frame image.
- [ ] **Step 3: Implement only the constants and filter graph.** Preserve stream mapping, H.264/AAC options, progress, cancellation, probing, and promotion.
- [ ] **Step 4: Run `cargo test -p radsuite-engines --test video`; expect PASS.**
- [ ] **Step 5: Commit:** `git commit -am "feat: add split presenter MP4 layout"`.

### Task 2: Add Transactional Presenter-Image Storage

**Files:**
- Create: `crates/radsuite-desktop/src/media_assets.rs`
- Modify: `crates/radsuite-desktop/src/media_export.rs`
- Modify: `crates/radsuite-desktop/src/lib.rs`
- Modify: `crates/radsuite-desktop/tests/media_export_contracts.rs`

- [ ] **Step 1: Write failing validation tests.** Accept PNG, JPEG, and static WebP; reject missing/non-regular/unsupported/animated/>50 MB inputs.
- [ ] **Step 2: Write failing storage tests.** Verify `<data_dir>/media/projects/<project_id>/cover/cover.<ext>`, metadata, UUID scratch copies, and `exports/<output_id>/image.<ext>`.
- [ ] **Step 3: Write failing transaction tests.** Failure/cancellation preserves the current cover; successful cross-extension replacement removes the obsolete cover.
- [ ] **Step 3a: Write failing recovery tests.** Failed rollback paths are written atomically to `<project-media-root>/orphan-cleanup.json`; cleanup-required errors name every path and ownership type; startup retry removes resolved entries but retains and reports failures.
- [ ] **Step 4: Run `cargo test -p radsuite-desktop --test media_export_contracts media_assets -- --nocapture`; expect FAIL.**
- [ ] **Step 5: Implement contained-path and static-image validation plus this lifecycle:**

```rust
pub fn saved_cover(&self, project_id: &str) -> Result<Option<ManagedImage>, MediaAssetError>;
pub fn stage_image(&self, project_id: &str, source: &Path) -> Result<StagedImage, MediaAssetError>;
pub fn prepare_commit(&self, staged: StagedImage, output_id: Uuid, save_as_cover: bool) -> Result<PendingImageCommit, MediaAssetError>;
pub fn finalize_commit(&self, pending: PendingImageCommit) -> Result<ManagedImage, MediaAssetError>;
pub fn rollback_commit(&self, pending: PendingImageCommit) -> Result<(), MediaAssetError>;
```

- [ ] **Step 6: Implement reversible promotion.** `PendingImageCommit` retains backup paths and previous metadata until the workflow manifest commits; rollback restores both the old cover file and metadata. Finalization removes backups and obsolete extensions only after manifest success.
- [ ] **Step 7: Implement safe deletion and persisted recovery.** Enforce project-root containment, reference-count output images across records, never delete the reusable cover, atomically update the orphan ledger, and return actionable cleanup errors.
- [ ] **Step 8: Wire startup recovery in `state.rs`.** Scan `.partial-*` files and each project orphan ledger without listing them as outputs; retry recorded paths and preserve unresolved entries.
- [ ] **Step 9: Run focused tests and commit:** `git commit -m "feat: add managed presenter images"`.

### Task 3: Integrate MP4 with RADcast

**Files:**
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `crates/radsuite-desktop/src/media_export.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `crates/radsuite-desktop/tests/radcast_contracts.rs`

- [ ] **Step 1: Write failing request tests.** MP4 requires a managed image and supports `save_image_as_project_default`; MP3/WAV ignore image fields.
- [ ] **Step 2: Write failing lifecycle tests.** The exact cleaned/trimmed result becomes a scratch WAV; only MP4 persists; captions remain separate; the record keeps `source_id`, never the scratch path.
- [ ] **Step 3: Write failing progress/failure/cancellation/deletion tests.** Video maps to 90-99%; 100% follows manifest commit; rollback preserves existing outputs and cover. Deletion validates containment, atomically removes the record, removes only owned MP4/caption artifacts, and retains shared images until their last reference.
- [ ] **Step 4: Run `cargo test -p radsuite-desktop --test radcast_contracts -- --nocapture`; expect MP4 failures.**
- [ ] **Step 5: Implement the MP4 branch.** Stage image, render the existing processor to scratch WAV unchanged, export video, atomically commit output/image/manifest, and clean scratch.
- [ ] **Step 6: Add list/delete and Tauri wiring.** Verify every primary/artifact/image path is inside the owning roots, atomically update the manifest, remove only files owned by that record, retain reusable covers, and retain an output image referenced by another record.
- [ ] **Step 7: Run tests and commit:** `git commit -m "feat: add RADcast MP4 exports"`.

### Task 4: Integrate MP4 with RADTTS Voice Generation

**Files:**
- Modify: `crates/radsuite-desktop/src/radt_ts.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Create or modify: `crates/radsuite-desktop/tests/radt_ts_contracts.rs`

- [ ] **Step 1: Write failing compatibility tests.** Existing MP3/WAV CLI arguments and `outputs.json` remain unchanged; MP4 maps to WAV only in an isolated scratch project.
- [ ] **Step 2: Write failing record tests.** `media-outputs.json` stores `kind=voice`, MP4/image/duration/captions/provenance and merges with legacy audio records.
- [ ] **Step 3: Write failing rollback/cancellation/deletion tests.** Scratch WAV and CLI manifests are never listed. Deletion covers the MP4, owned caption artifacts, contained paths, atomic manifest updates, and shared-image references.
- [ ] **Step 4: Run `cargo test -p radsuite-desktop --test radt_ts_contracts voice_mp4 -- --nocapture`; expect FAIL.**
- [ ] **Step 5: Implement synthesis to scratch WAV, promote durable captions, export/probe MP4, commit image/record, then remove scratch.**
- [ ] **Step 6: Add `RenderingVideo` and deletion commands; cancellation wins until commit.**
- [ ] **Step 7: Run tests and commit:** `git commit -m "feat: add RADTTS voice MP4 exports"`.

### Task 5: Integrate MP4 with RADTTS Verified Clips

**Files:**
- Modify: `crates/radsuite-desktop/src/radt_ts_tools.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `crates/radsuite-desktop/tests/radt_ts_contracts.rs`

- [ ] **Step 1: Write failing tests.** The extracted clip is the scratch WAV; boundary/timed artifacts become durable; `kind=clip` records merge with scanned legacy clips.
- [ ] **Step 2: Write failing progress/cancellation/deletion tests.** Verify 90-99%, no scratch paths, image ownership, cover retention, contained boundary/timed artifact deletion, atomic manifest updates, and shared-image references.
- [ ] **Step 3: Run `cargo test -p radsuite-desktop --test radt_ts_contracts clip_mp4 -- --nocapture`; expect FAIL.**
- [ ] **Step 4: Implement only the verified-clip MP4 branch; leave transcription-only and audio branches unchanged.**
- [ ] **Step 5: Run tests and commit:** `git commit -m "feat: add RADTTS clip MP4 exports"`.

### Task 6: Add Shared UI Contracts

**Files:**
- Create: `apps/desktop-ui/src/lib/mediaExport.ts`
- Create: `apps/desktop-ui/src/lib/mediaExport.test.ts`
- Modify: `apps/desktop-ui/src/types.ts`

- [ ] **Step 1: Write failing Vitest tests.** Cover labels/extensions, image requirement, saved-cover precedence, overrides/default intent, audio/video player selection, and download filters.
- [ ] **Step 2: From `apps/desktop-ui`, run `npm test -- --run src/lib/mediaExport.test.ts`; expect FAIL.**
- [ ] **Step 3: Implement pure helpers and matching types so components do not duplicate MP4 branching.**
- [ ] **Step 4: Run the focused test and `npm run check`; commit:** `git commit -m "feat: add MP4 export UI contracts"`.

### Task 7: Add MP4 Controls to All Audio Workspaces

**Files:**
- Modify: `apps/desktop-ui/src/components/RadcastWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadtTsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadtTsToolsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/styles.css`
- Modify: relevant UI tests

- [ ] **Step 1: Add failing tests.** MP4 shows required presenter controls; MP3/WAV hide them; saved images auto-select; creation is disabled without an image.
- [ ] **Step 2: Add the Tauri image chooser and actions:** `Use saved project image`, `Choose a different image`, and `Save this image for future MP4s`.
- [ ] **Step 3: Add a fixed 16:9 preview with the actual cropped image on the left and representative white waveform on charcoal at right.**
- [ ] **Step 4: Use `<video controls>` for MP4, retain `<audio controls>` for audio, add MP4 save filters, and wire deletion.**
- [ ] **Step 5: Show `Rendering video` in the existing status location, avoiding duplicate status or extra scrolling.**
- [ ] **Step 6: Run `npm run check`, `npm test -- --run`, and `npm run test:style`.**
- [ ] **Step 7: Commit:** `git commit -m "feat: add MP4 controls to audio workspaces"`.

### Task 8: Real Media and Regression Verification

**Files:**
- Modify: `crates/radsuite-desktop/tests/real_course_smoke.rs` only if suitable
- Modify: feature files only for defects found

- [ ] **Step 1: Run an ignored real-media smoke with a short lecture clip and non-16:9 presenter image for RADcast, RADTTS voice, and verified clip.**
- [ ] **Step 2: Probe each output.** Require one 1280x720 H.264/yuv420p 30fps stream, one AAC stream, positive duration within 0.10 seconds, visible presenter crop, and moving waveform.
- [ ] **Step 3: Verify the packaged app.** Reuse/override/replace an image, cancel, download, delete, and confirm MP3/WAV still work.
- [ ] **Step 3a: Simulate rollback failure and restart.** Confirm the error names cleanup paths, startup recovery consumes resolved orphan entries, unresolved entries remain recorded, and no partial/orphan appears in output listings.
- [ ] **Step 4: Run `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`.**
- [ ] **Step 5: From `apps/desktop-ui`, run `npm run check`, `npm test -- --run`, `npm run build`, and `npm run test:style`.**
- [ ] **Step 6: Use @superpowers:verification-before-completion and commit final smoke additions:** `git commit -m "test: verify MP4 media exports"`.

## Execution Notes

- Follow @superpowers:test-driven-development for every production change.
- Use @superpowers:verification-before-completion before claiming completion.
- Do not bump versions or publish installers in this plan; release packaging follows acceptance.
- Preserve the separate VTT download fix already committed on this branch.
