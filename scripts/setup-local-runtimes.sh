#!/usr/bin/env bash
set -euo pipefail

RADCAST_REPOSITORY="https://github.com/radicalmove/RADcast/archive/refs/heads/main.zip"
RADTTS_REPOSITORY="https://github.com/radicalmove/RADTTS/archive/refs/heads/main.zip"
RADCAST_ROOT="${HOME}/.radcast"
RADCAST_VENV="${RADCAST_ROOT}/venv"
RADTTS_ROOT="${HOME}/RADTTS"
RADTTS_VENV="${RADTTS_ROOT}/.venv"

runtime_arch="${RADSUITE_ARCH:-$(uname -m)}"
if [ "$runtime_arch" = "x86_64" ]; then
  RADTTS_TORCH_PACKAGES=(torch==2.2.2 torchaudio==2.2.2 torchvision==0.17.2)
else
  RADTTS_TORCH_PACKAGES=(torch==2.10.0 torchaudio==2.10.0)
fi

log() {
  printf '[RADsuite setup] %s\n' "$*"
}

fail() {
  printf '[RADsuite setup] ERROR: %s\n' "$*" >&2
  exit 1
}

dry_run=0
diagnostic=0
for argument in "$@"; do
  case "$argument" in
    --dry-run) dry_run=1 ;;
    --diagnostic) diagnostic=1 ;;
    *) fail "Unknown option: ${argument}" ;;
  esac
done

python_executable=""
if command -v python3.11 >/dev/null 2>&1; then
  python_executable="$(command -v python3.11)"
elif command -v brew >/dev/null 2>&1 && [ -x "$(brew --prefix python@3.11 2>/dev/null)/bin/python3.11" ]; then
  python_executable="$(brew --prefix python@3.11)/bin/python3.11"
fi

if [ "$diagnostic" -eq 1 ]; then
  log "Architecture: $(uname -m)"
  log "macOS: $(sw_vers -productVersion 2>/dev/null || printf 'unknown')"
  log "Python 3.11: ${python_executable:-not found}"
  log "Homebrew: $(command -v brew || printf 'not found')"
  log "RADcast helper: ${RADCAST_VENV}/bin/radcast-studio-enhance"
  log "RADTTS helper: ${RADTTS_VENV}/bin/radtts"
  exit 0
fi

if [ -z "$python_executable" ]; then
  if command -v brew >/dev/null 2>&1; then
    log "Installing Python 3.11 and local audio prerequisites with Homebrew."
    if [ "$dry_run" -eq 0 ]; then
      brew install python@3.11 ffmpeg git-lfs
      python_executable="$(brew --prefix python@3.11)/bin/python3.11"
    else
      python_executable="$(brew --prefix python@3.11 2>/dev/null || printf '%s' '/opt/homebrew/opt/python@3.11')/bin/python3.11"
    fi
  else
    fail "Python 3.11 is required. Install Homebrew from https://brew.sh, then run setup again."
  fi
fi

run() {
  if [ "$dry_run" -eq 1 ]; then
    printf '+ '
    printf '%q ' "$@"
    printf '\n'
  else
    "$@"
  fi
}

ensure_native_prerequisites() {
  if command -v ffmpeg >/dev/null 2>&1 && command -v ffprobe >/dev/null 2>&1; then
    return
  fi
  if ! command -v brew >/dev/null 2>&1; then
    fail "FFmpeg is required for RADcast audio processing. Install Homebrew from https://brew.sh, then run setup again."
  fi
  if ! brew list --formula ffmpeg >/dev/null 2>&1; then
    log "Installing FFmpeg for local audio conversion."
    run brew install ffmpeg
  fi
}

ensure_native_prerequisites

create_venv() {
  local venv_path="$1"
  if [ ! -x "${venv_path}/bin/python" ]; then
    run "$python_executable" -m venv "$venv_path"
  fi
}

install_radt_ts() {
  log "Preparing RADTTS voice generation."
  create_venv "$RADTTS_VENV"
  run "${RADTTS_VENV}/bin/python" -m pip install --upgrade pip
  run "${RADTTS_VENV}/bin/python" -m pip install --upgrade "${RADTTS_TORCH_PACKAGES[@]}"
  run "${RADTTS_VENV}/bin/python" -m pip install --upgrade --no-deps "radtts @ ${RADTTS_REPOSITORY}"
  run "${RADTTS_VENV}/bin/python" -m pip install --upgrade --no-deps "qwen-tts==0.1.1"
  run "${RADTTS_VENV}/bin/python" -m pip install --upgrade \
    "pydantic>=2.8,<3" "numpy>=1.26,<2" "soundfile>=0.12" "imageio-ffmpeg>=0.5" \
    "faster-whisper>=1.2.0,<2" "onnxruntime>=1.24.0,<2" \
    "transformers==4.57.3" "accelerate==1.12.0" "einops>=0.8,<1" "sox>=1.5,<2" "librosa>=0.11,<0.12"
  if [ "$dry_run" -eq 0 ] && [ ! -x "${RADTTS_VENV}/bin/radtts" ]; then
    fail "RADTTS installed but its command was not created at ${RADTTS_VENV}/bin/radtts."
  fi
}

install_radcast() {
  log "Preparing RADcast audio cleanup."
  create_venv "$RADCAST_VENV"
  run "${RADCAST_VENV}/bin/python" -m pip install --upgrade pip
  run "${RADCAST_VENV}/bin/python" -m pip install --upgrade "${RADCAST_REPOSITORY}" resemble-enhance deepfilternet nara-wpe
  run "${RADCAST_VENV}/bin/python" -m pip install --upgrade torch==2.1.1 torchaudio==2.1.1 torchvision==0.16.1
  if [ "$dry_run" -eq 0 ] && [ ! -x "${RADCAST_VENV}/bin/radcast-studio-enhance" ]; then
    fail "RADcast installed but its command was not created at ${RADCAST_VENV}/bin/radcast-studio-enhance."
  fi
}

install_radt_ts
install_radcast
log "Local runtimes are ready. Large model files will download when each tool is first used."
