# Public Stable Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship RADsuite `0.2.2` with a stable-only, signed Tauri updater and a public GitHub Releases installation/update path for Apple Silicon Mac, Intel Mac, and Windows x64.

**Architecture:** The Svelte shell owns the non-blocking daily update check and confirmation UI. Tauri's updater and process plugins perform signature verification, download, install, and restart; the updater endpoint is the immutable GitHub Releases `latest.json` URL. A tag-driven GitHub Actions workflow builds native artifacts on three runners, creates the manifest from the produced signatures, validates every asset, and publishes only after required platform-signing and updater-signing checks pass.

**Tech Stack:** Svelte 5, TypeScript, Vitest, Tauri v2, Rust, GitHub Actions, GitHub CLI, Python standard library for release-manifest validation.

---

## File Map

- Modify: `Cargo.toml` to add the Tauri updater and process workspace dependencies.
- Modify: `apps/desktop-ui/src-tauri/Cargo.toml` to enable those plugins for the desktop binary.
- Modify: `apps/desktop-ui/package.json` and `apps/desktop-ui/package-lock.json` to add the matching JavaScript plugin packages.
- Modify: `apps/desktop-ui/src-tauri/src/main.rs` to register updater/process plugins.
- Modify: `apps/desktop-ui/src-tauri/capabilities/default.json` to permit updater and restart operations.
- Modify: `apps/desktop-ui/src-tauri/tauri.conf.json` to set version `0.2.2`, enable updater artifacts, and embed the public verification key and stable endpoint.
- Create: `apps/desktop-ui/src/lib/updateState.ts` for storage-safe 24-hour scheduling and update prompt state transitions.
- Create: `apps/desktop-ui/src/lib/updateState.test.ts` for scheduling, version dismissal, and malformed-storage behavior.
- Create: `apps/desktop-ui/src/lib/updateCommands.ts` for the small testable adapter over the Tauri updater/process APIs.
- Modify: `apps/desktop-ui/src/App.svelte` to check on startup, show a compact update prompt, install only after confirmation, and expose a plain-language error without blocking work.
- Modify: `apps/desktop-ui/src/lib/appVersion.test.ts` or add a focused version test to require `0.2.2` fallback behavior.
- Create: `scripts/build-update-manifest.py` to generate and validate `latest.json` and `SHA256SUMS.txt` from named release assets.
- Create: `scripts/build-update-manifest.test.py` to test platform mapping, signatures, HTTPS URLs, checksums, and missing-asset failures.
- Create: `.github/workflows/release.yml` for tag validation, native builds, signed updater artifacts, draft release creation, manifest validation, and publication.
- Modify: `.github/workflows/windows-installer.yml` only as needed to keep manual branch builds separate from stable release publication.
- Create: `docs/releasing.md` documenting the public install URL, installer selection, updater key setup, required Apple/Windows signing secrets, and the `v0.2.2` bridge-release rule.

### Task 1: Lock the 0.2.2 updater contract

- [ ] Confirm the current dependency versions and Tauri updater API signatures with `cargo tauri --version`, `npm view @tauri-apps/plugin-updater version`, and the existing lockfile before editing manifests.
- [ ] Generate a Tauri updater signing key outside the repository, preserve the private key in an encrypted local backup, and record only its public key for `tauri.conf.json`.
- [ ] Write the public endpoint exactly as `https://github.com/radicalmove/radsuite/releases/latest/download/latest.json` and document that `0.2.1` requires one manual `0.2.2` install.
- [ ] Add a test fixture or script assertion that the manifest keys are `darwin-aarch64`, `darwin-x86_64`, and `windows-x86_64`, with the Windows automatic-update URL pointing to the signed NSIS setup executable.
- [ ] Run the focused manifest/version tests and confirm they fail for the missing implementation before adding the implementation.

### Task 2: Add Tauri updater and process plugins

- [ ] Add `tauri-plugin-updater` and `tauri-plugin-process` to workspace and desktop Rust dependencies, plus `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process` to the frontend dependencies; run `npm install` to update the lockfile.
- [ ] Register `tauri_plugin_updater::Builder::new().build()` and `tauri_plugin_process::init()` in `apps/desktop-ui/src-tauri/src/main.rs` without changing existing command registration or application state.
- [ ] Add the updater public key, stable endpoint, `createUpdaterArtifacts: true`, and version `0.2.2` to `tauri.conf.json`; keep the existing application identifier and resource paths unchanged.
- [ ] Add only the required updater/process capability permissions and verify the generated Tauri capability schema accepts them.
- [ ] Run `cargo check -p radsuite-tauri --all-features` and the frontend type check to catch plugin/configuration errors before UI work.

### Task 3: Implement update scheduling and installation UX

- [ ] Write failing Vitest cases for first-launch checks, the 24-hour boundary, future/invalid timestamps, dismissed-version handling, and storage read/write failures.
- [ ] Implement `updateState.ts` with named storage keys, a 24-hour interval constant, safe parsing, and pure functions for `shouldCheck`, `recordCheck`, `dismissVersion`, and `shouldShowVersion`.
- [ ] Write failing adapter tests for a no-update result, an available update, download progress, install failure, and restart failure using injected updater/process functions.
- [ ] Implement `updateCommands.ts` so the rest of the UI depends on a small adapter rather than directly on Tauri plugin globals; use `downloadAndInstall` and `relaunch` only after confirmation.
- [ ] Add Svelte state in `App.svelte` for checking, available update, download progress, install failure, and dismissal; invoke the adapter after existing startup status/project loading without delaying either operation.
- [ ] Add a compact, accessible notification near the existing version chip with `Update now`, `Later`, and a progress/error state; do not add a modal that blocks RADcast, RADTTS, or RADcite work.
- [ ] Keep the current version visible in the header and show the release version as the update target; on Windows allow the installer to exit/restart the app as required by the updater plugin.
- [ ] Run focused update tests, the complete frontend test suite, `npm run check`, and `npm run build`.

### Task 4: Build deterministic release metadata

- [ ] Write failing Python tests for all required assets, platform mapping, `.sig` contents, release URLs, SemVer/tag matching, SHA-256 output, and missing or unsigned artifact failures.
- [ ] Implement `scripts/build-update-manifest.py` using only the Python standard library; accept version, release owner/repository, asset directory, output path, and release timestamp as arguments.
- [ ] Rename/copy native build outputs into the documented stable names: `RADsuite_<version>_Apple-Silicon.dmg`, `RADsuite_<version>_Intel.dmg`, `RADsuite_<version>_Windows_x64_Setup.exe`, `RADsuite_<version>_Windows_x64.msi`, plus the signed updater artifacts and `.sig` files.
- [ ] Generate `latest.json` with the exact signed signature text and HTTPS download URL for each updater artifact, then generate `SHA256SUMS.txt` for every published binary and signature asset.
- [ ] Add validation that rejects path traversal, non-HTTPS URLs, missing signatures, duplicate platform entries, mismatched version names, and Windows MSI substitution for the automatic updater target.
- [ ] Run the Python test suite and a temporary fixture generation command, then inspect the generated JSON and checksums.

### Task 5: Add three-platform GitHub release workflow

- [ ] Add a tag-validation job that accepts only `v<SemVer>` tags, checks the Rust/Tauri/frontend versions all equal the tag version, and verifies the required Tauri updater signing secrets are present before any public release work begins.
- [ ] Add an Apple Silicon macOS build on the native ARM runner and create the Tauri-signed `.app.tar.gz` updater artifact using the protected Tauri private key; retain a documented path for adding Apple Developer signing/notarization later.
- [ ] Add an Intel macOS build on the native Intel runner with the same updater-signing steps; upload immutable labelled artifacts for the release job.
- [ ] Add a Windows x64 build on `windows-latest`, build the updater-signed NSIS and MSI installers with `createUpdaterArtifacts: true`, and upload the EXE updater artifact, MSI distribution installer, and signatures; retain a documented path for adding Windows platform signing later.
- [ ] Run frontend and Rust verification in the build jobs or a required validation job before artifact upload; retain the existing manually-triggered Windows workflow for non-release testing.
- [ ] Add a release job with `contents: write` as its only write permission; create a draft GitHub Release, upload labelled installers/updater artifacts, generate `latest.json` and `SHA256SUMS.txt`, and verify every referenced asset through the GitHub API.
- [ ] Publish the draft only after JSON, signature, checksum, platform-signing, and asset checks pass; fetch the public `releases/latest/download/latest.json` URL after publication and validate it once more.
- [ ] Ensure normal branch pushes and pull requests cannot publish or replace a stable release, and ensure failed jobs leave no misleading public `latest.json`.

### Task 6: Document the public install path and release operations

- [ ] Document the permanent install page as `https://github.com/radicalmove/radsuite/releases/latest` and explain which asset to choose for Apple Silicon, Intel Mac, and Windows.
- [ ] Document the first-install requirement for existing `0.2.1` users and the stable-only update behavior.
- [ ] Document the protected secrets required for Tauri updater signing, Apple signing/notarization, and Windows signing, without recording any secret values.
- [ ] Document the exact tag/release commands and the post-publication validation checklist.
- [ ] Add a release-note entry for `0.2.2` stating that it is the updater bridge release and that later stable releases can update automatically.

### Task 7: Verify and package the release

- [ ] Run `npm test -- --run`, `npm run check`, `npm run build`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features`.
- [ ] Run the manifest tests and a local release-fixture validation with deliberately missing and invalid signatures to prove failures are clear.
- [ ] Run `cargo tauri build` locally where possible with a temporary test signing key, inspect generated updater artifact names, and verify the config uses the expected updater endpoint.
- [ ] Confirm `git diff --check`, clean generated files, and a clean working tree except for intentional changes.
- [ ] Commit the implementation, push `codex/first-run-local-runtimes`, and create a pull request; do not publish `v0.2.2` until the required platform-signing secrets are configured and the workflow has completed successfully.
