# RADTTS Reference Transcript Passthrough Design

## Goal

Carry an optional voice-reference transcript from the RADsuite RADTTS workspace to the local RADTTS CLI. This restores the original CLI's `--reference-text-file` capability for voice-cloned synthesis without sending transcript content directly in process arguments.

## User experience

Add an optional `Reference transcript` text area beside the reference voice audio field. The transcript is project-scoped in the existing RADTTS browser preferences, is restored when the project is reopened, and does not affect the start validation because the original voice sample remains usable without a transcript.

The existing permission acknowledgement, quality, chunking, pause, and output controls remain unchanged. The UI explains that the transcript can improve pronunciation and timing for the supplied reference voice.

## Data flow

1. Svelte adds the trimmed transcript to the existing RADTTS request as `reference_text`, or `null` when blank.
2. The Tauri command deserializes the optional field with a default so older callers remain compatible.
3. The Rust bridge writes non-empty transcript text to a uniquely named file in the RADTTS cache directory.
4. The child process receives `--reference-text-file <path>` alongside the existing `--reference-audio` and `--text-file` arguments.
5. The bridge removes both temporary text files after successful completion, failure, cancellation, spawn failure, or validation failure.

Reference transcript content is never persisted in the project manifest or passed as a command-line value. Existing reference audio path validation and project output containment remain unchanged.

## Error handling

Failure to create either temporary file returns the existing I/O error and removes any earlier temporary file. Failure to spawn or finish a job follows the current RADTTS job-state behavior and removes the transcript file before releasing the project's active-job lock. Blank transcript input skips temporary-file creation and the CLI flag entirely.

## Testing

- Frontend workflow tests verify blank transcripts map to `null` and non-empty transcripts are trimmed and included.
- Rust argument tests verify the optional flag is present only when a transcript file is supplied.
- Rust job tests verify temporary transcript files are removed after the fake CLI completes and after failure paths.
- Existing workspace, desktop contract, and frontend checks must remain green.

## Deliberate non-goals

This slice does not add built-in speaker selection, remote workers, model download management, or a Rust-native TTS engine. Those remain separate parity slices because the current local RADTTS CLI already supports reference-voice synthesis but does not expose all of the original API's built-in-voice workflow through this bridge.
