# RADTTS Pause Seed Design

## Goal

Expose the original RADTTS CLI's optional `--pause-seed` control in the RADsuite voice-generation workspace. A seed makes sentence-pause choices repeatable across runs while leaving the current random behavior unchanged when no seed is supplied.

## User experience

Add an optional `Pause pattern seed` integer field to the existing RADTTS settings panel. The empty state means natural variation on every run. Entering an integer repeats the same pause pattern for the same script and settings. The field is project-scoped through the existing RADTTS preference storage.

The control is deliberately optional and text-backed at the workflow boundary so clearing it reliably removes the CLI flag. The UI accepts safe integers, including negative values supported by the Python CLI, and ignores non-integer or unsafe values when building a request.

## Data flow

1. The Svelte draft and the `voice.pauseSeed` project preference store an optional seed as a string, preserving the empty input state.
2. The frontend normalizes the value to `number | null` and maps it to `pause_seed`.
3. The Tauri request deserializes `pause_seed` as `Option<i64>`, defaulting to `None` for older JSON callers.
4. The Rust argument builder appends `--pause-seed <value>` only when a seed is present.

Existing pause minimum/maximum, chunking, quality, reference voice, transcript, output, and generation-budget behavior remains unchanged. The value is passed as a separate argument, not interpolated into a shell command.

## Error handling

The frontend omits invalid, fractional, non-finite, or unsafe seed values. Missing, whitespace-only, or malformed saved preferences fall back to a blank field; malformed live input is shown until the request is built, then maps to `pause_seed: null`. Rust receives only an integer option and therefore needs no additional range validation. An absent or blank value preserves the original random-seed behavior.

## Testing

- Frontend workflow tests verify blank, valid, negative, fractional, and unsafe seed handling, request mapping, and project preference persistence.
- Rust argument tests verify the optional flag is present when supplied, absent when omitted, and legacy JSON defaults to `None`.
- Existing frontend and workspace Rust checks must remain green.

## Deliberate non-goals

This slice does not change pause duration controls, add built-in voices, expose raw model IDs, or modify the separate RADTTS repository.
