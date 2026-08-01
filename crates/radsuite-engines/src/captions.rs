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

const MIN_COMPACTABLE_GAP_SECONDS: f64 = 0.35;
const SPEECH_INTERVAL_MERGE_TOLERANCE_SECONDS: f64 = 0.06;
const TIMING_EPSILON_SECONDS: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq)]
pub struct SpeechCleanupPlan {
    pub removal_intervals: Vec<AudioTimeInterval>,
    pub removed_pause_count: usize,
    pub removed_filler_count: usize,
}

#[derive(Debug, Error)]
pub enum SpeechCleanupPlanningError {
    #[error("cleanup duration must be finite and greater than zero")]
    InvalidDuration,
    #[error("maximum pause duration must be finite and non-negative: {seconds:?}")]
    InvalidMaxSilence { seconds: f64 },
    #[error(
        "cleanup word timing is invalid at index {index}: start {start_seconds}, end {end_seconds}"
    )]
    InvalidWordTiming {
        index: usize,
        start_seconds: f64,
        end_seconds: f64,
    },
}

#[derive(Debug, Error)]
pub enum SpeechCleanupError {
    #[error(transparent)]
    Caption(#[from] CaptionProcessingError),
    #[error(transparent)]
    Planning(#[from] SpeechCleanupPlanningError),
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

#[derive(Debug, Clone, PartialEq)]
pub struct CaptionProcessingResult {
    pub output_path: PathBuf,
    pub caption_format: CaptionFormat,
    pub segment_count: usize,
    pub quality: CaptionQualitySummary,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptionQualitySummary {
    pub average_probability: Option<f64>,
    pub low_confidence_segment_count: usize,
    pub total_segment_count: usize,
    pub review_recommended: bool,
    pub review_path: Option<PathBuf>,
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
    #[error("failed to write caption review file: {source}")]
    WriteReview {
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
            OsString::from("-oj"),
            OsString::from("-ojf"),
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

    pub fn speech_cleanup_plan(
        &self,
        request: &CaptionTranscriptionRequest,
        total_duration_seconds: f64,
        max_silence_seconds: Option<f64>,
        remove_filler_words: bool,
        filler_mode: FillerRemovalMode,
    ) -> Result<SpeechCleanupPlan, SpeechCleanupError> {
        let clip_start = request.clip_start_seconds.unwrap_or(0.0);
        let mut words = self.transcribe_words(request)?;
        if clip_start > 0.0 {
            for word in &mut words {
                word.start_seconds = (word.start_seconds - clip_start).max(0.0);
                word.end_seconds = (word.end_seconds - clip_start).max(0.0);
            }
        }
        Ok(plan_speech_cleanup(
            &words,
            total_duration_seconds,
            max_silence_seconds,
            remove_filler_words,
            filler_mode,
        )?)
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
        let json_path = output_base_path(&request.output_path).with_extension("json");
        let result = command_for_executable(&self.whisper_command)
            .args(&args)
            .output()
            .map_err(|source| CaptionProcessingError::StartCommand {
                command: self.whisper_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            let _ = fs::remove_file(&json_path);
            return Err(CaptionProcessingError::CommandFailed {
                command: self.whisper_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        if !request.output_path.is_file() {
            let _ = fs::remove_file(&json_path);
            return Err(CaptionProcessingError::MissingOutput {
                path: request.output_path,
            });
        }
        let contents = fs::read_to_string(&request.output_path)
            .map_err(|source| CaptionProcessingError::ReadOutput { source })?;
        let quality_document = fs::read_to_string(&json_path)
            .ok()
            .and_then(|contents| serde_json::from_str::<WhisperDocument>(&contents).ok());
        let _ = fs::remove_file(&json_path);
        let mut quality = quality_document
            .as_ref()
            .map(build_caption_quality_summary)
            .unwrap_or_default();
        if quality.review_recommended {
            let review_path = request.output_path.with_file_name(format!(
                "{}.review.txt",
                request
                    .output_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("captions")
            ));
            let review_document =
                format_caption_review_document(&quality, quality_document.as_ref());
            fs::write(&review_path, review_document)
                .map_err(|source| CaptionProcessingError::WriteReview { source })?;
            quality.review_path = Some(review_path);
        }
        Ok(CaptionProcessingResult {
            output_path: request.output_path,
            caption_format: request.caption_format,
            segment_count: contents
                .lines()
                .filter(|line| line.contains(" --> "))
                .count(),
            quality,
        })
    }
}

pub fn detect_filler_intervals(
    words: &[CaptionWord],
    mode: FillerRemovalMode,
) -> Vec<AudioTimeInterval> {
    let mut intervals: Vec<AudioTimeInterval> = Vec::new();
    for word in words {
        if !is_filler_word(word, mode) {
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

pub fn plan_speech_cleanup(
    words: &[CaptionWord],
    total_duration_seconds: f64,
    max_silence_seconds: Option<f64>,
    remove_filler_words: bool,
    filler_mode: FillerRemovalMode,
) -> Result<SpeechCleanupPlan, SpeechCleanupPlanningError> {
    if !total_duration_seconds.is_finite() || total_duration_seconds <= 0.0 {
        return Err(SpeechCleanupPlanningError::InvalidDuration);
    }
    if let Some(seconds) = max_silence_seconds
        && (!seconds.is_finite() || seconds < 0.0)
    {
        return Err(SpeechCleanupPlanningError::InvalidMaxSilence { seconds });
    }

    let mut ordered_words = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let valid_timing = word.start_seconds.is_finite()
            && word.end_seconds.is_finite()
            && word.start_seconds >= 0.0
            && word.end_seconds > word.start_seconds;
        if !valid_timing {
            return Err(SpeechCleanupPlanningError::InvalidWordTiming {
                index,
                start_seconds: word.start_seconds,
                end_seconds: word.end_seconds,
            });
        }
        if word.start_seconds >= total_duration_seconds {
            continue;
        }
        let end_seconds = word.end_seconds.min(total_duration_seconds);
        if end_seconds > word.start_seconds {
            ordered_words.push(CaptionWord {
                end_seconds,
                ..word.clone()
            });
        }
    }
    ordered_words.sort_by(|left, right| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
    });

    let filler_intervals = if remove_filler_words {
        detect_filler_intervals(&ordered_words, filler_mode)
            .into_iter()
            .filter_map(|interval| clamp_interval(interval, total_duration_seconds))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let speech_intervals = merge_speech_intervals(
        ordered_words
            .iter()
            .filter(|word| !remove_filler_words || !is_filler_word(word, filler_mode))
            .map(|word| AudioTimeInterval {
                start_seconds: word.start_seconds,
                end_seconds: word.end_seconds,
            }),
    );

    let (pause_intervals, removed_pause_count) = max_silence_seconds
        .map(|keep_seconds| {
            compact_pause_intervals(&speech_intervals, total_duration_seconds, keep_seconds)
        })
        .unwrap_or_default();
    let removal_intervals = merge_removal_intervals(
        pause_intervals
            .iter()
            .chain(filler_intervals.iter())
            .cloned(),
    );

    Ok(SpeechCleanupPlan {
        removal_intervals,
        removed_pause_count,
        removed_filler_count: filler_intervals.len(),
    })
}

fn is_filler_word(word: &CaptionWord, mode: FillerRemovalMode) -> bool {
    let normalized = word
        .text
        .trim()
        .trim_matches(|character: char| !character.is_alphanumeric())
        .to_ascii_lowercase();
    match mode {
        FillerRemovalMode::Normal => {
            matches!(normalized.as_str(), "um" | "uh" | "er") && word.probability >= 0.28
        }
        FillerRemovalMode::Aggressive => {
            matches!(
                normalized.as_str(),
                "um" | "uh" | "er" | "erm" | "hmm" | "hm" | "mm"
            )
        }
    }
}

fn merge_speech_intervals<I>(intervals: I) -> Vec<AudioTimeInterval>
where
    I: IntoIterator<Item = AudioTimeInterval>,
{
    let mut merged: Vec<AudioTimeInterval> = Vec::new();
    for interval in intervals {
        if let Some(previous) = merged.last_mut()
            && interval.start_seconds
                <= previous.end_seconds + SPEECH_INTERVAL_MERGE_TOLERANCE_SECONDS
        {
            previous.end_seconds = previous.end_seconds.max(interval.end_seconds);
        } else {
            merged.push(interval);
        }
    }
    merged
}

fn compact_pause_intervals(
    speech_intervals: &[AudioTimeInterval],
    total_duration_seconds: f64,
    keep_seconds: f64,
) -> (Vec<AudioTimeInterval>, usize) {
    if speech_intervals.is_empty() {
        return (Vec::new(), 0);
    }

    let trigger_seconds = MIN_COMPACTABLE_GAP_SECONDS.max(keep_seconds);
    let mut cursor: f64 = 0.0;
    let mut intervals = Vec::new();
    let mut count = 0;
    for speech in speech_intervals {
        let gap_end = cursor.max(speech.start_seconds);
        if gap_end - cursor > trigger_seconds + TIMING_EPSILON_SECONDS {
            let trim_start = (cursor + keep_seconds).min(gap_end);
            if trim_start < gap_end {
                intervals.push(AudioTimeInterval {
                    start_seconds: trim_start,
                    end_seconds: gap_end,
                });
                count += 1;
            }
        }
        cursor = cursor.max(speech.end_seconds);
    }

    if total_duration_seconds - cursor > trigger_seconds + TIMING_EPSILON_SECONDS {
        let trim_start = (cursor + keep_seconds).min(total_duration_seconds);
        if trim_start < total_duration_seconds {
            intervals.push(AudioTimeInterval {
                start_seconds: trim_start,
                end_seconds: total_duration_seconds,
            });
            count += 1;
        }
    }
    (intervals, count)
}

fn merge_removal_intervals<I>(intervals: I) -> Vec<AudioTimeInterval>
where
    I: IntoIterator<Item = AudioTimeInterval>,
{
    let mut intervals = intervals.into_iter().collect::<Vec<_>>();
    intervals.sort_by(|left, right| {
        left.start_seconds
            .total_cmp(&right.start_seconds)
            .then_with(|| left.end_seconds.total_cmp(&right.end_seconds))
    });
    let mut merged: Vec<AudioTimeInterval> = Vec::new();
    for interval in intervals {
        if let Some(previous) = merged.last_mut()
            && interval.start_seconds <= previous.end_seconds
        {
            previous.end_seconds = previous.end_seconds.max(interval.end_seconds);
        } else {
            merged.push(interval);
        }
    }
    merged
}

fn clamp_interval(
    interval: AudioTimeInterval,
    total_duration_seconds: f64,
) -> Option<AudioTimeInterval> {
    let start_seconds = interval.start_seconds.clamp(0.0, total_duration_seconds);
    let end_seconds = interval.end_seconds.clamp(0.0, total_duration_seconds);
    (end_seconds > start_seconds).then_some(AudioTimeInterval {
        start_seconds,
        end_seconds,
    })
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
    text: String,
    #[serde(default)]
    tokens: Vec<WhisperToken>,
    #[serde(default)]
    offsets: Option<WhisperOffsets>,
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

#[derive(Debug, Clone)]
struct CaptionQualityFlag {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
    probability: Option<f64>,
    reason: String,
}

fn build_caption_quality_summary(document: &WhisperDocument) -> CaptionQualitySummary {
    let segments = document
        .transcription
        .iter()
        .filter_map(caption_quality_segment)
        .collect::<Vec<_>>();
    let probabilities = segments
        .iter()
        .filter_map(|segment| segment.probability)
        .collect::<Vec<_>>();
    let average_probability = (!probabilities.is_empty())
        .then(|| probabilities.iter().sum::<f64>() / probabilities.len() as f64);
    let low_threshold = average_probability
        .map(|average| 0.45_f64.min(0.34_f64.max(average - 0.22)))
        .unwrap_or(0.45);
    let warn_threshold = average_probability
        .map(|average| 0.58_f64.min(0.46_f64.max(average - 0.14)))
        .unwrap_or(0.58);
    let flags = caption_quality_flags(&segments, low_threshold, warn_threshold);

    CaptionQualitySummary {
        average_probability,
        low_confidence_segment_count: flags.len(),
        total_segment_count: segments.len(),
        review_recommended: !flags.is_empty()
            || average_probability.is_some_and(|average| average < 0.72 && segments.len() >= 4),
        review_path: None,
    }
}

#[derive(Debug, Clone)]
struct CaptionQualitySegment {
    start_seconds: f64,
    end_seconds: f64,
    text: String,
    probability: Option<f64>,
}

fn caption_quality_segment(segment: &WhisperSegment) -> Option<CaptionQualitySegment> {
    let token_text = segment
        .tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let text = if token_text.is_empty() {
        segment
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        token_text
    };
    if text.is_empty() {
        return None;
    }

    let token_probabilities = segment
        .tokens
        .iter()
        .map(|token| token.probability)
        .filter(|probability| probability.is_finite())
        .collect::<Vec<_>>();
    let probability = (!token_probabilities.is_empty())
        .then(|| token_probabilities.iter().sum::<f64>() / token_probabilities.len() as f64);
    let start_seconds = segment
        .offsets
        .as_ref()
        .map(|offsets| offsets.from as f64 / 1000.0)
        .or_else(|| {
            segment
                .tokens
                .iter()
                .filter_map(|token| token.offsets.as_ref())
                .map(|offsets| offsets.from as f64 / 1000.0)
                .min_by(|left, right| left.total_cmp(right))
        })
        .unwrap_or_default();
    let end_seconds = segment
        .offsets
        .as_ref()
        .map(|offsets| offsets.to as f64 / 1000.0)
        .or_else(|| {
            segment
                .tokens
                .iter()
                .filter_map(|token| token.offsets.as_ref())
                .map(|offsets| offsets.to as f64 / 1000.0)
                .max_by(|left, right| left.total_cmp(right))
        })
        .unwrap_or(start_seconds);

    Some(CaptionQualitySegment {
        start_seconds,
        end_seconds: end_seconds.max(start_seconds),
        text,
        probability,
    })
}

fn caption_quality_flags(
    segments: &[CaptionQualitySegment],
    low_threshold: f64,
    warn_threshold: f64,
) -> Vec<CaptionQualityFlag> {
    segments
        .iter()
        .filter_map(|segment| {
            let reason = match segment.probability {
                Some(probability) if probability < low_threshold => {
                    "Very low confidence caption line."
                }
                Some(probability)
                    if probability < warn_threshold
                        && segment.text.split_whitespace().count() >= 5 =>
                {
                    "Low confidence on a longer caption line."
                }
                Some(_) => return None,
                None => "No word confidence data was available for this caption line.",
            };
            Some(CaptionQualityFlag {
                start_seconds: segment.start_seconds,
                end_seconds: segment.end_seconds,
                text: segment.text.clone(),
                probability: segment.probability,
                reason: reason.to_string(),
            })
        })
        .collect()
}

fn format_caption_review_document(
    quality: &CaptionQualitySummary,
    document: Option<&WhisperDocument>,
) -> String {
    let flags = document
        .map(|document| {
            let segments = document
                .transcription
                .iter()
                .filter_map(caption_quality_segment)
                .collect::<Vec<_>>();
            let probabilities = segments
                .iter()
                .filter_map(|segment| segment.probability)
                .collect::<Vec<_>>();
            let average = (!probabilities.is_empty())
                .then(|| probabilities.iter().sum::<f64>() / probabilities.len() as f64);
            let low_threshold = average
                .map(|value| 0.45_f64.min(0.34_f64.max(value - 0.22)))
                .unwrap_or(0.45);
            let warn_threshold = average
                .map(|value| 0.58_f64.min(0.46_f64.max(value - 0.14)))
                .unwrap_or(0.58);
            caption_quality_flags(&segments, low_threshold, warn_threshold)
                .into_iter()
                .take(24)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mut lines = vec!["RADcast Caption Review".to_string(), String::new()];
    if let Some(average) = quality.average_probability {
        lines.push(format!("Average word confidence: {:.0}%", average * 100.0));
    }
    lines.push(format!(
        "Low-confidence caption lines: {}",
        quality.low_confidence_segment_count
    ));
    lines.push(format!(
        "Total caption lines: {}",
        quality.total_segment_count
    ));
    lines.push(String::new());
    if flags.is_empty() {
        lines.push("No specific caption lines were flagged.".to_string());
    } else {
        lines.push("Review these timestamp ranges:".to_string());
        lines.push(String::new());
        for flag in flags {
            lines.push(format!(
                "{} --> {} | confidence {}",
                format_review_timestamp(flag.start_seconds),
                format_review_timestamp(flag.end_seconds.max(flag.start_seconds + 0.2)),
                flag.probability
                    .map(|probability| format!("{:.0}%", probability * 100.0))
                    .unwrap_or_else(|| "unknown".to_string())
            ));
            lines.push(format!("Reason: {}", flag.reason));
            lines.push(flag.text);
            lines.push(String::new());
        }
    }
    lines.join("\n") + "\n"
}

fn format_review_timestamp(seconds: f64) -> String {
    let total_milliseconds = (seconds.max(0.0) * 1000.0).round() as u64;
    let minutes = total_milliseconds / 60_000;
    let seconds = (total_milliseconds % 60_000) / 1000;
    let milliseconds = total_milliseconds % 1000;
    format!("{minutes:02}:{seconds:02}.{milliseconds:03}")
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
