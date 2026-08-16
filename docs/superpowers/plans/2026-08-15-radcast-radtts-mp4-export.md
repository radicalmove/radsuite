# RADcast and RADTTS MP4 Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional local MP4 exports to RADcast and RADTTS using a reusable project image and responsive neutral waveform, while preserving all existing MP3/WAV behavior and legacy output listings.

**Architecture:** Keep `AudioOutputFormat` and the RADTTS CLI contract audio-only. Add a desktop-level `MediaOutputFormat` with MP3, WAV, and MP4 variants. A shared Rust video exporter will mux an isolated final WAV with a managed image; desktop orchestration will persist one durable MP4 record and reuse one project-media store across RADcast, RADTTS voice generation, and verified clips.

**Tech Stack:** Rust 2024, `radsuite-engines`, `radsuite-desktop`, Tokio/serde/FFmpeg/FFprobe, Tauri 2, Svelte 5, TypeScript, Vitest.

---

## File Map

- Create `crates/radsuite-engines/src/process.rs`: synchronous cancellable FFmpeg/FFprobe process runner with platform process-tree termination and streamed progress. The runner accepts an executable path, argument vector, cancellation callback, and progress callback so tests can inject temporary fake executables without global mocks.
- Create `crates/radsuite-engines/src/video.rs`: fixed 1280x720 H.264/AAC exporter, exact filter graph, ffprobe validation, and deterministic argument helpers.
- Modify `crates/radsuite-engines/src/lib.rs`: expose process/video APIs.
- Modify `crates/radsuite-engines/src/audio.rs`: route existing FFmpeg/FFprobe methods through the runner while preserving public signatures; add cancellation-aware internal methods.
- Create `crates/radsuite-engines/tests/video.rs`: exporter argument, probe, progress, cancellation, and failure tests.
- Modify `crates/radsuite-engines/tests/audio.rs`: prove existing MP3/WAV arguments and duration behavior remain unchanged.

- Create `crates/radsuite-desktop/src/media_assets.rs`: shared project-cover storage, validation, per-export image copies, orphan cleanup, and safe deletion.
- Create `crates/radsuite-desktop/src/media_export.rs`: desktop media format, MP4 request/result records, project media paths, and shared MP4 orchestration.
- Modify `crates/radsuite-desktop/src/lib.rs`: register and re-export media modules.
- Modify `crates/radsuite-desktop/src/radcast.rs`: accept MP4 requests, render isolated WAV, invoke shared exporter, persist image metadata, list/delete MP4 records, and report video progress.
- Modify `crates/radsuite-desktop/src/radt_ts.rs`: add MP4 voice-generation requests, isolated CLI scratch workspace, durable `media-outputs.json` records, listing merge, and deletion.
- Modify `crates/radsuite-desktop/src/radt_ts_tools.rs`: add MP4 verified clips, shared image/export pipeline, listing merge, and deletion.
- Modify `crates/radsuite-desktop/src/commands.rs`: expose cover, output-delete, and media-aware workflow commands.
- Modify `apps/desktop-ui/src-tauri/src/main.rs`: register new Tauri commands.
- Modify `crates/radsuite-desktop/tests/radcast_contracts.rs`: MP4 workflow, manifest, cleanup, cancellation, and deletion coverage.
- Create `crates/radsuite-desktop/tests/media_export_contracts.rs`: shared image store, MP4 records, listing compatibility, and rollback coverage.
- Modify RADTTS desktop tests or add `crates/radsuite-desktop/tests/radt_ts_contracts.rs`: voice and verified-clip MP4 persistence and legacy compatibility.

- Create `apps/desktop-ui/src/lib/mediaExport.ts`: format labels, image-selection state, and shared output-player/download helpers.
- Create `apps/desktop-ui/src/lib/mediaExport.test.ts`: failing-first tests for format/image behavior.
- Modify `apps/desktop-ui/src/types.ts`: media formats, cover metadata, MP4 output fields, video progress phase, and RADTTS record types.
- Modify `apps/desktop-ui/src/components/RadcastWorkspace.svelte`: MP4 format, project-image controls, video playback/download/delete, and rendering-video status.
- Modify `apps/desktop-ui/src/components/RadtTsWorkspace.svelte`: same controls for voice generation.
- Modify `apps/desktop-ui/src/components/RadtTsToolsWorkspace.svelte`: same controls for verified clips.
- Modify `apps/desktop-ui/src/styles.css`: compact image controls and responsive video player styling consistent with the existing app.
- Modify relevant existing UI tests and `apps/desktop-ui/src/lib/workspaceLayout.test.ts` only where type/phase changes require it.

## Shared Contracts

The desktop boundary normalizes each incoming request as follows:

| Workflow | MP3 | WAV | MP4 |
|---|---|---|---|
| RADcast | existing audio processor and manifest path | existing audio processor and manifest path | final audio to scratch WAV, shared video exporter, one MP4 record |
| RADTTS voice | existing CLI and `outputs.json` | existing CLI and `outputs.json` | CLI to isolated scratch WAV, promote artifacts, shared video exporter, one `media-outputs.json` record |
| RADTTS verified clip | existing clip path and file scan | existing clip path and file scan | extracted clip to scratch WAV, promote boundary/timed artifacts, shared video exporter, one `media-outputs.json` record |

The new RADTTS durable record has `id`, `kind` (`voice` or `clip`), `name`, `primary_path`, `media_format`, optional `audio_source_path`, optional `image_path`, optional `duration_seconds`, `artifacts`, and `warnings`. Voice and clip list commands adapt these records into their existing response shapes, with `media_format` exposed to the UI; legacy `outputs.json` and MP3/WAV file scans are merged without rewriting them. Delete commands remove only records and files contained in the owning project roots, and never remove a reusable project cover.

Every MP4 job follows this promotion order: validate/copy image into managed storage; create isolated scratch audio and artifacts; render and probe `.partial.mp4`; promote the MP4 and output-scoped image; promote captions/boundary/timed artifacts; atomically write the durable manifest; then delete scratch. Any failure or cancellation before manifest commit removes staged files and writes an orphan-cleanup path if rollback fails. This order prevents a durable record from pointing to a deleted scratch artifact.

---

### Task 1: Add the media-format contract without changing audio-engine formats

**Files:**
- Create: `crates/radsuite-desktop/src/media_export.rs`
- Modify: `crates/radsuite-desktop/src/lib.rs`
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `crates/radsuite-desktop/src/radt_ts.rs`
- Modify: `crates/radsuite-desktop/src/radt_ts_tools.rs`
- Modify: `apps/desktop-ui/src/types.ts`
- Test: `crates/radsuite-desktop/tests/media_export_contracts.rs`

- [ ] **Step 1: Write failing Rust serialization tests** for `MediaOutputFormat::Mp3/Wav/Mp4`, legacy MP3/WAV JSON, and conversion to the existing audio-only enums.
- [ ] **Step 2: Run the focused test** with `cargo test -p radsuite-desktop --test media_export_contracts media_format -- --nocapture`; expect failure because the type and test module do not exist.
- [ ] **Step 3: Implement the minimal desktop media enum** with snake-case serde names, `extension()`, `is_audio()`, and `audio_format()`; keep `radsuite_engines::AudioOutputFormat` and CLI `RadtTsOutputFormat` limited to MP3/WAV.
- [ ] **Step 4: Define the wire migration explicitly**: add optional `media_format` to desktop requests/settings, retain legacy `output_format` as the fallback, prefer `media_format` when present, default missing values to MP3, and normalize once at the command boundary. MP3/WAV normalize to the corresponding audio enum; MP4 always normalizes its intermediate audio format to WAV and is never passed to an audio-only processor or RADTTS CLI.
- [ ] **Step 5: Add the enum to desktop requests/output records** while keeping old serialized fields readable through the precedence/default rules above; add tests for all nine workflow/format combinations (RADcast, RADTTS voice, verified clip × MP3/WAV/MP4).
- [ ] **Step 6: Add matching TypeScript unions and type fields** without changing default format from MP3.
- [ ] **Step 7: Re-run the focused Rust and TypeScript tests** and commit: `git add ... && git commit -m "feat: add desktop media format contract"`.

### Task 2: Build the cancellable process runner and exact video exporter

**Files:**
- Create: `crates/radsuite-engines/src/process.rs`
- Create: `crates/radsuite-engines/src/video.rs`
- Modify: `crates/radsuite-engines/src/lib.rs`
- Modify: `crates/radsuite-engines/src/audio.rs`
- Modify: `crates/radsuite-engines/Cargo.toml`
- Create: `crates/radsuite-engines/tests/video.rs`
- Modify: `crates/radsuite-engines/tests/audio.rs`

- [ ] **Step 1: Write failing tests** for exact video arguments: image input 0 with loop/framerate, audio input 1, 1280x720 cover crop, 1280x220 transparent white waveform over the 70% black band, `fps=30`, stream maps, H.264 CRF 23/yuv420p, AAC 192k, `-shortest`, `+faststart`, and `-progress pipe:1 -nostats`.
- [ ] **Step 2: Write failing ffprobe-fixture tests** accepting exactly one H.264/yuv420p 1280x720 30fps video stream plus one AAC stream and rejecting duplicates, wrong dimensions/codecs/frame rate, invalid duration, and duration mismatch over 0.10 seconds.
- [ ] **Step 3: Run `cargo test -p radsuite-engines --test video`; expect failure before implementation.
- [ ] **Step 4: Add engine dependencies**: `serde_json` and `libc` workspace dependencies, plus target-specific `windows-sys` Job Object/Threading features. Keep the runner synchronous and independent of Tokio; the existing desktop Tokio process-group helper remains for long-running RADTTS CLI jobs.
- [ ] **Step 5: Implement the runner** using spawned children, non-blocking output readers, cancellation polling, Unix process groups, and Windows process-group/job termination; never interpolate paths through a shell. Use temporary executable scripts in tests to exercise cancellation and streamed progress.
- [ ] **Step 6: Implement `VideoExporter`** with fixed constants and the exact filter graph from the approved design, plus deterministic argument builders for unit tests.
- [ ] **Step 7: Refactor `AudioProcessor`** so its current methods delegate to the runner with a no-op cancellation callback, while cancellation-aware methods are available to RADcast/RADTTS.
- [ ] **Step 8: Run engine tests** with `cargo test -p radsuite-engines`; verify all existing MP3/WAV tests pass and commit: `git add ... && git commit -m "feat: add cancellable MP4 video exporter"`.

### Task 3: Add shared project-image storage and transactional media paths

**Files:**
- Create: `crates/radsuite-desktop/src/media_assets.rs`
- Modify: `crates/radsuite-desktop/src/media_export.rs`
- Modify: `crates/radsuite-desktop/src/lib.rs`
- Create: `crates/radsuite-desktop/tests/media_export_contracts.rs`

- [ ] **Step 1: Write failing storage tests** for the canonical root `<data_dir>/media/projects/<project_id>/`, PNG/JPEG/WebP acceptance, non-regular/unsupported/>50 MB rejection, reusable cover metadata, and per-export override preparation.
- [ ] **Step 2: Write failing transaction tests** for output-scoped image retention, rollback cleanup, orphan-path recording, and reusable cover preservation.
- [ ] **Step 3: Run `cargo test -p radsuite-desktop --test media_export_contracts media_assets -- --nocapture`; expect failure.
- [ ] **Step 4: Implement `ProjectMediaStore`** with contained-path checks, managed copy names, metadata JSON, UUID scratch folders, output-scoped image copies, and actionable error messages.
- [ ] **Step 5: Implement atomic manifest/orphan helpers** used by both workflows; only complete records are listable, and startup cleanup retries partial/orphan paths.
- [ ] **Step 6: Re-run the focused tests** and commit: `git add ... && git commit -m "feat: add shared project media storage"`.

### Task 4: Integrate RADcast MP4 rendering and lifecycle

**Files:**
- Modify: `crates/radsuite-desktop/src/radcast.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `crates/radsuite-desktop/tests/radcast_contracts.rs`
- Modify: `crates/radsuite-desktop/tests/media_export_contracts.rs`

- [ ] **Step 1: Add failing RADcast tests** for MP3/WAV unchanged behavior, MP4 using an isolated WAV, final MP4-only record, managed image metadata, caption preservation, and exact cleanup on success/failure/cancellation.
- [ ] **Step 2: Add failing progress tests** requiring `RenderingVideo` between audio/caption work and manifest persistence, with 90-99% FFmpeg progress and 100% only after save.
- [ ] **Step 3: Add failing deletion/listing tests** for MP4 output deletion, output-scoped image removal, project-cover retention, and legacy manifest deserialization.
- [ ] **Step 4: Run focused tests** with `cargo test -p radsuite-desktop --test radcast_contracts`; expect failures.
- [ ] **Step 5: Implement the MP4 branch**: render final audio as scratch WAV, generate captions against that final audio, invoke the shared exporter, commit the output/image metadata atomically, then remove scratch files.
- [ ] **Step 6: Preserve the current MP3/WAV branch** and current enhancement/filter arguments; only MP4 uses the new video stage.
- [ ] **Step 7: Add safe output deletion and Tauri command wiring** without changing source deletion semantics.
- [ ] **Step 8: Re-run focused desktop tests** and commit: `git add ... && git commit -m "feat: add RADcast MP4 exports"`.

### Task 5: Integrate RADTTS voice-generation MP4 output

**Files:**
- Modify: `crates/radsuite-desktop/src/radt_ts.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Create/modify: `crates/radsuite-desktop/tests/radt_ts_contracts.rs`

- [ ] **Step 1: Write failing tests** proving normal MP3/WAV CLI arguments are unchanged and MP4 uses an isolated scratch project, scratch WAV, and no temporary `outputs.json` record.
- [ ] **Step 2: Write failing listing tests** for `manifests/media-outputs.json` MP4 voice records merged with legacy `outputs.json` records. Define the response mapping explicitly: `RadtTsAudioOutput` gains `media_format`, optional `image_path`, and existing caption paths; the listing maps `kind=voice` media records into that type while retaining legacy audio records.
- [ ] **Step 3: Write failing deletion/rollback tests** for voice MP4 and output-scoped image cleanup.
- [ ] **Step 4: Run `cargo test -p radsuite-desktop --test radt_ts_contracts`; expect failures.
- [ ] **Step 5: Implement media-aware RADTTS requests** and map MP4 to the existing CLI's WAV audio output.
- [ ] **Step 6: Add `RenderingVideo` to the RADTTS voice job phases**, pass the job cancellation probe into the shared exporter, map FFmpeg progress to 90-99%, and ensure cancellation wins any race before manifest commit.
- [ ] **Step 7: Promote artifacts before deleting scratch**: copy captions and any other CLI-generated artifacts needed by the durable record into the project output/artifact directory, validate contained paths, then write `media-outputs.json` atomically. Only after promotion and manifest commit may the isolated CLI workspace be deleted.
- [ ] **Step 8: Run the shared video exporter after successful synthesis**, persist one MP4 record with captions and source relationship, and remove the isolated CLI workspace only after the promotion step succeeds.
- [ ] **Step 9: Merge legacy and new listings and expose deletion commands**, then run focused tests and commit: `git add ... && git commit -m "feat: add RADTTS voice MP4 exports"`.

### Task 6: Integrate RADTTS verified-clip MP4 output

**Files:**
- Modify: `crates/radsuite-desktop/src/radt_ts_tools.rs`
- Modify: `crates/radsuite-desktop/src/commands.rs`
- Modify: `apps/desktop-ui/src-tauri/src/main.rs`
- Modify: `crates/radsuite-desktop/tests/radt_ts_contracts.rs`

- [ ] **Step 1: Write failing tests** for MP3/WAV clip behavior, MP4 clip rendering from the extracted clip audio, boundary-report/timed-segment artifact retention, and MP4 records in the merged listing. Define the response mapping explicitly: `RadtTsMediaOutput` gains `media_format`, optional `image_path`, and durable artifact paths; legacy MP3/WAV file scans remain supported.
- [ ] **Step 2: Write failing cancellation and cleanup tests** for clip video rendering, `RenderingVideo` progress from 90-99%, cancellation races, and scratch-file removal.
- [ ] **Step 3: Run the focused RADTTS tests** and verify failure.
- [ ] **Step 4: Add `RenderingVideo` to the verified-clip phases**, pass cancellation/progress into the exporter, and make cancellation win before the record is committed.
- [ ] **Step 5: Promote boundary reports and timed-segment artifacts into durable project storage before deleting any scratch workspace**, then implement MP4 as a post-clip mux step using the same project-image store and `media-outputs.json` schema; leave transcription-only actions unchanged.
- [ ] **Step 6: Add output deletion and rerun desktop tests**, then commit: `git add ... && git commit -m "feat: add RADTTS verified clip MP4 exports"`.

### Task 7: Add shared UI state and testable format/image helpers

**Files:**
- Create: `apps/desktop-ui/src/lib/mediaExport.ts`
- Create: `apps/desktop-ui/src/lib/mediaExport.test.ts`
- Modify: `apps/desktop-ui/src/types.ts`

- [ ] **Step 1: Write failing Vitest tests** for format labels/extensions, MP4 image-required behavior, image override vs project-cover-save behavior, video/audio player selection, and output download filters.
- [ ] **Step 2: Run `npm test -- --run src/lib/mediaExport.test.ts`; expect failure.
- [ ] **Step 3: Implement pure helpers** so Svelte components share one format/image/output contract and do not duplicate branching logic.
- [ ] **Step 4: Run the focused test and `npm run check`; commit: `git add ... && git commit -m "feat: add media export UI helpers"`.

### Task 8: Add MP4 controls and playback to all three workspaces

**Files:**
- Modify: `apps/desktop-ui/src/components/RadcastWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadtTsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/components/RadtTsToolsWorkspace.svelte`
- Modify: `apps/desktop-ui/src/styles.css`
- Modify: relevant UI tests

- [ ] **Step 1: Add failing component/helper tests** for MP4 revealing image controls, audio-only formats hiding image requirements, saved-cover reuse, per-export override, and correct output player/download/delete actions.
- [ ] **Step 2: Run the focused UI tests and capture the expected failures.**
- [ ] **Step 3: Add the MP4 option and image chooser** using the existing Tauri dialog plugin; call shared cover commands and send image/save-default fields only for MP4.
- [ ] **Step 4: Add conditional `<video controls>` playback** for MP4 and retain `<audio controls>` for MP3/WAV; use existing save-dialog download behavior with MP4 filters.
- [ ] **Step 5: Add clear rendering-video progress text and output deletion actions** without adding decorative UI clutter.
- [ ] **Step 6: Add compact responsive styling**, run `npm run check`, `npm test -- --run`, and `npm run test:style`; commit: `git add ... && git commit -m "feat: add MP4 controls to RADsuite workspaces"`.

### Task 9: Wire startup cleanup, acceptance smoke, and regression verification

**Files:**
- Modify: `crates/radsuite-desktop/src/state.rs` or startup initialization location
- Modify: `apps/desktop-ui/src/App.svelte` only if startup cleanup/status needs UI refresh
- Modify: `crates/radsuite-desktop/tests/real_course_smoke.rs`
- Modify: `docs/superpowers/specs/2026-08-14-radcast-radtts-mp4-export-design.md` only if implementation discovers a necessary contract correction

- [ ] **Step 1: Write failing startup-cleanup test** for partial/orphan media paths not appearing in listings.
- [ ] **Step 2: Implement cleanup invocation** at desktop initialization and verify it is safe when no media directory exists.
- [ ] **Step 3: Run Rust verification**: `cargo fmt --all -- --check`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- [ ] **Step 4: Run UI verification**: `npm run check`, `npm test -- --run`, `npm run build`, and `npm run test:style`.
- [ ] **Step 5: Run real acceptance smoke** with a short lecture clip and 16:9 PNG: RADcast Optimized MP4, RADTTS voice MP4, verified clip MP4, stream/duration probe, waveform/image playback, output download, output deletion, and cancellation.
- [ ] **Step 6: Review `git diff`, confirm no MP3/WAV regression, and commit the final verified slice: `git add ... && git commit -m "test: verify MP4 media exports"`.

## Execution Notes

- Follow @superpowers:test-driven-development: every production change begins with a failing focused test.
- Use @superpowers:verification-before-completion before claiming the feature is complete.
- Keep the approved RADcast enhancement filters and RADTTS models unchanged; MP4 is only a post-processing container/output option.
- Do not bump the application version or publish installers in this feature plan unless a separate release task is requested after acceptance.
