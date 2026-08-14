# Windows Runtime Setup

The RADsuite Windows installer contains the desktop application. On first
launch, choose **Prepare audio and voice tools**. RADsuite installs the local
processing runtimes into the current user's profile; the Python environments
and model caches remain outside the small app installer because they are large
and hardware-specific.

The setup does not require administrator rights or a pre-installed Python
package manager. If Python 3.11 is not already available, RADsuite downloads
the official Python 3.11.9 Windows runtime and installs it for the current user
only at:

```text
%LOCALAPPDATA%\RADsuite\runtime\python311
```

If FFmpeg is not already available, RADsuite downloads a private FFmpeg build
for the current user at:

```text
%LOCALAPPDATA%\RADsuite\runtime\ffmpeg\bin
```

FFmpeg is used by RADcast for trimming and output conversion. RADsuite does
not modify the system Python installation or the system PATH.

The setup checks for a real Python 3.11 executable before creating either
environment and is safe to repeat. A blocked download or application execution
policy can still prevent setup; in that case the organisation's IT policy must
allow downloads from `python.org` and `gyan.dev`.

RADcast installs its CPU Torch runtime before its audio-cleanup packages. This
is required on Windows because some of those packages compile native Torch
extensions during installation.

## RADTTS

The first-run setup performs the equivalent PowerShell steps automatically.
The expected runtime location is:

```powershell
%USERPROFILE%\RADTTS\.venv\Scripts\radtts.exe
```

The first synthesis downloads the selected Qwen model. Built-in voices use
the CustomVoice model; reference-voice synthesis uses the Base model and
requires the normal permission acknowledgement in RADsuite.

## RADcast Optimized

RADcast Optimized uses the helper environment prepared by RADsuite at:

```text
%USERPROFILE%\.radcast\venv
```

The helper expected by RADsuite is:

```text
%USERPROFILE%\.radcast\venv\Scripts\radcast-studio-enhance.exe
```

If a specialist installation is elsewhere, set a user environment variable
before starting RADsuite:

```powershell
[Environment]::SetEnvironmentVariable(
  "RADSUITE_STUDIO_COMMAND",
  "C:\path\to\radcast-studio-enhance.exe",
  "User"
)
```

Restart RADsuite after changing the environment. The Audio cleanup capability
status reports whether the helper is available. Standard cleanup remains
available without the optimized helper.

## Verification

After installing the runtimes:

1. Start RADsuite and open Help to confirm the local runtime notes.
2. Open Audio cleanup and check that RADcast Optimized is available.
3. Open Voice generation and confirm the built-in voice option appears.
4. Generate a short test output before processing a full lecture.

The Windows installer itself is available from the successful GitHub Actions
run linked from the RADsuite release workflow.
