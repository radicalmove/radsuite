# RADsuite Releases

## Public install page

Send users this permanent link:

<https://github.com/radicalmove/radsuite/releases/latest>

Choose the installer for the computer:

- Apple Silicon Mac: `RADsuite_<version>_Apple-Silicon.dmg`
- Intel Mac: `RADsuite_<version>_Intel.dmg`
- Windows x64: `RADsuite_<version>_Windows_x64_Setup.exe`

The MSI is also published for managed or institutional Windows deployment. Ordinary Windows users should use the Setup EXE.

## Updating

RADsuite checks the stable GitHub Releases endpoint after launch and no more than once every 24 hours. When a newer stable version is available, the app shows an `Update now` prompt. The updater verifies the signed package before installing it and keeps the local database, project files, generated outputs, Python environments, and model caches in place.

Version `0.2.2` is the updater bridge release. Users running `0.2.1` must install `0.2.2` once from the public install page. Later stable releases can be discovered inside the app.

## Publishing a stable release

1. Update all application versions together: `apps/desktop-ui/package.json`, `apps/desktop-ui/package-lock.json`, `apps/desktop-ui/src-tauri/Cargo.toml`, `crates/radsuite-desktop/Cargo.toml`, and `apps/desktop-ui/src-tauri/tauri.conf.json`.
2. Run the frontend and Rust verification commands used by CI.
3. Commit and push the version change.
4. Create an annotated tag, for example `git tag -a v0.2.3 -m "RADsuite 0.2.3"`, and push it with `git push origin v0.2.3`.
5. The `Stable release` workflow validates the tag, builds native signed artifacts, creates a draft release, generates `latest.json` and `SHA256SUMS.txt`, validates the assets, and publishes only after the `stable` environment gate succeeds.

## Required GitHub Actions secrets

Keep the Tauri updater private key in an encrypted offline backup and add these required repository/environment secrets without committing them:

- `TAURI_SIGNING_PRIVATE_KEY`
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

These platform-trust secrets are optional for the initial direct-download release and can be added later:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `KEYCHAIN_PASSWORD`
- `WINDOWS_CERTIFICATE`
- `WINDOWS_CERTIFICATE_PASSWORD`

The updater signing secrets are required for every release. Apple and Windows platform-signing secrets are optional in the initial direct-download workflow; without them, macOS Gatekeeper or Windows SmartScreen may show the normal first-download warning. Add the Apple certificate/notarization secrets and Windows certificate secrets later to remove those warnings. The Tauri updater signature still verifies the update package independently.

The Tauri updater key is separate from Apple and Windows platform signing. Rotating it requires a new application release containing the replacement public key.
