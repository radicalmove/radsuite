"""Regression checks for the Windows local-runtime bootstrap script."""

from pathlib import Path


SCRIPT = (Path(__file__).with_name("setup-local-runtimes.ps1")).read_text(
    encoding="utf-8"
)


def test_resolves_a_real_python_311_executable():
    assert "function Resolve-Python311" in SCRIPT
    assert "sys.executable" in SCRIPT
    assert "--list" in SCRIPT
    assert 'Invoke-Checked $Python @("-m", "venv", $Venv)' in SCRIPT
    assert 'Invoke-Checked $Python @("-3.11", "-m", "venv", $Venv)' not in SCRIPT


def test_installs_torch_before_radcast_build_dependencies():
    torch_install = SCRIPT.index('"torch==2.1.1"')
    radcast_install = SCRIPT.index("$RadcastRepository", torch_install)
    assert torch_install < radcast_install
    assert '"--no-build-isolation"' in SCRIPT


def test_installs_radt_ts_runtime_dependencies_that_were_missing_on_windows():
    assert '"edge-tts>=6.1.4"' in SCRIPT
    assert '"gradio"' in SCRIPT


def test_bootstraps_python_without_admin_rights_or_winget():
    assert 'python-3.11.9-amd64.exe' in SCRIPT
    assert 'InstallAllUsers=0' in SCRIPT
    assert 'TargetDir=' in SCRIPT
    assert '$PythonRoot' in SCRIPT
    assert '$RuntimeRoot' in SCRIPT
    assert 'winget' not in SCRIPT.lower()


def test_bootstraps_ffmpeg_inside_the_user_local_runtime():
    assert 'ffmpeg-release-essentials.zip' in SCRIPT
    assert 'Expand-Archive' in SCRIPT
    assert '$FfmpegBin' in SCRIPT
    assert 'winget' not in SCRIPT.lower()


def test_uses_windows_powershell_compatible_syntax():
    assert "Join-String" not in SCRIPT


if __name__ == "__main__":
    test_resolves_a_real_python_311_executable()
    test_installs_torch_before_radcast_build_dependencies()
    test_installs_radt_ts_runtime_dependencies_that_were_missing_on_windows()
    test_bootstraps_python_without_admin_rights_or_winget()
    test_bootstraps_ffmpeg_inside_the_user_local_runtime()
