$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$RadcastRepository = "https://github.com/radicalmove/RADcast/archive/refs/heads/main.zip"
$RadtTsRepository = "https://github.com/radicalmove/RADTTS/archive/refs/heads/main.zip"
$RadtTsRoot = Join-Path $env:USERPROFILE "RADTTS"
$RadtTsVenv = Join-Path $RadtTsRoot ".venv"
$RadcastRoot = Join-Path $env:USERPROFILE ".radcast"
$RadcastVenv = Join-Path $RadcastRoot "venv"

$LocalAppData = [Environment]::GetEnvironmentVariable("LOCALAPPDATA")
if (-not $LocalAppData) {
  $LocalAppData = Join-Path $env:USERPROFILE "AppData\Local"
}
$RuntimeRoot = Join-Path $LocalAppData "RADsuite\runtime"
$PythonRoot = Join-Path $RuntimeRoot "python311"
$PythonExecutable = Join-Path $PythonRoot "python.exe"
$PythonInstaller = Join-Path $RuntimeRoot "python-3.11.9-amd64.exe"
$PythonInstallerUrl = "https://www.python.org/ftp/python/3.11.9/python-3.11.9-amd64.exe"
$FfmpegRoot = Join-Path $RuntimeRoot "ffmpeg"
$FfmpegBin = Join-Path $FfmpegRoot "bin"
$FfmpegExecutable = Join-Path $FfmpegBin "ffmpeg.exe"
$FfprobeExecutable = Join-Path $FfmpegBin "ffprobe.exe"
$FfmpegArchive = Join-Path $RuntimeRoot "ffmpeg-release-essentials.zip"
$FfmpegExtract = Join-Path $RuntimeRoot "ffmpeg-extract"
$FfmpegArchiveUrl = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip"

function Log([string]$Message) {
  Write-Output "[RADsuite setup] $Message"
}

function Invoke-Checked([string]$File, [string[]]$Arguments) {
  & $File @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code $LASTEXITCODE`: $File $($Arguments -join ' ')"
  }
}

function Invoke-Installer([string]$File, [string[]]$Arguments) {
  $process = Start-Process -FilePath $File -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
  if ($process.ExitCode -notin @(0, 3010)) {
    throw "Installer failed with exit code $($process.ExitCode): $File"
  }
}

function Download-File([string]$Url, [string]$Destination) {
  New-Item -ItemType Directory -Force -Path (Split-Path $Destination) | Out-Null
  Log "Downloading $Url."
  Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $Destination
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
  $candidates = @($PythonExecutable)
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

New-Item -ItemType Directory -Force -Path $RuntimeRoot | Out-Null
$Python = Resolve-Python311
if (-not $Python) {
  Log "Installing a private Python 3.11 runtime for this Windows user. No administrator access is required."
  if (-not (Test-Path -LiteralPath $PythonInstaller -PathType Leaf)) {
    Download-File $PythonInstallerUrl $PythonInstaller
  }
  $targetDirArgument = 'TargetDir="{0}"' -f $PythonRoot
  Invoke-Installer $PythonInstaller @(
    "/quiet",
    "InstallAllUsers=0",
    $targetDirArgument,
    "Include_pip=1",
    "Include_test=0",
    "Include_launcher=0",
    "Shortcuts=0",
    "PrependPath=0",
    "AssociateFiles=0",
    "SimpleInstall=1"
  )
  $Python = Resolve-Python311
  if (-not $Python) {
    throw "The private Python 3.11 runtime could not be started after installation. Check whether your organisation blocks downloaded applications, then run Prepare audio and voice tools again."
  }
  Remove-Item -LiteralPath $PythonInstaller -Force -ErrorAction SilentlyContinue
}
Log "Using Python 3.11 at $Python."

function Ensure-Ffmpeg() {
  if (Test-Path -LiteralPath $FfmpegExecutable -PathType Leaf) -and (Test-Path -LiteralPath $FfprobeExecutable -PathType Leaf) {
    $env:Path = "$FfmpegBin;$env:Path"
    return
  }
  if ((Get-Command ffmpeg -ErrorAction SilentlyContinue) -and (Get-Command ffprobe -ErrorAction SilentlyContinue)) {
    return
  }

  Log "Installing a private FFmpeg runtime for this Windows user."
  Remove-Item -LiteralPath $FfmpegExtract -Recurse -Force -ErrorAction SilentlyContinue
  if (-not (Test-Path -LiteralPath $FfmpegArchive -PathType Leaf)) {
    Download-File $FfmpegArchiveUrl $FfmpegArchive
  }
  Expand-Archive -LiteralPath $FfmpegArchive -DestinationPath $FfmpegExtract -Force
  $downloadedFfmpeg = Get-ChildItem -LiteralPath $FfmpegExtract -Filter "ffmpeg.exe" -File -Recurse | Select-Object -First 1
  $downloadedFfprobe = Get-ChildItem -LiteralPath $FfmpegExtract -Filter "ffprobe.exe" -File -Recurse | Select-Object -First 1
  if (-not $downloadedFfmpeg -or -not $downloadedFfprobe) {
    throw "The downloaded FFmpeg package did not contain ffmpeg.exe and ffprobe.exe."
  }
  New-Item -ItemType Directory -Force -Path $FfmpegBin | Out-Null
  Copy-Item -LiteralPath $downloadedFfmpeg.FullName -Destination $FfmpegExecutable -Force
  Copy-Item -LiteralPath $downloadedFfprobe.FullName -Destination $FfprobeExecutable -Force
  Remove-Item -LiteralPath $FfmpegArchive -Force -ErrorAction SilentlyContinue
  Remove-Item -LiteralPath $FfmpegExtract -Recurse -Force -ErrorAction SilentlyContinue
  $env:Path = "$FfmpegBin;$env:Path"
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
