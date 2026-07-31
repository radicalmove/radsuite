# RADTTS Local Bridge Design

## Status

Approved under the standing RADsuite migration instruction to continue toward feature parity. This is the first local RADTTS slice after the RADcite document-management work was merged.

## Goal

Replace the disabled RADsuite `Voice generation` navigation item with a usable local-first reference-voice synthesis workflow while preserving the existing RADTTS model, audio tuning, and job orchestration. The Rust desktop shell owns the user-facing contract and launches the existing RADTTS CLI directly; no network service is introduced in this slice.

## Scope

The first slice includes:

- local RADTTS CLI discovery using an explicit environment override, PATH lookup, and the conventional `~/RADTTS/.venv/bin/radtts` installation;
- a project-scoped reference-voice synthesis workflow;
- explicit voice-clone authorization before reference audio is used;
- fast/high-quality model selection, sentence/single chunking, pause range, and MP3/WAV output;
- the captions produced by the current RADTTS CLI, surfaced as downloadable output artifacts rather than a toggle;
- one active synthesis per project, an indeterminate running state, cancellation, failure reporting, and child-process cleanup;
- persisted generated-output discovery from the RADTTS project manifest;
- native Svelte controls for script text, reference audio, settings, progress, playback, and downloads;
- contract tests for request validation, executable discovery, argument construction, output parsing, and job state transitions.

Built-in voices, filler insertion, remote workers, network sync, account sharing, a Rust-native TTS model, and Windows process-tree management remain subsequent slices. This boundary is intentional: the current RADTTS CLI already supports the reference-voice path and its carefully tuned model settings, while the built-in voice path currently exists only in the separate API/UI contract. This first bridge is supported on macOS and Linux, where the process-group cleanup contract is implemented; the UI keeps Voice generation unavailable on Windows until a Job Object adapter is added.

## Local runtime boundary

RADsuite invokes `radtts` as a direct child process with no shell interpolation. The command receives an app-local RADTTS project root under RADsuite's local application data directory. No listener, HTTP client, or externally reachable service is required.

The executable search order is:

1. `RADSUITE_RADTTS_CLI`, when set;
2. `radtts` available on PATH;
3. `$HOME/RADTTS/.venv/bin/radtts`.

Capability discovery returns a plain-language status and the resolved executable path. Missing runtime support disables synthesis with an actionable message rather than failing at process launch.

## Data flow and lifecycle

1. The UI requests capabilities and lists outputs for the selected RADsuite project.
2. The user supplies text and selects a reference audio file.
3. Rust validates the request and ensures the RADTTS project directory and manifest folders exist.
4. Rust rejects a second active job for the same project, starts `radtts synthesize` with a secure temporary text file, and records the child process handle.
5. Because the current CLI emits structured JSON only at completion, the desktop job reports `starting` and then an indeterminate `running` state; it does not invent percentage progress.
6. Rust drains stdout and stderr with bounded buffers while polling the child. The final stdout JSON is parsed as the explicit CLI result contract:
   `{"job_id": string, "status": string, "stage": string, "outputs": object}`.
7. On completion, Rust reads the output metadata and returns only output/caption paths under the selected RADTTS project root. On cancellation or failure it terminates and reaps the process tree.
8. The UI displays the generated audio and captions using Tauri file URLs and refreshes the persisted output list.

Cancellation is OS-level process termination; the current `radtts job --cancel` command cannot cancel a separate CLI process and is not used. On Unix, the child is started in its own process group and cancellation sends a group signal before force-killing the group after a bounded wait. The first bridge refuses to start on Windows because the equivalent Job Object cleanup is not yet implemented. Tauri app shutdown calls the desktop state's synchronous cancellation hook so active process groups receive termination rather than being abandoned.

## Storage, validation, and safety

- A project identifier is the existing UUID-backed `ProjectId`; it is converted only to its canonical hyphenated string. Output names must be non-empty, at most 80 characters, and contain only ASCII letters, digits, `_`, `-`, or `.`; `.` and `..` are rejected.
- Reference audio is canonicalized, must be a regular non-symlink file, must have a supported audio extension, and must be no larger than 250 MB. The original path is passed as a direct argument and is never copied into the RADTTS project by Rust.
- Temporary text files are created with exclusive creation and restrictive permissions in the app cache directory, and are removed in success, failure, cancellation, and startup-error paths.
- Rust creates the required RADTTS project folders and serializes synthesis per project so Python `jobs.json` and `outputs.json` writes cannot race. Output records are accepted only when their manifest `project_id` and job identity match the active request.
- Output and caption paths are canonicalized, must be regular files, and must remain below the selected RADTTS project root. Symlinks and stale rows outside that root are ignored.
- A failed capability check, unsupported-platform check, process-start error, startup timeout, non-zero exit, malformed JSON result, missing output file, or job timeout produces a user-readable error.
- Cancellation waits a bounded interval for process-group reaping and force-kills when necessary. The desktop state retains active child handles and cancellation flags, and its shutdown hook requests cleanup for every active job.

## Testing

- Rust unit tests cover executable discovery, safe component/path validation, reference-file limits, request validation, argument construction, output parsing, and terminal state mapping without model downloads.
- Desktop contract tests use a deterministic fake executable that records arguments, writes a known JSON result, supports a controlled delay, emits bounded/unbounded fixture output, and spawns a child on Unix. They cover startup, indeterminate running state, single-flight rejection, cancellation, timeout, non-zero exit, malformed output, descendant cleanup, temporary-file cleanup, output-root filtering, shutdown cleanup, and unsupported-platform handling.
- Frontend tests cover required reference-audio validation, disabled states, indeterminate progress rendering, and output actions.
- Existing workspace Rust and desktop UI checks remain required.
- A real local capability smoke is allowed when the RADTTS venv is present; a real synthesis smoke remains optional and gated on model assets.

## Future replacement

The UI-facing request and output types do not expose Python-specific details such as CLI flags or model cache paths. A later Rust-native engine can implement the same adapter contract and retain the current Svelte workflow. A subsequent RADTTS adapter can add built-in voices without reopening project navigation or output handling.
