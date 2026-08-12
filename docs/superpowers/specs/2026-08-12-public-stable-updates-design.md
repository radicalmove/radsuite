# Public Stable Updates Design

## Goal

Publish RADsuite through a single public GitHub Releases page and allow installed copies to discover and install signed stable updates for Apple Silicon Mac, Intel Mac, and Windows.

## User-facing distribution

The canonical installation page is:

`https://github.com/radicalmove/radsuite/releases/latest`

Each stable release will contain clearly labelled assets:

- `RADsuite_<version>_Apple-Silicon.dmg`
- `RADsuite_<version>_Intel.dmg`
- `RADsuite_<version>_Windows_x64_Setup.exe`
- `RADsuite_<version>_Windows_x64.msi`
- `RADsuite_<version>_Apple-Silicon.app.tar.gz`
- `RADsuite_<version>_Intel.app.tar.gz`
- `RADsuite_<version>_Windows_x64_Setup.exe.sig`
- `RADsuite_<version>_Windows_x64.msi.sig`
- `latest.json`
- `SHA256SUMS.txt`

The EXE is the recommended Windows download. The MSI remains available for managed or institutional deployment.

## Updater architecture

RADsuite will use the Tauri v2 updater plugin. The application embeds a public verification key and points to the public GitHub Releases `latest.json` asset over HTTPS. GitHub Actions signs updater artifacts with the corresponding private key stored only as a repository Actions secret.

The configured endpoint is:

`https://github.com/radicalmove/radsuite/releases/latest/download/latest.json`

The generated metadata uses Tauri's static platform map:

```json
{
  "version": "0.2.2",
  "notes": "Stable RADsuite update.",
  "pub_date": "2026-08-12T00:00:00Z",
  "platforms": {
    "darwin-aarch64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/radicalmove/radsuite/releases/download/v0.2.2/RADsuite_0.2.2_Apple-Silicon.app.tar.gz"
    },
    "darwin-x86_64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/radicalmove/radsuite/releases/download/v0.2.2/RADsuite_0.2.2_Intel.app.tar.gz"
    },
    "windows-x86_64": {
      "signature": "<base64 signature>",
      "url": "https://github.com/radicalmove/radsuite/releases/download/v0.2.2/RADsuite_0.2.2_Windows_x64_Setup.exe"
    }
  }
}
```

The app will:

- check once after startup;
- check at most once per 24 hours thereafter;
- show the available version and release notes;
- offer `Update now` and `Later`;
- download and verify the platform-specific signed updater artifact;
- install and restart only after the user confirms;
- leave the local SQLite database, project files, generated outputs, Python environments, and model caches in place.

The last-check timestamp and the dismissed version are stored in the existing local browser storage. A failed check does not update the timestamp, so the next launch can retry; selecting `Later` removes the prompt for the current session and allows it to reappear on the next daily check.

Stable releases are the only channel. There is no beta or nightly endpoint in this first updater implementation.

The automatic Windows update target is the NSIS Setup EXE, which is the recommended download for ordinary users. MSI remains available for institutional deployment and is not used as the automatic update target; managed installations should continue to receive MSI updates through the organisation's software deployment process.

## Release automation

The release workflow will run on a `v*` tag after the version in the Rust, Tauri, and frontend manifests has been updated. A first job validates that the tag is valid SemVer and exactly matches all application manifests before any release is created. It uses Tauri's v2 updater artifact mode (`createUpdaterArtifacts: true`), producing macOS `.app.tar.gz` bundles and signed Windows installer artifacts. It will:

1. build the Apple Silicon Mac updater artifact;
2. build the Intel Mac updater artifact;
3. build the Windows NSIS updater artifact and MSI distribution installer;
4. run the existing frontend and Rust verification checks;
5. generate Tauri updater artifacts, metadata, and signatures using the protected signing secret;
6. create a draft GitHub Release, upload and validate all labelled installers, updater artifacts, `latest.json`, release notes, and `SHA256SUMS.txt`, then publish the release only after validation succeeds.

The existing manually-triggered Windows installer workflow remains useful for testing non-release branches. Stable publishing is tag-driven so a release cannot be accidentally replaced by an ordinary branch push.

## Security and failure handling

- The private signing key is never committed to the repository or bundled in the application.
- The signing key is stored as a protected GitHub Actions secret in the stable release environment and kept in an encrypted offline backup. Key rotation requires a new app release containing the replacement public key; the old key remains valid until all supported installations have moved to that release.
- The app rejects unsigned or incorrectly signed update metadata and packages.
- HTTPS is required for the update endpoint.
- Network failures are silent or shown as a non-blocking update-check notice; the current version continues to work.
- Download, disk-space, permission, cancellation, signature, install, and restart failures are caught and shown as a plain-language non-blocking error; the current installation remains usable and the next scheduled check can retry.
- A user can defer an update without losing the prompt permanently; the next daily check can offer it again.
- The updater does not update RADcast/RADTTS Python environments. Those remain independently managed by RADsuite's first-run runtime setup and are preserved across app updates.

The current release does not change the SQLite schema. Future schema changes must use the existing versioned startup migrations and add an upgrade regression test before release. The updater preserves the application data directory; it does not attempt to copy or relocate user files.

RADsuite `0.2.1` predates the updater and cannot discover it. Existing users must install `0.2.2` once from the public Releases page; all later stable releases can then update automatically.

Apple distribution signing/notarization and Windows installer signing are separate from Tauri updater signatures. The first direct-download release will require Tauri updater signatures but will not block publication on platform certificate secrets that are not yet configured. Users may therefore see the normal Gatekeeper or SmartScreen confirmation for the initial direct download. Adding Apple Developer notarization and Windows code signing later will remove those platform warnings without changing the updater endpoint or public installation URL.

The release job has protected `contents: write` permission and is the only job allowed to publish. Build jobs have read-only repository permissions and upload immutable artifacts to the release job. Before building, the workflow verifies that `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are present. After publication, the workflow fetches `latest.json` from the public `releases/latest/download` URL and validates its JSON, version, platform keys, HTTPS URLs, signatures, and referenced release assets. Platform signing can be enabled later by adding the corresponding protected credentials and signing steps.

## Testing

- Unit-test the update-check scheduling decision and user-facing update state.
- Verify the updater plugin permissions and Tauri command registration at build time.
- Validate the `v*` tag, SemVer, manifest versions, platform keys, asset URLs, signatures, and `SHA256SUMS.txt` contents before publishing.
- Run the frontend test/build suite and Rust formatting, clippy, and test suites.
- Build signed artifacts in GitHub Actions for all three platform targets.
- Validate the generated GitHub Release contains the labelled installers, `latest.json`, signatures, and checksums.
- Test update installation against a local/static test endpoint and test key before using the public stable endpoint; this avoids publishing a test version through `releases/latest`.
- Install the stable release on each platform and verify that a subsequent stable release prompts, updates, restarts, and preserves local project data, generated outputs, Python environments, and model caches.
