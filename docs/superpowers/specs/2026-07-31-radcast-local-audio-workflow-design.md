# RADcast Local Audio Workflow Design

## Status

Approved under the standing RADsuite migration instruction to make RADcast the primary next workstream.

## Context

RADsuite currently exposes RADcast only as a disabled sidebar placeholder. The original RADcast workflow lets users select or reuse project audio, preview it, trim a non-destructive working range, choose an output format, clean or enhance the audio, watch progress, and reuse completed versions. The existing Python enhancement model is not available in the current Mac environment because its virtual environment does not contain Torch.

## Goal

Deliver a usable local RADcast foundation in the Rust desktop app now, without blocking on the model runtime. Users should be able to import audio into a project, preview it, trim and clean it with installed FFmpeg, save a new output without overwriting the source, and play or download completed outputs. The future RADcast model engine must fit behind the same processing boundary.

## Design

### Engine boundary

Add an audio processor to `radsuite-engines`. It validates input and clip ranges, probes duration with `ffprobe`, builds an FFmpeg command, runs the command, and probes the resulting duration. Output formats are MP3 and WAV. The first local cleanup profile uses FFmpeg filters for speech-oriented high-pass, low-pass, noise reduction, and loudness normalization. A later RADcast model implementation can replace or extend this processor without changing the desktop command or UI contract.

### Project storage

Store RADcast files under the RADsuite application data directory in a project-specific folder. Imported source files are copied into a `sources` directory and output versions into an `outputs` directory. A small JSON manifest preserves source names, output names, durations, processing settings, and creation times. Source files are never overwritten by processing.

### Desktop commands

Add commands to list project audio, import a source audio file, and process a selected source. The commands resolve the selected RADsuite project before touching the project folder. Processing runs off the UI thread and returns a completed output summary with a local file path suitable for Tauri asset playback.

### User interface

Enable the existing RADcast project navigation item and replace its placeholder with a focused workspace. The workspace contains:

- source audio picker and saved-source list
- native audio preview for the selected source
- start/end trim controls constrained to the source duration
- MP3/WAV output selection
- a clearly labelled local cleanup option
- processing status and error feedback
- completed output cards with playback and download links

The UI will not present unavailable model controls as if they work. The model engine remains a follow-on capability behind the processor boundary.

### Error handling

Invalid paths, unsupported output formats, missing project/source IDs, invalid trim ranges, missing FFmpeg tools, failed FFmpeg commands, and failed manifest writes return user-readable command errors. A failed process does not create a completed output entry.

## Testing

- Engine unit tests cover trim-range validation, output extension selection, cleanup filter construction, and command argument construction.
- Desktop contract tests cover source import, project isolation, processing through a deterministic fake processor boundary, output listing, and invalid source handling.
- UI verification uses the existing Svelte test suite, `svelte-check`, and production build.
- Full workspace tests, clippy, formatting, and packaged app rebuild remain required before merge.

## Scope Boundary

This slice does not implement Torch, Resemble Enhance, DeepFilterNet, speech recognition, filler-word removal, captions, remote helpers, or waveform rendering. Those capabilities will reuse the same source/output and processing contracts once the local foundation is usable.

