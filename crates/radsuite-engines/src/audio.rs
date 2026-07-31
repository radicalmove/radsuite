use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const CLEANUP_FILTER: &str = "highpass=f=80,lowpass=f=12000,afftdn,loudnorm=I=-16:TP=-1.5:LRA=11";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioOutputFormat {
    Mp3,
    Wav,
}

impl AudioOutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioProcessingRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub output_format: AudioOutputFormat,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
    pub cleanup_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioProcessingResult {
    pub output_path: PathBuf,
    pub duration_seconds: f64,
    pub output_format: AudioOutputFormat,
}

#[derive(Debug, Error)]
pub enum AudioProcessingError {
    #[error("audio input does not exist: {path}")]
    MissingInput { path: PathBuf },
    #[error("audio output path has no parent directory: {path}")]
    MissingOutputParent { path: PathBuf },
    #[error("invalid audio clip range: start {start:?}, end {end:?}")]
    InvalidClipRange {
        start: Option<f64>,
        end: Option<f64>,
    },
    #[error("could not start {command}: {source}")]
    StartCommand {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{command} failed: {message}")]
    CommandFailed { command: String, message: String },
    #[error("{command} returned an invalid duration")]
    InvalidDuration { command: String },
    #[error("failed to prepare audio output directory")]
    PrepareOutput(#[source] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProcessor {
    ffmpeg_command: PathBuf,
    ffprobe_command: PathBuf,
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::from_commands(
            resolve_tool("RADSUITE_FFMPEG", "ffmpeg"),
            resolve_tool("RADSUITE_FFPROBE", "ffprobe"),
        )
    }
}

impl AudioProcessor {
    pub fn from_commands(
        ffmpeg_command: impl Into<PathBuf>,
        ffprobe_command: impl Into<PathBuf>,
    ) -> Self {
        Self {
            ffmpeg_command: ffmpeg_command.into(),
            ffprobe_command: ffprobe_command.into(),
        }
    }

    pub fn validate_request(request: &AudioProcessingRequest) -> Result<(), AudioProcessingError> {
        let valid_start = request
            .clip_start_seconds
            .is_none_or(|value| value.is_finite() && value >= 0.0);
        let valid_end = request
            .clip_end_seconds
            .is_none_or(|value| value.is_finite() && value > 0.0);
        let valid_order = match (request.clip_start_seconds, request.clip_end_seconds) {
            (Some(start), Some(end)) => end > start,
            _ => true,
        };

        if !valid_start || !valid_end || !valid_order {
            return Err(AudioProcessingError::InvalidClipRange {
                start: request.clip_start_seconds,
                end: request.clip_end_seconds,
            });
        }

        Ok(())
    }

    pub fn ffmpeg_arguments(
        request: &AudioProcessingRequest,
    ) -> Result<Vec<OsString>, AudioProcessingError> {
        Self::validate_request(request)?;

        let mut args = vec![
            OsString::from("-y"),
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
        ];

        if let Some(start) = request.clip_start_seconds {
            args.push(OsString::from("-ss"));
            args.push(OsString::from(format!("{start:.3}")));
        }

        args.push(OsString::from("-i"));
        args.push(request.input_path.clone().into_os_string());

        if let Some(end) = request.clip_end_seconds {
            let start = request.clip_start_seconds.unwrap_or(0.0);
            args.push(OsString::from("-t"));
            args.push(OsString::from(format!("{:.3}", end - start)));
        }

        if request.cleanup_enabled {
            args.push(OsString::from("-af"));
            args.push(OsString::from(CLEANUP_FILTER));
        }

        match request.output_format {
            AudioOutputFormat::Mp3 => {
                args.extend([
                    OsString::from("-codec:a"),
                    OsString::from("libmp3lame"),
                    OsString::from("-q:a"),
                    OsString::from("2"),
                ]);
            }
            AudioOutputFormat::Wav => {
                args.extend([OsString::from("-codec:a"), OsString::from("pcm_s16le")]);
            }
        }

        args.push(request.output_path.clone().into_os_string());
        Ok(args)
    }

    pub fn process(
        &self,
        request: AudioProcessingRequest,
    ) -> Result<AudioProcessingResult, AudioProcessingError> {
        Self::validate_request(&request)?;
        if !request.input_path.is_file() {
            return Err(AudioProcessingError::MissingInput {
                path: request.input_path,
            });
        }

        let Some(parent) = request.output_path.parent() else {
            return Err(AudioProcessingError::MissingOutputParent {
                path: request.output_path,
            });
        };
        fs::create_dir_all(parent).map_err(AudioProcessingError::PrepareOutput)?;

        let args = Self::ffmpeg_arguments(&request)?;
        let result = Command::new(&self.ffmpeg_command)
            .args(&args)
            .output()
            .map_err(|source| AudioProcessingError::StartCommand {
                command: self.ffmpeg_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            return Err(AudioProcessingError::CommandFailed {
                command: self.ffmpeg_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        let duration_seconds = self.probe_duration(&request.output_path)?;
        Ok(AudioProcessingResult {
            output_path: request.output_path,
            duration_seconds,
            output_format: request.output_format,
        })
    }

    pub fn probe_duration(&self, path: &Path) -> Result<f64, AudioProcessingError> {
        let result = Command::new(&self.ffprobe_command)
            .args([
                OsString::from("-v"),
                OsString::from("error"),
                OsString::from("-show_entries"),
                OsString::from("format=duration"),
                OsString::from("-of"),
                OsString::from("default=noprint_wrappers=1:nokey=1"),
                path.to_path_buf().into_os_string(),
            ])
            .output()
            .map_err(|source| AudioProcessingError::StartCommand {
                command: self.ffprobe_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            return Err(AudioProcessingError::CommandFailed {
                command: self.ffprobe_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        let raw = String::from_utf8_lossy(&result.stdout).trim().to_string();
        let duration_seconds = raw
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| AudioProcessingError::InvalidDuration {
                command: self.ffprobe_command.display().to_string(),
            })?;
        Ok(duration_seconds)
    }
}

fn command_output(stdout: &[u8], stderr: &[u8]) -> String {
    let output = String::from_utf8_lossy(if stderr.is_empty() { stdout } else { stderr });
    let message = output.trim();
    if message.is_empty() {
        "no diagnostic output".to_string()
    } else {
        message.to_string()
    }
}

fn resolve_tool(environment_variable: &str, command: &str) -> PathBuf {
    if let Ok(value) = std::env::var(environment_variable) {
        let path = PathBuf::from(value.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }

    for candidate in [
        format!("/opt/homebrew/bin/{command}"),
        format!("/usr/local/bin/{command}"),
        format!("/usr/bin/{command}"),
    ] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return path;
        }
    }

    PathBuf::from(command)
}
