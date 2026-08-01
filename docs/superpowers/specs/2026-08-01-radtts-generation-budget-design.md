# RADTTS Generation Budget Design

## Goal

Expose the local RADTTS CLI's `--max-new-tokens` control in the RADsuite voice-generation workspace. This lets users trade generation headroom against processing cost and restores a supported synthesis option without changing the local execution model.

## User experience

Add a `Generation budget` range control to the existing voice and settings panel. It spans the RADTTS-supported range of 64 to 8192 tokens and displays the current value in a human-readable form. The default remains 1200 tokens, matching the original CLI. The control is project-scoped through the existing RADTTS preference storage and is available for both Fast and High quality modes.

The control is intentionally a slider rather than a free-form field because it is a bounded numeric setting and should not require users to understand model internals. It uses an explicit step of one token. The nearby note explains that a larger budget helps longer or more complex scripts but may take longer locally.

## Data flow

1. The Svelte draft and project preferences store `maxNewTokens` as a number.
2. The frontend request maps it to `max_new_tokens`.
3. The Tauri request deserializes the value and the Rust bridge validates the supported range.
4. The CLI argument builder adds `--max-new-tokens <value>` to the existing `synthesize` command.

The frontend uses shared constants and helpers for the 64-token minimum, 8192-token maximum, and 1200-token default. The Rust request field is a `u32` and uses the same default when omitted from older JSON requests.

Existing quality, chunking, pause, output, reference-audio, and reference-transcript behavior remains unchanged. The value is passed as a numeric argument, not interpolated into a shell command.

## Error handling

Values below 64 or above 8192 are rejected by the Rust bridge with a clear validation error. The UI clamps slider values to the same range. Older JSON callers receive the default of 1200 when the new field is absent.

## Testing

- Frontend workflow tests verify the request includes the budget, preference storage restores it, and older preferences resolve to 1200.
- Rust argument tests verify `--max-new-tokens` is present and legacy requests without the field default to 1200.
- Rust validation tests cover the inclusive lower and upper bounds as well as rejection outside them.
- Existing frontend and workspace Rust checks must remain green.

## Deliberate non-goals

This slice does not add built-in voices, custom speaker selection, model downloads, pause seeds, or remote workers. Built-in voices require extending the current RADTTS CLI beyond its reference-audio-only command contract and will be handled separately.
