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
  $env:Path = "$machinePath;$userPath"
}

$Python = $null
try {
  $Python = (Get-Command py -ErrorAction Stop).Source
  & $Python -3.11 -c "import sys; assert sys.version_info[:2] == (3, 11)"
  if ($LASTEXITCODE -ne 0) { $Python = $null }
} catch {
  $Python = $null
}

if (-not $Python) {
  $winget = Get-Command winget -ErrorAction SilentlyContinue
  if ($winget) {
    Log "Installing Python 3.11 with winget."
    Invoke-Checked $winget.Source @(
      "install", "--exact", "--id", "Python.Python.3.11", "--scope", "user",
      "--accept-source-agreements", "--accept-package-agreements"
    )
    Refresh-ProcessPath
    $Python = (Get-Command py -ErrorAction Stop).Source
  } else {
    throw "Python 3.11 is required. Install it from https://www.python.org/downloads/windows/ and run setup again."
  }
}

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
    Invoke-Checked $Python @("-3.11", "-m", "venv", $Venv)
  }
  return $pythonPath
}

Log "Preparing RADTTS voice generation."
$RadtTsPython = Ensure-Venv $RadtTsVenv
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "pip")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "torch==2.10.0", "torchaudio==2.10.0")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "--no-deps", "radtts @ $RadtTsRepository")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "--no-deps", "qwen-tts==0.1.1")
Invoke-Checked $RadtTsPython @("-m", "pip", "install", "--upgrade", "pydantic>=2.8,<3", "numpy>=1.26,<2", "soundfile>=0.12", "imageio-ffmpeg>=0.5", "faster-whisper>=1.2.0,<2", "onnxruntime>=1.24.0,<2", "transformers==4.57.3", "accelerate==1.12.0", "einops>=0.8,<1", "sox>=1.5,<2", "librosa>=0.11,<0.12")

Log "Preparing RADcast audio cleanup."
$RadcastPython = Ensure-Venv $RadcastVenv
Invoke-Checked $RadcastPython @("-m", "pip", "install", "--upgrade", "pip")
Invoke-Checked $RadcastPython @("-m", "pip", "install", "--upgrade", $RadcastRepository, "resemble-enhance", "deepfilternet", "nara-wpe")

Log "Local runtimes are ready. Large model files will download when each tool is first used."
