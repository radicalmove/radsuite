# Windows Packaging

## Current Status

RADsuite has an initial Windows 11 x64 installer build. The workflow at
`.github/workflows/windows-installer.yml` builds both formats:

- NSIS `.exe`, suitable for the normal guided installation flow
- Windows Installer `.msi`, suitable for managed or enterprise deployment

Run the workflow manually from GitHub Actions, or push a version tag matching
`v*`. The workflow uploads both installers and a `SHA256SUMS.txt` checksum file
as the `RADsuite-windows-installers` artifact. The workflow also runs the Rust
workspace checks and fails if it does not produce exactly one MSI and one NSIS
installer. This is an internal alpha packaging path, not yet a signed public
release.

## Release Checklist

Before distributing a Windows build outside controlled testing:

- Test installation, launch, upgrade, and uninstall on a clean Windows 11 x64 machine.
- Confirm the app can create and reopen its local projects under Windows AppData.
- Test RADcite import, review, readings, exports, and local backup behavior.
- Test RADcast processing with the runtime bundled or installed for Windows.
- Add the Windows RADTTS process-management adapter before advertising RADTTS support.
- Obtain a Windows code-signing certificate and sign the installer and bundled binaries where practical.
- Test Defender SmartScreen behavior with a signed build.
- Decide whether the public distribution should offer NSIS, MSI, or both.
- Publish checksums with every external installer release and document the
  verification command for users.

## Runtime Scope

The installer packages the Tauri application. It does not currently bundle every
Python/model runtime used by the local audio and voice tools. RADcast and RADTTS
must therefore be treated as separate Windows runtime work until their helper
discovery, dependencies, cancellation, and quality have been verified on a real
Windows machine. RADTTS currently keeps its voice-generation workflow disabled
on Windows because process-tree cleanup still needs a Windows Job Object adapter.

Native sidecars should be bundled with the app and discovered through
`radsuite-engines` once their Windows builds are available. Runtime selection
should account for CPU fallback, DirectML where useful, and CUDA where present
and supportable.

## Data Directories

The desktop crate resolves app data directories through the `directories` crate.
Windows data should resolve under the user's application data area and should
not require administrator permissions.

## Signing

Production builds will require:

- Windows code-signing certificate
- Signed installer
- Signed sidecar binaries where practical
- Defender SmartScreen testing before external release
