# RADcast speech cleanup parity implementation plan

**Goal:** Restore Python-equivalent filler cleanup and verify long-pause shortening.

**Architecture:** Port the pure filler planner to a small Rust module. Configure the existing whisper.cpp transcription boundary for cleanup windows and assemble word tokens. Use bounded FFmpeg crossfades for the existing removal interval renderer.

**Tech stack:** Rust, whisper.cpp CLI, FFmpeg, existing Cargo integration tests.

- [x] Add failing regression cases in `crates/radsuite-engines/tests/captions.rs` for filler variants, context, duration bounds, run counts, neighbouring speech, and pause safety. Run `cargo test -p radsuite-engines --test captions`.
- [x] Implement Python filler heuristics in `crates/radsuite-engines/src/fillers.rs`; integrate into `captions.rs` and verify pure Python/Rust timing parity against 60 saved cases.
- [x] Add failing CLI cases for cleanup prompts, beam/context settings, window ownership, selected clip offsets, token assembly and cancellation. Implement cleanup transcription in `captions.rs` and wire cancellation in `crates/radsuite-desktop/src/radcast.rs`.
- [x] Add failing real waveform tests in `crates/radsuite-engines/tests/audio.rs` for crossfades, tiny retained chunks and selected clip removal. Implement bounded crossfades and clipping in `audio.rs`.
- [x] Update existing expectations only where the Python behaviour intentionally differs. Run engine tests, desktop contracts, formatting and Clippy; review the diff and record evidence.

The user requested direct implementation; proceed in this chat without another approval gate. Keep pre-existing unrelated untracked files intact.

Evidence is recorded in `docs/radcast/cleanup-parity/verification.json` and `installation.json`. Independent review also covered and resolved zero-duration/untimed leading subwords, guarded Studio duration accounting, and pause shortening when every recognised word is an accepted filler. Preserve an original-word timing fallback in that case. The local build remains version 0.2.11 and is not a public release; the running app needs a restart.
