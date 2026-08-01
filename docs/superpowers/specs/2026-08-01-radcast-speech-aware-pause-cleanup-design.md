# RADcast Speech-Aware Pause Cleanup Design

## Status

Approved as the next RADsuite parity slice under the standing migration instruction.

## Context

The original RADcast cleanup workflow uses local speech timestamps to identify spoken regions. When pause reduction is enabled, it removes only the part of a leading, inter-word, or trailing gap beyond the user's selected maximum. Filler-word removal uses the same transcript and removes the detected filler spans before the remaining audio is rendered.

RADsuite currently exposes the pause slider but sends the value to FFmpeg's generic `silenceremove` filter. That filter makes decisions from audio energy rather than speech timing, so it can trim room tone or quiet speech and does not share the original workflow's boundary decisions.

## Goal

Make RADsuite's pause reduction behave like the original RADcast workflow while retaining local processing, the existing Rust desktop command surface, and the existing FFmpeg output renderer.

## Design

### Shared cleanup plan

Add a pure cleanup-planning boundary in `radsuite-engines` that accepts timestamped `CaptionWord` values, total duration, the selected maximum pause duration, and filler-removal mode. It returns merged removal intervals plus separate pause and filler counts.

The planner will:

- merge touching speech-word intervals using the original 60 ms tolerance;
- exclude recognized filler words from the speech timeline when filler removal is enabled;
- remove only the portion of a gap after the configured keep duration;
- apply the same rule to leading and trailing gaps;
- treat gaps shorter than 350 ms as non-compaction candidates, matching the original minimum;
- merge pause and filler intervals before rendering;
- clamp and reject non-finite or negative timing values rather than generating unsafe FFmpeg arguments.

The existing filler heuristics remain the source of truth for filler interval detection. A single transcription pass will supply words for both pause and filler planning when either feature is enabled.

### Desktop processing

When pause reduction or filler removal is requested, `radsuite-desktop` will ask `CaptionProcessor` for fast word timestamps, build the shared cleanup plan, and pass the merged intervals to `AudioProcessor`. The generic `silenceremove` filter will not be used for this speech-aware path. Enhancement preparation remains unchanged, and pause/filler planning will happen against the selected clip before enhancement output is rendered.

The output manifest will record the number of shortened pauses in addition to the existing filler count. Existing manifests deserialize with a zero default. If local caption support is unavailable, the UI will explain that speech-aware pause and filler cleanup require the local transcription runtime.

### Rendering

FFmpeg remains responsible for output format conversion, trimming, and concatenating kept spans. This avoids introducing a second Rust waveform buffer pipeline while preserving source immutability and the current MP3/WAV output contract. A short crossfade is not introduced in this slice; the planner and interval boundaries are the parity-critical behavior, and crossfade can be added later with a separate rendering test.

## Error handling

- Missing Whisper runtime or model returns the existing local-caption capability error before rendering begins.
- Invalid timestamps produce a typed cleanup-planning error or an empty safe plan, never invalid FFmpeg intervals.
- A failed cleanup transcription removes temporary files and does not add a completed output manifest entry.
- Existing cancellation checks remain active before transcription, after planning, and before final manifest persistence.

## Testing

- Pure engine tests cover speech interval merging, leading/inter-word/trailing pause shortening, the 350 ms threshold, filler exclusion, overlap merging, and invalid timing handling.
- Caption processor tests cover the shared cleanup-plan request and clip-relative timing.
- Desktop contract tests verify that pause cleanup uses interval rendering rather than `silenceremove`, persists pause counts, and leaves output/source isolation intact.
- Existing real local RADcast smoke coverage remains the final runtime check when the local model and helper assets are present.

## Scope boundary

This slice does not change the RADcast Optimized enhancement model, caption quality review, remote/cloud processing, or waveform UI. It focuses on making the new pause slider produce the same speech-aware cleanup decisions as the original application.
