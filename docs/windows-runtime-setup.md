# Windows Runtime Setup

The RADsuite Windows installer contains the desktop application. The local
processing runtimes remain separate because their Python environments and
model caches are large and hardware-specific.

## RADTTS

Install Python 3.11 and Git, then run PowerShell:

```powershell
$root = Join-Path $env:USERPROFILE "RADTTS"
py -3.11 -m venv "$root\.venv"
$python = "$root\.venv\Scripts\python.exe"
& $python -m pip install --upgrade pip
& $python -m pip install --index-url https://download.pytorch.org/whl/cpu --extra-index-url https://pypi.org/simple "git+https://github.com/radicalmove/RADTTS.git#egg=radtts[asr,tts]"
```

RADsuite will detect:

```text
%USERPROFILE%\RADTTS\.venv\Scripts\radtts.exe
```

The first synthesis downloads the selected Qwen model. Built-in voices use
the CustomVoice model; reference-voice synthesis uses the Base model and
requires the normal permission acknowledgement in RADsuite.

## RADcast Optimized

RADcast Optimized requires the separate RADcast helper environment. Install
the RADcast package and its local audio/model dependencies into:

```text
%USERPROFILE%\.radcast\venv
```

The helper expected by RADsuite is:

```text
%USERPROFILE%\.radcast\venv\Scripts\radcast-studio-enhance.exe
```

If the helper is installed elsewhere, set a user environment variable before
starting RADsuite:

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
