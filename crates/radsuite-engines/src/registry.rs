use std::{
    env,
    path::{Path, PathBuf},
};

use crate::EngineStatus;

#[derive(Debug, Clone)]
pub struct EngineRegistry {
    ffmpeg: Option<PathBuf>,
    asr: Option<PathBuf>,
    audio_cleanup: Option<PathBuf>,
    tts: Option<PathBuf>,
}

impl EngineRegistry {
    pub fn from_commands(
        ffmpeg: Option<PathBuf>,
        asr: Option<PathBuf>,
        audio_cleanup: Option<PathBuf>,
        tts: Option<PathBuf>,
    ) -> Self {
        Self {
            ffmpeg: existing_file(ffmpeg),
            asr: existing_file(asr),
            audio_cleanup: existing_file(audio_cleanup),
            tts: existing_file(tts),
        }
    }

    pub fn list(&self) -> Vec<EngineStatus> {
        [
            (
                "ffmpeg",
                "FFmpeg media processing",
                self.ffmpeg.as_ref(),
                "Install FFmpeg or set RADSUITE_FFMPEG to its executable.",
            ),
            (
                "asr",
                "Speech recognition",
                self.asr.as_ref(),
                "Install whisper.cpp or set RADSUITE_WHISPER to its executable.",
            ),
            (
                "audio_cleanup",
                "Audio cleanup",
                self.audio_cleanup.as_ref(),
                "Install the local RADcast helper or set RADSUITE_STUDIO_COMMAND to its executable.",
            ),
            (
                "tts",
                "Voice and text-to-speech",
                self.tts.as_ref(),
                "Install RADTTS or set RADSUITE_RADTTS_CLI to its executable.",
            ),
        ]
        .into_iter()
        .map(|(id, label, command, missing_detail)| EngineStatus {
            id: id.to_string(),
            label: label.to_string(),
            available: command.is_some(),
            detail: command
                .map(|path| format!("Available locally at {}.", path.display()))
                .unwrap_or_else(|| missing_detail.to_string()),
        })
        .collect()
    }
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::from_commands(
            resolve_command("RADSUITE_FFMPEG", "ffmpeg", &[]),
            resolve_command("RADSUITE_WHISPER", "whisper-cli", &[]),
            resolve_command(
                "RADSUITE_STUDIO_COMMAND",
                "radcast-studio-enhance",
                &[
                    ".radcast/venv311/bin/radcast-studio-enhance",
                    ".radcast/venv/bin/radcast-studio-enhance",
                ],
            ),
            resolve_command(
                "RADSUITE_RADTTS_CLI",
                "radtts",
                &["RADTTS/.venv/bin/radtts"],
            ),
        )
    }
}

fn existing_file(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_file())
}

fn resolve_command(env_name: &str, command: &str, home_candidates: &[&str]) -> Option<PathBuf> {
    if let Some(path) = env::var_os(env_name).map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }

    if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
        for relative_path in home_candidates {
            let path = home.join(relative_path);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    find_on_path(command)
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file().then(|| command_path.to_path_buf());
    }

    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(command))
        .find(|path| path.is_file())
}
