$ErrorActionPreference = "Stop"

$RadcastRepository = "https://github.com/radicalmove/RADcast/archive/refs/heads/main.zip"
$RadtTsRepository = "https://github.com/radicalmove/RADTTS/archive/refs/heads/main.zip"
$RadtTsRoot = Join-Path $env:USERPROFILE "RADTTS"
$RadtTsVenv = Join-Path $RadtTsRoot ".venv"
$RadcastRoot = Join-Path $env:USERPROFILE ".radcast"
$RadcastVenv = Join-Path $RadcastRoot "venv"

function Log([string]$Message) {
  Write-Output "[RADsuite setup] $Message"
}

function Invoke-Checked([string]$File, [string[]]$Arguments) {
  & $File @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code $LASTEXITCODE`: $File $($Arguments -join ' ')"
  }
}

function Refresh-ProcessPath() {
  $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
  $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
  $env:Path = (@($machinePath, $userPath, $env:Path) | Where-Object { $_ } | Select-Object -Unique) -join ";"
}

function Test-Python311([string]$Executable) {
  if ([string]::IsNullOrWhiteSpace($Executable) -or -not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
    return $false
  }
  try {
    & $Executable -c "import sys; raise SystemExit(0 if sys.version_info[:2] == (3, 11) else 1)" 2>$null
    return $LASTEXITCODE -eq 0
  } catch {
    return $false
  }
}

function Resolve-Python311() {
  $candidates = @()
  foreach ($commandName in @("python.exe", "python3.exe")) {
    $command = Get-Command $commandName -ErrorAction SilentlyContinue
    if ($command -and $command.Source) {
      $candidates += $command.Source
    }
  }

  $localAppData = [Environment]::GetEnvironmentVariable("LOCALAPPDATA")
  $programFiles = [Environment]::GetEnvironmentVariable("ProgramFiles")
  $programFilesX86 = [Environment]::GetEnvironmentVariable("ProgramFiles(x86)")
  if ($localAppData) { $candidates += Join-Path $localAppData "Programs\Python\Python311\python.exe" }
  if ($programFiles) { $candidates += Join-Path $programFiles "Python311\python.exe" }
  if ($programFilesX86) { $candidates += Join-Path $programFilesX86 "Python311\python.exe" }

  foreach ($candidate in ($candidates | Select-Object -Unique)) {
    if (Test-Python311 $candidate) {
      return $candidate
    }
  }

  $launcher = Get-Command py -ErrorAction SilentlyContinue
  if ($launcher -and $launcher.Source) {
    try {
      $installedVersions = (& $launcher.Source --list 2>$null) -join "`n"
      if ($installedVersions -match "3\.11") {
        $resolved = (& $launcher.Source -3.11 -c "import sys; print(sys.executable)" 2>$null | Select-Object -Last 1)
        if ($resolved) {
          $resolved = $resolved.ToString().Trim()
          if (Test-Python311 $resolved) {
            return $resolved
          }
        }
      }
    } catch {
      # The launcher may exist without having Python 3.11 installed.
    }
  }

  return $null
}

$Python = Resolve-Python311
if (-not $Python) {
  $winget = Get-Command winget -ErrorAction SilentlyContinue
  if ($winget) {
    Log "Installing Python 3.11 with winget."
    Invoke-Checked $winget.Source @(
      "install", "--exact", "--id", "Python.Python.3.11", "--scope", "user",
      "--accept-source-agreements", "--accept-package-agreements"
    )
    Refresh-ProcessPath
    $Python = Resolve-Python311
    if (-not $Python) {
      throw "Python 3.11 was installed but is not available to RADsuite yet. Close and reopen RADsuite, then choose Prepare audio and voice tools again."
    }
  } else {
    throw "Python 3.11 is required. Install it from https://www.python.org/downloads/windows/ and run setup again."
  }
}
Log "Using Python 3.11 at $Python."

function Ensure-Ffmpeg() {
  if ((Get-Command ffmpeg -ErrorAction SilentlyContinue) -and (Get-Command ffprobe -ErrorAction SilentlyContinue)) {
    return
  }
  $winget = Get-Command winget -ErrorAction SilentlyContinue
  if (-not $winget) {
    throw "FFmpeg is required for audio processing. Install it from https://ffmpeg.org/download.html and run setup again."
  }
  Log "Installing FFmpeg for local audio conversion."
  Invoke-Checked $winget.Source @(
    "install", "--exact", "--id", "Gyan.FFmpeg.Shared", "--scope", "user",
    "--accept-source-agreements", "--accept-package-agreements"
  )
  Refresh-ProcessPath
  if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue) -or -not (Get-Command ffprobe -ErrorAction SilentlyContinue)) {
    throw "FFmpeg was installed but is not available on PATH. Restart RADsuite and run setup again."
  }
}

Ensure-Ffmpeg

function Ensure-Venv([string]$Venv) {
  $pythonPath = Join-Path $Venv "Scripts\python.exe"
  if (-not (Test-Path $pythonPath)) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Venv) | Out-Null
    Invoke-Checked $Python @("-m", "venv", $Venv)
  }
  return $pythonPath
}

$PyTorchCpuIndex = "https://download.pytorch.org/whl/cpu"
$PyPiIndex = "https://pypi.org/simple"

Log "Preparing RADTTS voice generation."
$RadtTsPython = Ensure-Venv $RadtTsVenv
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "pip")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "--index-url", $PyTorchCpuIndex, "--extra-index-url", $PyPiIndex, "torch==2.10.0", "torchaudio==2.10.0")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "--no-deps", "radtts @ $RadtTsRepository")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "--no-deps", "qwen-tts==0.1.1")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "pydantic>=2.8,<3", "numpy>=1.26,<2", "soundfile>=0.12", "imageio-ffmpeg>=0.5", "faster-whisper>=1.2.0,<2", "onnxruntime>=1.24.0,<2", "transformers==4.57.3", "accelerate==1.12.0", "einops>=0.8,<1", "sox>=1.5,<2", "librosa>=0.11,<0.12", "edge-tts>=6.1.4", "gradio")

Log "Preparing RADcast audio cleanup."
$RadcastPython = Ensure-Venv $RadcastVenv
Invoke-Checked $RadcastPython @("-m", "pip", "install", "--upgrade", "pip")
Invoke-Checked $RadcastPython @("-m", "pip", "install", "--upgrade", "--index-url", $PyTorchCpuIndex, "--extra-index-url", $PyPiIndex, "torch==2.1.1", "torchaudio==2.1.1", "torchvision==0.16.1")
Invoke-Checked $RadcastPython @("-m", "pip", "install", "--upgrade", "--no-build-isolation", $RadcastRepository, "resemble-enhance", "deepfilternet", "nara-wpe")

Log "Local runtimes are ready. Large model files will download when each tool is first used."
