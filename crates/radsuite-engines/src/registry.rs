use std::{
    env,
    path::{Path, PathBuf},
};

const AUDIO_CLEANUP_HOME_CANDIDATES: &[&str] = &[
    ".radcast/venv311/bin/radcast-studio-enhance",
    ".radcast/venv/bin/radcast-studio-enhance",
    ".radcast/venv311/Scripts/radcast-studio-enhance.exe",
    ".radcast/venv/Scripts/radcast-studio-enhance.exe",
    ".radcast/venv311/Scripts/radcast-studio-enhance.cmd",
    ".radcast/venv/Scripts/radcast-studio-enhance.cmd",
];

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
                AUDIO_CLEANUP_HOME_CANDIDATES,
            ),
            resolve_command(
                "RADSUITE_RADTTS_CLI",
                "radtts",
                &["RADTTS/.venv/bin/radtts", "RADTTS/.venv/Scripts/radtts.exe"],
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

    let home = if cfg!(windows) {
        env::var_os("USERPROFILE").or_else(|| env::var_os("HOME"))
    } else {
        env::var_os("HOME")
    }
    .map(PathBuf::from);
    if let Some(path) = resolve_home_command(home.as_deref(), home_candidates) {
        return Some(path);
    }

    find_on_path(command)
}

fn resolve_home_command(home: Option<&Path>, candidates: &[&str]) -> Option<PathBuf> {
    let home = home?;
    candidates
        .iter()
        .map(|relative_path| home.join(relative_path))
        .find(|path| path.is_file())
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return command_path.is_file().then(|| command_path.to_path_buf());
    }

    let path = env::var_os("PATH")?;
    let command_names = if cfg!(windows) {
        vec![
            command.to_string(),
            format!("{command}.exe"),
            format!("{command}.cmd"),
            format!("{command}.bat"),
        ]
    } else {
        vec![command.to_string()]
    };
    env::split_paths(&path)
        .flat_map(|directory| command_names.iter().map(move |name| directory.join(name)))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{AUDIO_CLEANUP_HOME_CANDIDATES, resolve_home_command};

    #[test]
    fn finds_audio_cleanup_helper_in_windows_user_venv() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let home = std::env::temp_dir().join(format!("radsuite-registry-home-{suffix}"));
        let helper = home
            .join(".radcast")
            .join("venv")
            .join("Scripts")
            .join("radcast-studio-enhance.exe");
        fs::create_dir_all(helper.parent().expect("helper has a parent"))
            .expect("create helper directory");
        fs::write(&helper, b"helper").expect("write helper marker");

        assert_eq!(
            resolve_home_command(Some(&home), AUDIO_CLEANUP_HOME_CANDIDATES),
            Some(helper)
        );

        fs::remove_dir_all(home).expect("remove helper directory");
    }
}
