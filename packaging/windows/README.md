# Windows Packaging

## Target

- Windows 11 x64
- Tauri installer from `apps/desktop-ui/src-tauri`

## Installer And Signing

Installer format still needs a final decision. The first implementation should keep Tauri's Windows bundle configuration active, then choose the distribution format after internal alpha testing.

Production builds will require:

- Windows code-signing certificate
- Signed installer
- Signed sidecar binaries where practical
- Defender SmartScreen testing before external release

## Sidecars

Native sidecars should be bundled with the app and discovered through `radsuite-engines`. Windows-specific runtime selection should account for:

- CPU fallback
- DirectML where useful
- CUDA where present and supportable

## Data Directories

The desktop crate resolves app data directories through the `directories` crate. Windows data should resolve under the user's application data area.
