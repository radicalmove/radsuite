use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionFormat {
    Srt,
    Vtt,
}

impl CaptionFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Srt => "srt",
            Self::Vtt => "vtt",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionProcessingRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub caption_format: CaptionFormat,
    pub language: String,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionProcessingResult {
    pub output_path: PathBuf,
    pub caption_format: CaptionFormat,
    pub segment_count: usize,
}

#[derive(Debug, Error)]
pub enum CaptionProcessingError {
    #[error("audio input does not exist: {path}")]
    MissingInput { path: PathBuf },
    #[error("caption model does not exist: {path}")]
    MissingModel { path: PathBuf },
    #[error("caption output path has no parent directory: {path}")]
    MissingOutputParent { path: PathBuf },
    #[error("invalid caption clip range: start {start:?}, end {end:?}")]
    InvalidClipRange {
        start: Option<f64>,
        end: Option<f64>,
    },
    #[error("caption language cannot be empty")]
    EmptyLanguage,
    #[error("could not start {command}: {source}")]
    StartCommand {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{command} failed: {message}")]
    CommandFailed { command: String, message: String },
    #[error("whisper did not create the expected caption output: {path}")]
    MissingOutput { path: PathBuf },
    #[error("failed to prepare caption output directory")]
    PrepareOutput(#[source] std::io::Error),
    #[error("failed to read caption output: {source}")]
    ReadOutput {
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptionProcessor {
    whisper_command: PathBuf,
    model_path: PathBuf,
}

impl Default for CaptionProcessor {
    fn default() -> Self {
        Self::from_commands(
            resolve_tool("RADSUITE_WHISPER", "whisper-cli"),
            resolve_model(),
        )
    }
}

impl CaptionProcessor {
    pub fn from_commands(
        whisper_command: impl Into<PathBuf>,
        model_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            whisper_command: whisper_command.into(),
            model_path: model_path.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        self.whisper_command.is_file() && self.model_path.is_file()
    }

    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    pub fn whisper_arguments(
        &self,
        request: &CaptionProcessingRequest,
    ) -> Result<Vec<OsString>, CaptionProcessingError> {
        Self::validate_request(request)?;
        let output_base = output_base_path(&request.output_path);
        let language = request.language.trim();
        let mut args = vec![
            OsString::from("-m"),
            self.model_path.clone().into_os_string(),
            OsString::from("-f"),
            request.input_path.clone().into_os_string(),
            OsString::from("-of"),
            output_base.into_os_string(),
        ];

        if let Some(start) = request.clip_start_seconds {
            args.extend([
                OsString::from("-ot"),
                OsString::from(format!("{}", seconds_to_milliseconds(start))),
            ]);
        }
        if let Some(end) = request.clip_end_seconds {
            let start = request.clip_start_seconds.unwrap_or(0.0);
            args.extend([
                OsString::from("-d"),
                OsString::from(format!("{}", seconds_to_milliseconds(end - start))),
            ]);
        }

        args.extend([
            OsString::from(match request.caption_format {
                CaptionFormat::Srt => "-osrt",
                CaptionFormat::Vtt => "-ovtt",
            }),
            OsString::from("-l"),
            OsString::from(language),
            OsString::from("-np"),
        ]);
        Ok(args)
    }

    pub fn validate_request(
        request: &CaptionProcessingRequest,
    ) -> Result<(), CaptionProcessingError> {
        if request.language.trim().is_empty() {
            return Err(CaptionProcessingError::EmptyLanguage);
        }
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
            return Err(CaptionProcessingError::InvalidClipRange {
                start: request.clip_start_seconds,
                end: request.clip_end_seconds,
            });
        }
        Ok(())
    }

    pub fn process(
        &self,
        request: CaptionProcessingRequest,
    ) -> Result<CaptionProcessingResult, CaptionProcessingError> {
        Self::validate_request(&request)?;
        if !request.input_path.is_file() {
            return Err(CaptionProcessingError::MissingInput {
                path: request.input_path,
            });
        }
        if !self.model_path.is_file() {
            return Err(CaptionProcessingError::MissingModel {
                path: self.model_path.clone(),
            });
        }
        let Some(parent) = request.output_path.parent() else {
            return Err(CaptionProcessingError::MissingOutputParent {
                path: request.output_path,
            });
        };
        fs::create_dir_all(parent).map_err(CaptionProcessingError::PrepareOutput)?;

        let args = self.whisper_arguments(&request)?;
        let result = Command::new(&self.whisper_command)
            .args(&args)
            .output()
            .map_err(|source| CaptionProcessingError::StartCommand {
                command: self.whisper_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            return Err(CaptionProcessingError::CommandFailed {
                command: self.whisper_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        if !request.output_path.is_file() {
            return Err(CaptionProcessingError::MissingOutput {
                path: request.output_path,
            });
        }
        let contents = fs::read_to_string(&request.output_path)
            .map_err(|source| CaptionProcessingError::ReadOutput { source })?;
        Ok(CaptionProcessingResult {
            output_path: request.output_path,
            caption_format: request.caption_format,
            segment_count: contents
                .lines()
                .filter(|line| line.contains(" --> "))
                .count(),
        })
    }
}

fn seconds_to_milliseconds(seconds: f64) -> u64 {
    (seconds * 1000.0).round() as u64
}

fn output_base_path(path: &Path) -> PathBuf {
    if path.extension().is_some() {
        path.with_extension("")
    } else {
        path.to_path_buf()
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

fn resolve_model() -> PathBuf {
    if let Ok(value) = std::env::var("RADSUITE_WHISPER_MODEL") {
        let path = PathBuf::from(value.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    for candidate in [
        home.join(".radcast/whispercpp-models/ggml-small.bin"),
        home.join(".radcast/whispercpp-models/ggml-medium.bin"),
        home.join(".cache/whisper/ggml-small.en.bin"),
        home.join(".cache/whisper/ggml-base.en.bin"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    home.join("Library/Application Support/RADsuite/radcast/models/ggml-small.bin")
}
