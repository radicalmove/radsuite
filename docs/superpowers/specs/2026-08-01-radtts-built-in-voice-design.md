# RADTTS Built-In Voices

## Goal

Expose the built-in CustomVoice path already supported by the original RADTTS
engine in the Rust desktop workflow, while preserving the existing authorised
reference-voice path.

## Scope

Two repositories are involved:

- RADTTS gains the missing CLI flags and maps them into its existing
  `SynthesisRequest` model: `--voice-source`, `--built-in-speaker`, and
  `--built-in-instruct`. Reference audio remains required by validation only
  when the source is `reference`; built-in mode chooses the CustomVoice model
  alias from the existing fast/high mode.
- RADsuite gains a project-scoped voice-source choice. Reference mode keeps the
  current audio picker, transcript, and permission acknowledgement. Built-in
  mode shows a fixed list matching `QWEN_CUSTOM_VOICE_SPEAKERS` and an optional
  style instruction, and hides controls that do not apply.

No voice preview endpoint is added to the local desktop bridge in this slice;
the synthesis path is the priority. Preview support can use the same model
selection later without changing this request contract.

## Data Flow

The desktop request carries `voice_source`, optional `reference_audio_path`,
optional `built_in_speaker`, and optional `built_in_instruct`. Rust validates
the selected mode before spawning RADTTS. It emits `--voice-source builtin`
and built-in arguments only for built-in mode; reference mode keeps the
existing safe path and optional reference transcript temp file.

Project preferences persist the selected voice source, speaker, and
instruction. The permission acknowledgement is never persisted and is
required only for reference voice cloning.

## Error Handling

- Built-in mode requires a supported speaker and does not require reference
  audio or voice-clone acknowledgement.
- Reference mode continues to require a regular supported audio file and
  explicit permission acknowledgement.
- Missing or incompatible CLI support produces a clear capability/error message
  rather than silently falling back to reference mode.
- Existing output containment and job cancellation rules remain unchanged.

## Testing

- RADTTS tests cover CLI parsing and request construction for both modes, with
  reference mode remaining backwards compatible.
- RADsuite frontend tests cover draft defaults, mode-specific start validation,
  request construction, and preference persistence.
- Rust tests cover argument construction, legacy JSON defaults, and validation
  of required fields per voice source.
- Run RADTTS tests, desktop UI tests/check/build, Rust clippy/workspace tests,
  and CI for both repositories before merging.
