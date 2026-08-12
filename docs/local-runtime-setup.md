# Local Runtime Setup

RADsuite is intentionally a small desktop download. The first-run setup installs the larger local Python environments for RADcast and RADTTS under the current user's home directory:

- RADcast: `~/.radcast/venv/bin/radcast-studio-enhance` on macOS/Linux and `%USERPROFILE%\.radcast\venv\Scripts\radcast-studio-enhance.exe` on Windows.
- RADTTS: `~/RADTTS/.venv/bin/radtts` on macOS/Linux and `%USERPROFILE%\RADTTS\.venv\Scripts\radtts.exe` on Windows.

The app launches the platform-specific setup script from its bundled resources. The scripts are safe to run again: existing environments are reused and packages are upgraded in place. They install Python packages and native prerequisites, but do not download the largest speech or enhancement model files. Those models are downloaded by the relevant tool when it is first used, which keeps the initial app package small and makes setup failures easier to diagnose.

The scripts install FFmpeg/FFprobe when the platform package manager is available, because RADcast uses them for trimming and output conversion. They use the current `main` branches of the RADcast and RADTTS repositories. On Intel macOS, RADTTS uses the newest Torch line for which Intel wheels are available; Apple Silicon and Windows use the current RADTTS pins. A later release process can replace those URLs with pinned release archives once the runtime APIs are versioned independently.

For diagnostics without changing the machine:

```sh
bash scripts/setup-local-runtimes.sh --diagnostic
bash scripts/setup-local-runtimes.sh --dry-run
```
