use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::audio::AudioTimeInterval;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionFormat {
    Srt,
    Vtt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaptionQualityMode {
    Fast,
    Accurate,
    Reviewed,
}

impl CaptionQualityMode {
    const fn beam_size(self) -> u8 {
        match self {
            Self::Fast => 1,
            Self::Accurate => 3,
            Self::Reviewed => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FillerRemovalMode {
    Normal,
    Aggressive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionTranscriptionRequest {
    pub input_path: PathBuf,
    pub language: String,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionWord {
    pub text: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub probability: f64,
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
    #[error("failed to parse whisper transcription output: {0}")]
    ParseTranscription(#[source] serde_json::Error),
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
        self.whisper_arguments_with_options(request, CaptionQualityMode::Accurate, None)
    }

    pub fn whisper_arguments_with_options(
        &self,
        request: &CaptionProcessingRequest,
        quality_mode: CaptionQualityMode,
        glossary: Option<&str>,
    ) -> Result<Vec<OsString>, CaptionProcessingError> {
        Self::validate_request(request)?;
        let model_path = self.model_path_for_quality(quality_mode);
        let output_base = output_base_path(&request.output_path);
        let language = request.language.trim();
        let mut args = vec![
            OsString::from("-m"),
            model_path.into_os_string(),
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
            OsString::from("-bs"),
            OsString::from(quality_mode.beam_size().to_string()),
            OsString::from("-np"),
        ]);
        if let Some(prompt) = caption_prompt(glossary) {
            args.extend([OsString::from("--prompt"), OsString::from(prompt)]);
        }
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

    pub fn transcribe_words(
        &self,
        request: &CaptionTranscriptionRequest,
    ) -> Result<Vec<CaptionWord>, CaptionProcessingError> {
        self.transcribe_words_with_options(request, CaptionQualityMode::Fast, None)
    }

    pub fn transcribe_words_with_options(
        &self,
        request: &CaptionTranscriptionRequest,
        quality_mode: CaptionQualityMode,
        glossary: Option<&str>,
    ) -> Result<Vec<CaptionWord>, CaptionProcessingError> {
        validate_transcription_request(request)?;
        if !request.input_path.is_file() {
            return Err(CaptionProcessingError::MissingInput {
                path: request.input_path.clone(),
            });
        }
        let model_path = self.model_path_for_quality(quality_mode);
        if !model_path.is_file() {
            return Err(CaptionProcessingError::MissingModel { path: model_path });
        }

        let output_base = temporary_output_base();
        let output_path = output_base.with_extension("json");
        let args = self.transcription_arguments(
            request,
            &output_base,
            &model_path,
            quality_mode,
            glossary,
        );
        let result = command_for_executable(&self.whisper_command)
            .args(&args)
            .output()
            .map_err(|source| CaptionProcessingError::StartCommand {
                command: self.whisper_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            let _ = fs::remove_file(&output_path);
            return Err(CaptionProcessingError::CommandFailed {
                command: self.whisper_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }
        if !output_path.is_file() {
            return Err(CaptionProcessingError::MissingOutput { path: output_path });
        }

        let contents = fs::read_to_string(&output_path)
            .map_err(|source| CaptionProcessingError::ReadOutput { source });
        let _ = fs::remove_file(&output_path);
        let contents = contents?;
        let document: WhisperDocument =
            serde_json::from_str(&contents).map_err(CaptionProcessingError::ParseTranscription)?;

        Ok(document
            .transcription
            .into_iter()
            .flat_map(|segment| segment.tokens)
            .filter_map(|token| {
                let offsets = token.offsets?;
                let start_seconds = offsets.from as f64 / 1000.0;
                let end_seconds = offsets.to as f64 / 1000.0;
                if token.text.trim().is_empty() || end_seconds <= start_seconds {
                    return None;
                }
                Some(CaptionWord {
                    text: token.text.trim().to_string(),
                    start_seconds,
                    end_seconds,
                    probability: token.probability,
                })
            })
            .collect())
    }

    pub fn filler_intervals(
        &self,
        request: &CaptionTranscriptionRequest,
        mode: FillerRemovalMode,
    ) -> Result<Vec<AudioTimeInterval>, CaptionProcessingError> {
        let clip_start = request.clip_start_seconds.unwrap_or(0.0);
        let mut words =
            self.transcribe_words_with_options(request, CaptionQualityMode::Fast, None)?;
        if clip_start > 0.0 {
            for word in &mut words {
                word.start_seconds = (word.start_seconds - clip_start).max(0.0);
                word.end_seconds = (word.end_seconds - clip_start).max(0.0);
            }
        }
        Ok(detect_filler_intervals(&words, mode))
    }

    fn transcription_arguments(
        &self,
        request: &CaptionTranscriptionRequest,
        output_base: &Path,
        model_path: &Path,
        quality_mode: CaptionQualityMode,
        glossary: Option<&str>,
    ) -> Vec<OsString> {
        let mut args = vec![
            OsString::from("-m"),
            model_path.to_path_buf().into_os_string(),
            OsString::from("-f"),
            request.input_path.clone().into_os_string(),
            OsString::from("-of"),
            output_base.to_path_buf().into_os_string(),
        ];
        append_clip_arguments(
            &mut args,
            request.clip_start_seconds,
            request.clip_end_seconds,
        );
        args.extend([
            OsString::from("-oj"),
            OsString::from("-ojf"),
            OsString::from("-l"),
            OsString::from(request.language.trim()),
            OsString::from("-bs"),
            OsString::from(quality_mode.beam_size().to_string()),
            OsString::from("-np"),
        ]);
        if let Some(prompt) = caption_prompt(glossary) {
            args.extend([OsString::from("--prompt"), OsString::from(prompt)]);
        }
        args
    }

    pub fn model_path_for_quality(&self, quality_mode: CaptionQualityMode) -> PathBuf {
        let mode_variable = match quality_mode {
            CaptionQualityMode::Fast => return self.model_path.clone(),
            CaptionQualityMode::Accurate => "RADSUITE_WHISPER_ACCURATE_MODEL",
            CaptionQualityMode::Reviewed => "RADSUITE_WHISPER_REVIEWED_MODEL",
        };
        let mut candidates = Vec::new();
        if let Ok(value) = std::env::var(mode_variable) {
            candidates.push(PathBuf::from(value.trim()));
        }
        if quality_mode == CaptionQualityMode::Reviewed
            && let Ok(value) = std::env::var("RADSUITE_WHISPER_ACCURATE_MODEL")
        {
            candidates.push(PathBuf::from(value.trim()));
        }

        let standard_model = self
            .model_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("ggml-"));
        if standard_model {
            let mut local_models = Vec::new();
            if quality_mode == CaptionQualityMode::Reviewed {
                local_models.push("ggml-large-v3.bin");
            }
            local_models.push("ggml-medium.bin");
            if let Some(parent) = self.model_path.parent() {
                candidates.extend(local_models.iter().map(|name| parent.join(name)));
            }
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default();
            for name in local_models {
                candidates.push(home.join(".radcast/whispercpp-models").join(name));
            }
            if quality_mode == CaptionQualityMode::Reviewed {
                candidates.push(home.join(".cache/whisper/ggml-large-v3.bin"));
            }
            candidates.push(home.join(".cache/whisper/ggml-medium.en.bin"));
        }
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .unwrap_or_else(|| self.model_path.clone())
    }

    pub fn process(
        &self,
        request: CaptionProcessingRequest,
    ) -> Result<CaptionProcessingResult, CaptionProcessingError> {
        self.process_with_options(request, CaptionQualityMode::Accurate, None)
    }

    pub fn process_with_options(
        &self,
        request: CaptionProcessingRequest,
        quality_mode: CaptionQualityMode,
        glossary: Option<&str>,
    ) -> Result<CaptionProcessingResult, CaptionProcessingError> {
        Self::validate_request(&request)?;
        if !request.input_path.is_file() {
            return Err(CaptionProcessingError::MissingInput {
                path: request.input_path,
            });
        }
        let model_path = self.model_path_for_quality(quality_mode);
        if !model_path.is_file() {
            return Err(CaptionProcessingError::MissingModel { path: model_path });
        }
        let Some(parent) = request.output_path.parent() else {
            return Err(CaptionProcessingError::MissingOutputParent {
                path: request.output_path,
            });
        };
        fs::create_dir_all(parent).map_err(CaptionProcessingError::PrepareOutput)?;

        let args = self.whisper_arguments_with_options(&request, quality_mode, glossary)?;
        let result = command_for_executable(&self.whisper_command)
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

pub fn detect_filler_intervals(
    words: &[CaptionWord],
    mode: FillerRemovalMode,
) -> Vec<AudioTimeInterval> {
    let mut intervals: Vec<AudioTimeInterval> = Vec::new();
    for word in words {
        let normalized = word
            .text
            .trim()
            .trim_matches(|character: char| !character.is_alphanumeric())
            .to_ascii_lowercase();
        let is_filler = match mode {
            FillerRemovalMode::Normal => {
                matches!(normalized.as_str(), "um" | "uh" | "er") && word.probability >= 0.28
            }
            FillerRemovalMode::Aggressive => {
                matches!(
                    normalized.as_str(),
                    "um" | "uh" | "er" | "erm" | "hmm" | "hm" | "mm"
                )
            }
        };
        if !is_filler {
            continue;
        }

        let start_seconds = round_milliseconds((word.start_seconds - 0.02).max(0.0));
        let end_seconds = round_milliseconds(word.end_seconds + 0.02);
        if let Some(previous) = intervals.last_mut()
            && start_seconds <= previous.end_seconds + 0.15
        {
            previous.end_seconds = previous.end_seconds.max(end_seconds);
        } else {
            intervals.push(AudioTimeInterval {
                start_seconds,
                end_seconds,
            });
        }
    }
    intervals
}

fn round_milliseconds(seconds: f64) -> f64 {
    (seconds * 1000.0).round() / 1000.0
}

fn caption_prompt(glossary: Option<&str>) -> Option<String> {
    let glossary = glossary?.split_whitespace().collect::<Vec<_>>().join(" ");
    if glossary.is_empty() {
        return None;
    }
    let glossary = glossary.chars().take(1500).collect::<String>();
    Some(format!(
        "Use these exact names and terms when they occur: {glossary}"
    ))
}

#[derive(Debug, Deserialize)]
struct WhisperDocument {
    #[serde(default)]
    transcription: Vec<WhisperSegment>,
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    #[serde(default)]
    tokens: Vec<WhisperToken>,
}

#[derive(Debug, Deserialize)]
struct WhisperToken {
    text: String,
    #[serde(default)]
    offsets: Option<WhisperOffsets>,
    #[serde(rename = "p", default)]
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct WhisperOffsets {
    from: u64,
    to: u64,
}

fn validate_transcription_request(
    request: &CaptionTranscriptionRequest,
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

fn append_clip_arguments(args: &mut Vec<OsString>, start: Option<f64>, end: Option<f64>) {
    if let Some(start) = start {
        args.extend([
            OsString::from("-ot"),
            OsString::from(format!("{}", seconds_to_milliseconds(start))),
        ]);
    }
    if let Some(end) = end {
        let start = start.unwrap_or(0.0);
        args.extend([
            OsString::from("-d"),
            OsString::from(format!("{}", seconds_to_milliseconds(end - start))),
        ]);
    }
}

fn temporary_output_base() -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!("radsuite-whisper-{}-{suffix}", std::process::id()))
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

fn command_for_executable(path: &Path) -> Command {
    #[cfg(unix)]
    if path.extension().is_some_and(|extension| extension == "sh") {
        let mut command = Command::new("sh");
        command.arg(path);
        return command;
    }

    Command::new(path)
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
