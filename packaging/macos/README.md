# macOS Packaging

## Target

- macOS 14+
- Apple Silicon only for the first release
- Tauri app bundle from `apps/desktop-ui/src-tauri`

## Signing And Notarisation

Production builds will require:

- Apple Developer ID Application certificate
- Hardened runtime enabled through Tauri bundling configuration
- Notarisation submission before distribution outside local development machines

Internal alpha builds can remain unsigned while testing locally, but any wider distribution should use signed and notarised bundles.

## Sidecars

Native sidecars should live under the Tauri resource directory and be selected through `radsuite-engines`. Expected sidecars include:

- `ffmpeg`
- ASR runtime binary
- Audio cleanup runtime binary
- TTS runtime binary, if local TTS is available without Python

## Data Directories

The desktop crate resolves app data directories through the `directories` crate. macOS data should resolve under the user's Library application support area.
