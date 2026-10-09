use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::Utc;
use radsuite_engines::{
    AudioOutputFormat, AudioProcessingRequest, AudioProcessor, AudioTimeInterval, CaptionFormat,
    CaptionProcessingRequest, CaptionProcessor, CaptionQualityMode, CaptionQualitySummary,
    CaptionTranscriptionRequest, DetectedSilence, EnhancementModel, EnhancementProcessingRequest,
    EnhancementProcessor, EnhancementQuality, FillerRemovalMode,
    RADCAST_NATURAL_DOUBLE_PLUS_POSTFILTER, RADCAST_NATURAL_PLUS_POSTFILTER,
    RADCAST_NATURAL_POSTFILTER, RADCAST_OPTIMIZED_POSTFILTER, RADCAST_STANDARD_POSTFILTER,
    RADCAST_STANDARD_PREFILTER, RADCAST_STUDIO_POSTFILTER, SpeechCleanupError,
};

fn silence_plan(
    silences: &[DetectedSilence],
    clip_start: f64,
    clip_end: f64,
    keep_percent: f64,
    target_duration: Option<f64>,
) -> (Vec<AudioTimeInterval>, RadcastSilenceAnalysis) {
    let duration = (clip_end - clip_start).max(0.0);
    let qualifying: Vec<(f64, f64)> = silences
        .iter()
        .filter_map(|item| {
            let start = item.start_seconds.max(clip_start);
            let end = item.end_seconds.min(clip_end);
            (end > start && end - start >= 0.0).then_some((start - clip_start, end - clip_start))
        })
        .collect();
    let silence_total: f64 = qualifying.iter().map(|(s, e)| e - s).sum();
    let max_removal: f64 = qualifying
        .iter()
        .map(|(s, e)| (e - s - 0.10).max(0.0))
        .sum();
    let shortest = (duration - max_removal).max(0.0);
    let requested_removal = target_duration
        .map(|target| (duration - target).max(0.0))
        .unwrap_or(silence_total * (1.0 - keep_percent.clamp(0.0, 100.0) / 100.0));
    let actual_removal = requested_removal.min(max_removal);
    let scale = if max_removal > 0.0 {
        actual_removal / max_removal
    } else {
        0.0
    };
    let intervals = qualifying
        .iter()
        .filter_map(|(start, end)| {
            let span = end - start;
            let remove = ((span - 0.10).max(0.0) * scale).min(span);
            (remove > 0.001).then_some(AudioTimeInterval {
                start_seconds: start + (span - remove) / 2.0,
                end_seconds: end - (span - remove) / 2.0,
            })
        })
        .collect();
    let result_duration = duration - actual_removal;
    let retained = if silence_total > 0.0 {
        ((silence_total - actual_removal) / silence_total * 100.0).clamp(0.0, 100.0)
    } else {
        100.0
    };
    (
        intervals,
        RadcastSilenceAnalysis {
            original_duration_seconds: duration,
            qualifying_silence_count: qualifying.len(),
            qualifying_silence_seconds: silence_total,
            shortest_duration_seconds: shortest,
            estimated_duration_seconds: result_duration,
            retained_silence_percent: retained,
            target_reachable: target_duration.is_none_or(|target| target >= shortest),
        },
    )
}
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;
use uuid::Uuid;

const RADCAST_ROOT: &str = "radcast";

fn default_filler_removal_mode() -> FillerRemovalMode {
    FillerRemovalMode::Aggressive
}

fn default_caption_quality_mode() -> CaptionQualityMode {
    CaptionQualityMode::Reviewed
}

fn default_enhancement_model() -> EnhancementModel {
    EnhancementModel::None
}

fn default_project_enhancement_model() -> EnhancementModel {
    EnhancementModel::StudioV18
}

fn default_enhancement_quality() -> EnhancementQuality {
    EnhancementQuality::High
}

fn default_output_format() -> AudioOutputFormat {
    AudioOutputFormat::Mp3
}

fn default_cleanup_enabled() -> bool {
    true
}

fn effective_cleanup_enabled(enhancement_model: EnhancementModel, cleanup_enabled: bool) -> bool {
    enhancement_model == EnhancementModel::None && cleanup_enabled
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRadcastAudioRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
    pub path: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRadcastAudioLinkRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRadcastAudioRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRadcastAudioRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
    pub source_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessRadcastAudioRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
    pub source_id: String,
    pub output_format: AudioOutputFormat,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
    pub cleanup_enabled: bool,
    #[serde(default)]
    pub max_silence_seconds: Option<f64>,
    #[serde(default)]
    pub silence_shortening_enabled: bool,
    #[serde(default = "default_silence_minimum_seconds")]
    pub silence_minimum_seconds: f64,
    #[serde(default = "default_silence_threshold_db")]
    pub silence_threshold_db: f64,
    #[serde(default = "default_silence_keep_percent")]
    pub silence_keep_percent: f64,
    #[serde(default)]
    pub target_duration_seconds: Option<f64>,
    #[serde(default)]
    pub caption_format: Option<CaptionFormat>,
    #[serde(default = "default_caption_language")]
    pub caption_language: String,
    #[serde(default = "default_caption_quality_mode")]
    pub caption_quality_mode: CaptionQualityMode,
    #[serde(default)]
    pub caption_glossary: Option<String>,
    #[serde(default = "default_enhancement_model")]
    pub enhancement_model: EnhancementModel,
    #[serde(default = "default_enhancement_quality")]
    pub enhancement_quality: EnhancementQuality,
    #[serde(default)]
    pub remove_filler_words: bool,
    #[serde(default = "default_filler_removal_mode")]
    pub filler_removal_mode: FillerRemovalMode,
}

fn default_silence_minimum_seconds() -> f64 {
    2.0
}
fn default_silence_threshold_db() -> f64 {
    -40.0
}
fn default_silence_keep_percent() -> f64 {
    50.0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyzeRadcastSilenceRequest {
    pub project_id: Option<radsuite_core::ProjectId>,
    pub source_id: String,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
    pub minimum_seconds: f64,
    pub threshold_db: f64,
    pub keep_percent: f64,
    pub target_duration_seconds: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastSilenceAnalysis {
    pub original_duration_seconds: f64,
    pub qualifying_silence_count: usize,
    pub qualifying_silence_seconds: f64,
    pub shortest_duration_seconds: f64,
    pub estimated_duration_seconds: f64,
    pub retained_silence_percent: f64,
    pub target_reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastTrimRange {
    pub clip_start_seconds: f64,
    pub clip_end_seconds: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastProjectSettings {
    #[serde(default = "default_output_format")]
    pub output_format: AudioOutputFormat,
    #[serde(default)]
    pub caption_format: Option<CaptionFormat>,
    #[serde(default = "default_caption_language")]
    pub caption_language: String,
    #[serde(default = "default_caption_quality_mode")]
    pub caption_quality_mode: CaptionQualityMode,
    #[serde(default)]
    pub caption_glossary: Option<String>,
    #[serde(default = "default_project_enhancement_model")]
    pub enhancement_model: EnhancementModel,
    #[serde(default = "default_enhancement_quality")]
    pub enhancement_quality: EnhancementQuality,
    #[serde(default = "default_cleanup_enabled")]
    pub cleanup_enabled: bool,
    #[serde(default)]
    pub max_silence_seconds: Option<f64>,
    #[serde(default)]
    pub silence_shortening_enabled: bool,
    #[serde(default = "default_silence_minimum_seconds")]
    pub silence_minimum_seconds: f64,
    #[serde(default = "default_silence_threshold_db")]
    pub silence_threshold_db: f64,
    #[serde(default = "default_silence_keep_percent")]
    pub silence_keep_percent: f64,
    #[serde(default)]
    pub target_duration_seconds: Option<f64>,
    #[serde(default)]
    pub remove_filler_words: bool,
    #[serde(default = "default_filler_removal_mode")]
    pub filler_removal_mode: FillerRemovalMode,
    #[serde(default)]
    pub trim_ranges_by_source_id: HashMap<String, RadcastTrimRange>,
}

impl Default for RadcastProjectSettings {
    fn default() -> Self {
        Self {
            output_format: default_output_format(),
            caption_format: None,
            caption_language: default_caption_language(),
            caption_quality_mode: default_caption_quality_mode(),
            caption_glossary: None,
            enhancement_model: default_project_enhancement_model(),
            enhancement_quality: default_enhancement_quality(),
            cleanup_enabled: default_cleanup_enabled(),
            max_silence_seconds: None,
            silence_shortening_enabled: false,
            silence_minimum_seconds: default_silence_minimum_seconds(),
            silence_threshold_db: default_silence_threshold_db(),
            silence_keep_percent: default_silence_keep_percent(),
            target_duration_seconds: None,
            remove_filler_words: false,
            filler_removal_mode: default_filler_removal_mode(),
            trim_ranges_by_source_id: HashMap::new(),
        }
    }
}

impl RadcastProjectSettings {
    pub fn from_request(request: &ProcessRadcastAudioRequest) -> Self {
        Self {
            output_format: request.output_format,
            caption_format: request.caption_format,
            caption_language: request.caption_language.clone(),
            caption_quality_mode: request.caption_quality_mode,
            caption_glossary: request.caption_glossary.clone(),
            enhancement_model: request.enhancement_model,
            enhancement_quality: request.enhancement_quality,
            cleanup_enabled: request.cleanup_enabled,
            max_silence_seconds: request.max_silence_seconds,
            silence_shortening_enabled: request.silence_shortening_enabled,
            silence_minimum_seconds: request.silence_minimum_seconds,
            silence_threshold_db: request.silence_threshold_db,
            silence_keep_percent: request.silence_keep_percent,
            target_duration_seconds: request.target_duration_seconds,
            remove_filler_words: request.remove_filler_words,
            filler_removal_mode: request.filler_removal_mode,
            trim_ranges_by_source_id: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastAudioSource {
    pub id: String,
    pub original_filename: String,
    pub path: String,
    pub duration_seconds: f64,
    pub byte_size: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastAudioOutput {
    pub id: String,
    pub source_id: String,
    pub filename: String,
    pub path: String,
    pub duration_seconds: f64,
    pub output_format: AudioOutputFormat,
    pub cleanup_enabled: bool,
    #[serde(default)]
    pub studio_qa_path: Option<String>,
    #[serde(default)]
    pub studio_qa_warnings: Vec<String>,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
    #[serde(default)]
    pub max_silence_seconds: Option<f64>,
    #[serde(default)]
    pub caption_path: Option<String>,
    #[serde(default)]
    pub caption_format: Option<CaptionFormat>,
    #[serde(default = "default_caption_quality_mode")]
    pub caption_quality_mode: CaptionQualityMode,
    #[serde(default)]
    pub caption_glossary: Option<String>,
    #[serde(default = "default_enhancement_model")]
    pub enhancement_model: EnhancementModel,
    #[serde(default = "default_enhancement_quality")]
    pub enhancement_quality: EnhancementQuality,
    #[serde(default)]
    pub caption_segment_count: usize,
    #[serde(default)]
    pub caption_review_path: Option<String>,
    #[serde(default)]
    pub caption_review_required: bool,
    #[serde(default)]
    pub caption_average_probability: Option<f64>,
    #[serde(default)]
    pub caption_low_confidence_segments: usize,
    #[serde(default)]
    pub caption_total_segments: usize,
    #[serde(default)]
    pub remove_filler_words: bool,
    #[serde(default = "default_filler_removal_mode")]
    pub filler_removal_mode: FillerRemovalMode,
    #[serde(default)]
    pub removed_filler_count: usize,
    #[serde(default)]
    pub removed_pause_count: usize,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadcastAudioListing {
    pub sources: Vec<RadcastAudioSource>,
    pub outputs: Vec<RadcastAudioOutput>,
    pub settings: RadcastProjectSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadcastProcessingPhase {
    Preparing,
    RemovingFillerWords,
    PreparingEnhancement,
    EnhancingAudio,
    RenderingAudio,
    GeneratingCaptions,
    SavingOutput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadcastProcessingProgress {
    pub phase: RadcastProcessingPhase,
    pub percent: u8,
}

#[derive(Debug, Error)]
pub enum RadcastStorageError {
    #[error("choose an audio file before importing it")]
    EmptyPath,
    #[error("paste a OneDrive or SharePoint sharing link before importing it")]
    EmptyLink,
    #[error("only OneDrive or SharePoint sharing links are supported")]
    UnsupportedLink,
    #[error(
        "this OneDrive link needs Microsoft sign-in or access permission. Open it in OneDrive, download the audio file, and choose it from RADcast instead"
    )]
    LinkRequiresSignIn,
    #[error("could not download the OneDrive audio: {0}")]
    LinkDownload(String),
    #[error("could not determine the audio filename")]
    MissingFilename,
    #[error("audio source does not exist: {path}")]
    MissingInput { path: PathBuf },
    #[error(
        "could not copy selected audio file from '{source_path}' to RADcast project storage at '{destination}': {source}"
    )]
    SourceCopy {
        source_path: PathBuf,
        destination: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "could not copy selected cloud audio file from '{source_path}' to RADcast project storage at '{destination}': {source}. In OneDrive or iCloud, choose 'Always Keep on This Device' or move the file to a local folder, wait for the download to finish, then retry."
    )]
    CloudSourceCopy {
        source_path: PathBuf,
        destination: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("saved audio source was not found: {0}")]
    MissingSource(String),
    #[error("invalid RADcast request: {0}")]
    InvalidRequest(String),
    #[error("failed to access RADcast project storage")]
    Io(#[from] std::io::Error),
    #[error("failed to read RADcast project manifest")]
    ManifestRead(#[source] serde_json::Error),
    #[error("failed to write RADcast project manifest")]
    ManifestWrite(#[source] serde_json::Error),
    #[error("failed to process audio: {0}")]
    Processing(#[from] radsuite_engines::AudioProcessingError),
    #[error("failed to generate captions: {0}")]
    CaptionProcessing(#[from] radsuite_engines::CaptionProcessingError),
    #[error("failed to plan speech-aware cleanup: {0}")]
    SpeechCleanup(#[from] SpeechCleanupError),
    #[error("failed to enhance audio: {0}")]
    EnhancementProcessing(#[from] radsuite_engines::EnhancementProcessingError),
    #[error("local RADcast processing was cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct RadcastManifest {
    sources: Vec<RadcastAudioSource>,
    outputs: Vec<RadcastAudioOutput>,
    #[serde(default)]
    settings: RadcastProjectSettings,
}

pub(crate) fn list_audio(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
) -> Result<RadcastAudioListing, RadcastStorageError> {
    let manifest = load_manifest(data_dir, project_id)?;
    Ok(RadcastAudioListing {
        sources: manifest.sources,
        outputs: manifest.outputs,
        settings: manifest.settings,
    })
}

pub(crate) fn analyze_audio_silence(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: AnalyzeRadcastSilenceRequest,
    processor: AudioProcessor,
) -> Result<RadcastSilenceAnalysis, RadcastStorageError> {
    if !request.keep_percent.is_finite()
        || !(0.0..=100.0).contains(&request.keep_percent)
        || request
            .target_duration_seconds
            .is_some_and(|target| !target.is_finite() || target <= 0.0)
    {
        return Err(RadcastStorageError::InvalidRequest(
            "invalid silence retention or target duration".to_string(),
        ));
    }
    let manifest = load_manifest(data_dir, project_id)?;
    let source = manifest
        .sources
        .iter()
        .find(|source| source.id == request.source_id)
        .ok_or_else(|| RadcastStorageError::MissingSource(request.source_id.clone()))?;
    let start = request.clip_start_seconds.unwrap_or(0.0);
    let end = request.clip_end_seconds.unwrap_or(source.duration_seconds);
    if !start.is_finite()
        || !end.is_finite()
        || start < 0.0
        || end <= start
        || end > source.duration_seconds
    {
        return Err(RadcastStorageError::InvalidRequest(
            "invalid clip range".to_string(),
        ));
    }
    let detected = processor.detect_silences(
        Path::new(&source.path),
        request.threshold_db,
        request.minimum_seconds,
    )?;
    Ok(silence_plan(
        &detected,
        start,
        end,
        request.keep_percent,
        request.target_duration_seconds,
    )
    .1)
}

pub(crate) fn delete_audio(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: DeleteRadcastAudioRequest,
) -> Result<(), RadcastStorageError> {
    let mut manifest = load_manifest(data_dir, project_id)?;
    let source_index = manifest
        .sources
        .iter()
        .position(|source| source.id == request.source_id)
        .ok_or_else(|| RadcastStorageError::MissingSource(request.source_id.clone()))?;
    let source = manifest.sources.remove(source_index);
    manifest
        .settings
        .trim_ranges_by_source_id
        .remove(&source.id);

    write_manifest(data_dir, project_id, &manifest)?;

    let source_path = PathBuf::from(source.path);
    let project_root = project_root(data_dir, project_id);
    if let (Ok(resolved_path), Ok(resolved_root)) =
        (source_path.canonicalize(), project_root.canonicalize())
        && resolved_path.starts_with(resolved_root)
        && resolved_path.is_file()
    {
        fs::remove_file(resolved_path)?;
    }

    Ok(())
}

pub(crate) fn save_settings(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    settings: RadcastProjectSettings,
) -> Result<RadcastProjectSettings, RadcastStorageError> {
    let mut manifest = load_manifest(data_dir, project_id)?;
    manifest.settings = settings.clone();
    write_manifest(data_dir, project_id, &manifest)?;
    Ok(settings)
}

pub(crate) fn import_audio(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ImportRadcastAudioRequest,
    processor: AudioProcessor,
) -> Result<RadcastAudioSource, RadcastStorageError> {
    let path = request.path.trim();
    if path.is_empty() {
        return Err(RadcastStorageError::EmptyPath);
    }

    let source_path = PathBuf::from(path);
    if !source_path.is_file() {
        return Err(RadcastStorageError::MissingInput { path: source_path });
    }

    let original_filename = request
        .original_filename
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            source_path
                .file_name()
                .and_then(|filename| filename.to_str())
                .map(str::to_string)
        })
        .ok_or(RadcastStorageError::MissingFilename)?;

    let id = Uuid::new_v4().to_string();
    let project_root = project_root(data_dir, project_id);
    let sources_dir = project_root.join("sources");
    fs::create_dir_all(&sources_dir)?;
    let destination = sources_dir.join(format!("{}-{}", id, safe_filename(&original_filename)));
    if let Err(source) = fs::copy(&source_path, &destination) {
        let _ = fs::remove_file(&destination);
        let _ = fs::remove_dir(&sources_dir);
        return Err(if is_cloud_storage_path(&source_path) {
            RadcastStorageError::CloudSourceCopy {
                source_path,
                destination,
                source,
            }
        } else {
            RadcastStorageError::SourceCopy {
                source_path,
                destination,
                source,
            }
        });
    }
    let duration_seconds = match processor.probe_duration(&destination) {
        Ok(duration_seconds) => duration_seconds,
        Err(error) => {
            let _ = fs::remove_file(&destination);
            return Err(error.into());
        }
    };
    let byte_size = fs::metadata(&destination)?.len();

    let source = RadcastAudioSource {
        id,
        original_filename,
        path: destination.to_string_lossy().into_owned(),
        duration_seconds,
        byte_size,
        created_at: Utc::now().to_rfc3339(),
    };
    let mut manifest = load_manifest(data_dir, project_id)?;
    manifest.sources.push(source.clone());
    if let Err(error) = write_manifest(data_dir, project_id, &manifest) {
        let _ = fs::remove_file(destination);
        return Err(error);
    }
    Ok(source)
}

pub(crate) fn import_audio_from_link(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ImportRadcastAudioLinkRequest,
    processor: AudioProcessor,
) -> Result<RadcastAudioSource, RadcastStorageError> {
    let (download_path, filename) = download_audio_link(&request.url)?;
    let import_result = import_audio(
        data_dir,
        project_id,
        ImportRadcastAudioRequest {
            project_id: Some(project_id),
            path: download_path.to_string_lossy().into_owned(),
            original_filename: Some(filename),
        },
        processor,
    );
    let _ = fs::remove_file(&download_path);
    import_result
}

pub(crate) fn process_audio_with_processors(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
) -> Result<RadcastAudioOutput, RadcastStorageError> {
    process_audio_with_processors_and_enhancement(
        data_dir,
        project_id,
        request,
        processor,
        caption_processor,
        EnhancementProcessor::default(),
    )
}

pub(crate) fn process_audio_with_processors_and_enhancement(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
    enhancement_processor: EnhancementProcessor,
) -> Result<RadcastAudioOutput, RadcastStorageError> {
    process_audio_with_processors_and_enhancement_with_progress(
        data_dir,
        project_id,
        request,
        processor,
        caption_processor,
        enhancement_processor,
        |_| {},
    )
}

pub fn process_audio_with_processors_and_enhancement_with_progress<F>(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
    enhancement_processor: EnhancementProcessor,
    report_progress: F,
) -> Result<RadcastAudioOutput, RadcastStorageError>
where
    F: FnMut(RadcastProcessingProgress),
{
    process_audio_with_processors_and_enhancement_with_progress_and_cancellation(
        data_dir,
        project_id,
        request,
        processor,
        caption_processor,
        enhancement_processor,
        report_progress,
        || false,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn process_audio_with_processors_and_enhancement_with_progress_and_cancellation<F, C>(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    request: ProcessRadcastAudioRequest,
    processor: AudioProcessor,
    caption_processor: CaptionProcessor,
    enhancement_processor: EnhancementProcessor,
    mut report_progress: F,
    mut is_cancelled: C,
) -> Result<RadcastAudioOutput, RadcastStorageError>
where
    F: FnMut(RadcastProcessingProgress),
    C: FnMut() -> bool,
{
    if request.silence_shortening_enabled
        && (!request.silence_keep_percent.is_finite()
            || !(0.0..=100.0).contains(&request.silence_keep_percent)
            || request
                .target_duration_seconds
                .is_some_and(|target| !target.is_finite() || target <= 0.0))
    {
        return Err(RadcastStorageError::InvalidRequest(
            "invalid silence retention or target duration".to_string(),
        ));
    }
    if is_cancelled() {
        return Err(RadcastStorageError::Cancelled);
    }
    report_progress(RadcastProcessingProgress {
        phase: RadcastProcessingPhase::Preparing,
        percent: 5,
    });
    let mut manifest = load_manifest(data_dir, project_id)?;
    let source = manifest
        .sources
        .iter()
        .find(|source| source.id == request.source_id)
        .cloned()
        .ok_or_else(|| RadcastStorageError::MissingSource(request.source_id.clone()))?;
    let mut project_settings = RadcastProjectSettings::from_request(&request);
    project_settings.trim_ranges_by_source_id = manifest.settings.trim_ranges_by_source_id.clone();
    if let (Some(clip_start_seconds), Some(clip_end_seconds)) =
        (request.clip_start_seconds, request.clip_end_seconds)
    {
        project_settings.trim_ranges_by_source_id.insert(
            request.source_id.clone(),
            RadcastTrimRange {
                clip_start_seconds,
                clip_end_seconds,
            },
        );
    }
    let source_path = PathBuf::from(&source.path);
    if !source_path.is_file() {
        return Err(RadcastStorageError::MissingSource(source.id));
    }
    if is_cancelled() {
        return Err(RadcastStorageError::Cancelled);
    }

    let output_id = Uuid::new_v4().to_string();
    let output_filename = format!(
        "{}-radcast-{}.{}",
        safe_stem(&source.original_filename),
        &output_id[..8],
        request.output_format.extension()
    );
    let output_path = project_root(data_dir, project_id)
        .join("outputs")
        .join(&output_filename);
    let cleanup_plan = if request.max_silence_seconds.is_some() || request.remove_filler_words {
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::RemovingFillerWords,
            percent: 12,
        });
        let clip_start_seconds = request.clip_start_seconds.unwrap_or(0.0);
        let clip_end_seconds = request.clip_end_seconds.unwrap_or(source.duration_seconds);
        let clip_duration_seconds = clip_end_seconds - clip_start_seconds;
        Some(
            caption_processor
                .speech_cleanup_plan_with_cancellation(
                    &CaptionTranscriptionRequest {
                        input_path: source_path.clone(),
                        language: request.caption_language.trim().to_string(),
                        clip_start_seconds: request.clip_start_seconds,
                        clip_end_seconds: request.clip_end_seconds,
                    },
                    clip_duration_seconds,
                    request.max_silence_seconds,
                    request.remove_filler_words,
                    request.filler_removal_mode,
                    &mut is_cancelled,
                )
                .map_err(|error| match error {
                    SpeechCleanupError::Cancelled => RadcastStorageError::Cancelled,
                    other => RadcastStorageError::SpeechCleanup(other),
                })?,
        )
    } else {
        None
    };
    if is_cancelled() {
        return Err(RadcastStorageError::Cancelled);
    }
    let mut removal_intervals = cleanup_plan
        .as_ref()
        .map(|plan| plan.removal_intervals.clone())
        .unwrap_or_default();
    if request.silence_shortening_enabled {
        let start = request.clip_start_seconds.unwrap_or(0.0);
        let end = request.clip_end_seconds.unwrap_or(source.duration_seconds);
        let detected = processor.detect_silences(
            &source_path,
            request.silence_threshold_db,
            request.silence_minimum_seconds,
        )?;
        let (silence_intervals, _) = silence_plan(
            &detected,
            start,
            end,
            request.silence_keep_percent,
            request.target_duration_seconds,
        );
        removal_intervals.extend(silence_intervals);
        removal_intervals.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));
        let mut merged: Vec<AudioTimeInterval> = Vec::new();
        for interval in removal_intervals.drain(..) {
            if let Some(previous) = merged
                .last_mut()
                .filter(|p| interval.start_seconds <= p.end_seconds)
            {
                previous.end_seconds = previous.end_seconds.max(interval.end_seconds);
            } else {
                merged.push(interval);
            }
        }
        removal_intervals = merged;
    }
    let removed_pause_count = cleanup_plan
        .as_ref()
        .map(|plan| plan.removed_pause_count)
        .unwrap_or_default();
    let removed_filler_count = cleanup_plan
        .as_ref()
        .map(|plan| plan.removed_filler_count)
        .unwrap_or_default();
    let mut temporary_paths = Vec::new();
    let processing_input_path = if request.enhancement_model != EnhancementModel::None {
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::PreparingEnhancement,
            percent: 20,
        });
        let prepared_path = output_path.with_file_name(format!(".{output_id}-prepared.wav"));
        let enhanced_path = output_path.with_file_name(format!(".{output_id}-enhanced.wav"));
        let preparation_filter = match request.enhancement_model {
            EnhancementModel::Resemble
            | EnhancementModel::DeepFilterNet
            | EnhancementModel::Studio => Some(RADCAST_STANDARD_PREFILTER),
            EnhancementModel::StudioV18
            | EnhancementModel::StudioV18Natural
            | EnhancementModel::StudioV18NaturalPlus
            | EnhancementModel::StudioV18NaturalDoublePlus
            | EnhancementModel::StudioV1
            | EnhancementModel::StudioTreble
            | EnhancementModel::None => None,
        };
        let preparation_request = AudioProcessingRequest {
            input_path: source_path.clone(),
            output_path: prepared_path.clone(),
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: request.clip_start_seconds,
            clip_end_seconds: request.clip_end_seconds,
            max_silence_seconds: None,
            remove_intervals: Vec::new(),
            cleanup_enabled: false,
        };
        let preparation = if request.enhancement_model.is_guarded_studio() {
            processor.process_studio_with_additional_filter(preparation_request, None)
        } else {
            processor.process_with_additional_filter(preparation_request, preparation_filter)
        };
        if let Err(error) = preparation {
            cleanup_temporary_paths(&[
                prepared_path.clone(),
                enhanced_path.clone(),
                enhanced_path.with_extension("qa.json"),
            ]);
            return Err(error.into());
        }
        if is_cancelled() {
            cleanup_temporary_paths(&[
                prepared_path.clone(),
                enhanced_path.clone(),
                enhanced_path.with_extension("qa.json"),
            ]);
            return Err(RadcastStorageError::Cancelled);
        }
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::EnhancingAudio,
            percent: 35,
        });
        let enhancement_result = enhancement_processor.process_model_with_quality_and_progress(
            EnhancementProcessingRequest {
                input_path: prepared_path.clone(),
                output_path: enhanced_path.clone(),
            },
            request.enhancement_model,
            request.enhancement_quality,
            |completed, total| {
                let progress = if total == 0 {
                    0.0
                } else {
                    (completed as f64 / total as f64).clamp(0.0, 1.0)
                };
                report_progress(RadcastProcessingProgress {
                    phase: RadcastProcessingPhase::EnhancingAudio,
                    percent: (35.0 + (43.0 * progress)).round() as u8,
                });
            },
        );
        if let Err(error) = enhancement_result {
            cleanup_temporary_paths(&[
                prepared_path.clone(),
                enhanced_path.clone(),
                enhanced_path.with_extension("qa.json"),
            ]);
            return Err(error.into());
        }
        if is_cancelled() {
            cleanup_temporary_paths(&[
                prepared_path.clone(),
                enhanced_path.clone(),
                enhanced_path.with_extension("qa.json"),
            ]);
            return Err(RadcastStorageError::Cancelled);
        }
        if request.enhancement_model.is_guarded_studio() {
            temporary_paths.push(enhanced_path.with_extension("qa.json"));
        }
        temporary_paths.extend([prepared_path, enhanced_path.clone()]);
        enhanced_path
    } else {
        source_path.clone()
    };
    report_progress(RadcastProcessingProgress {
        phase: RadcastProcessingPhase::RenderingAudio,
        percent: if request.enhancement_model != EnhancementModel::None {
            78
        } else {
            35
        },
    });
    let (clip_start_seconds, clip_end_seconds) =
        if request.enhancement_model == EnhancementModel::None {
            (request.clip_start_seconds, request.clip_end_seconds)
        } else {
            match (request.clip_start_seconds, request.clip_end_seconds) {
                (Some(start), Some(end)) if end > start => (Some(0.0), Some(end - start)),
                (Some(start), None) if source.duration_seconds > start => {
                    (Some(0.0), Some(source.duration_seconds - start))
                }
                (None, Some(end)) if end > 0.0 => (Some(0.0), Some(end)),
                _ => (None, None),
            }
        };
    let additional_filter = match request.enhancement_model {
        EnhancementModel::StudioV1 | EnhancementModel::StudioTreble => None,
        EnhancementModel::Resemble | EnhancementModel::DeepFilterNet => {
            Some(RADCAST_STANDARD_POSTFILTER)
        }
        EnhancementModel::Studio => Some(RADCAST_STUDIO_POSTFILTER),
        EnhancementModel::StudioV18 => Some(RADCAST_OPTIMIZED_POSTFILTER),
        EnhancementModel::StudioV18Natural => Some(RADCAST_NATURAL_POSTFILTER),
        EnhancementModel::StudioV18NaturalPlus => Some(RADCAST_NATURAL_PLUS_POSTFILTER),
        EnhancementModel::StudioV18NaturalDoublePlus => {
            Some(RADCAST_NATURAL_DOUBLE_PLUS_POSTFILTER)
        }
        EnhancementModel::None => None,
    };
    let exact_duration_filter = if request.enhancement_model != EnhancementModel::None
        && removal_intervals.is_empty()
    {
        clip_end_seconds
            .map(|duration| format!("apad=whole_dur={duration:.3},atrim=duration={duration:.3}"))
    } else {
        None
    };
    let final_filter = match (additional_filter, exact_duration_filter.as_deref()) {
        (Some(base), Some(duration_filter)) => Some(format!("{base},{duration_filter}")),
        (Some(base), None) => Some(base.to_string()),
        (None, Some(duration_filter)) => Some(duration_filter.to_string()),
        (None, None) => None,
    };
    let cleanup_enabled =
        effective_cleanup_enabled(request.enhancement_model, request.cleanup_enabled);
    let final_request = AudioProcessingRequest {
        input_path: processing_input_path,
        output_path: output_path.clone(),
        output_format: request.output_format,
        clip_start_seconds,
        clip_end_seconds,
        max_silence_seconds: None,
        remove_intervals: removal_intervals,
        cleanup_enabled,
    };
    let timeline_edited = !final_request.remove_intervals.is_empty();
    let selected_duration = request.clip_end_seconds.unwrap_or(source.duration_seconds)
        - request.clip_start_seconds.unwrap_or(0.0);
    let removed_seconds = AudioProcessor::removal_duration_seconds(
        &final_request.remove_intervals,
        selected_duration,
    );
    let rendering = if request.enhancement_model.is_guarded_studio() {
        processor.process_studio_with_additional_filter(final_request, final_filter.as_deref())
    } else {
        processor.process_with_additional_filter(final_request, final_filter.as_deref())
    };
    let result = match rendering {
        Ok(result) => result,
        Err(error) => {
            cleanup_temporary_paths(&temporary_paths);
            return Err(error.into());
        }
    };
    if is_cancelled() {
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(output_path.with_extension("qa.json"));
        cleanup_temporary_paths(&temporary_paths);
        return Err(RadcastStorageError::Cancelled);
    }
    let mut studio_qa_warnings = Vec::new();
    let retained_qa = (|| -> Result<Option<String>, RadcastStorageError> {
        let path = if request.enhancement_model.is_guarded_studio() {
            let source = temporary_paths
                .iter()
                .find(|path| path.to_string_lossy().ends_with(".qa.json"));
            match source {
                Some(source) => {
                    let prepared = temporary_paths
                        .iter()
                        .find(|path| path.to_string_lossy().ends_with("-prepared.wav"))
                        .expect("Studio preparation recorded");
                    if let Err(error) = enhancement_processor.verify_guarded_studio_export(
                        request.enhancement_model,
                        prepared,
                        &output_path,
                        source,
                        &source_path,
                        timeline_edited,
                        removed_seconds,
                    ) {
                        let _ = fs::remove_file(&output_path);
                        let _ = fs::remove_file(output_path.with_extension("qa.json"));
                        cleanup_temporary_paths(&temporary_paths);
                        return Err(error.into());
                    }
                    let report: serde_json::Value =
                        serde_json::from_slice(&fs::read(source).map_err(|error| {
                            RadcastStorageError::InvalidRequest(error.to_string())
                        })?)
                        .map_err(|error| RadcastStorageError::InvalidRequest(error.to_string()))?;
                    if let Some(warnings) = report["warnings"].as_array() {
                        studio_qa_warnings = warnings
                            .iter()
                            .map(|warning| {
                                format!(
                                    "{}: {}",
                                    warning["code"].as_str().unwrap_or("warning"),
                                    warning["detail"]
                                        .as_str()
                                        .unwrap_or("Review Studio QA report")
                                )
                            })
                            .collect();
                    }
                    let qa_path = output_path.with_extension("qa.json");
                    fs::copy(source, &qa_path).map_err(|error| {
                        RadcastStorageError::InvalidRequest(format!(
                            "Could not retain Studio QA: {error}"
                        ))
                    })?;
                    Some(qa_path.to_string_lossy().into_owned())
                }
                None => {
                    return Err(RadcastStorageError::InvalidRequest(
                        "Studio QA report missing".to_string(),
                    ));
                }
            }
        } else {
            None
        };
        Ok(path)
    })();
    let studio_qa_path = match retained_qa {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_file(&output_path);
            let _ = fs::remove_file(output_path.with_extension("qa.json"));
            cleanup_temporary_paths(&temporary_paths);
            return Err(error);
        }
    };
    cleanup_temporary_paths(&temporary_paths);

    let (caption_path, caption_format, caption_segment_count, caption_quality) =
        if let Some(format) = request.caption_format {
            report_progress(RadcastProcessingProgress {
                phase: RadcastProcessingPhase::GeneratingCaptions,
                percent: 90,
            });
            let path = output_path.with_extension(format.extension());
            let caption_result = caption_processor.process_with_options(
                CaptionProcessingRequest {
                    input_path: output_path.clone(),
                    output_path: path.clone(),
                    caption_format: format,
                    language: request.caption_language.trim().to_string(),
                    clip_start_seconds: None,
                    clip_end_seconds: None,
                },
                request.caption_quality_mode,
                request.caption_glossary.as_deref(),
            );
            match caption_result {
                Ok(caption_result) => (
                    Some(caption_result.output_path.to_string_lossy().into_owned()),
                    Some(caption_result.caption_format),
                    caption_result.segment_count,
                    caption_result.quality,
                ),
                Err(error) => {
                    let _ = fs::remove_file(&output_path);
                    let _ = fs::remove_file(output_path.with_extension("qa.json"));
                    let _ = fs::remove_file(path);
                    cleanup_temporary_paths(&temporary_paths);
                    return Err(error.into());
                }
            }
        } else {
            (None, None, 0, CaptionQualitySummary::default())
        };
    if is_cancelled() {
        let _ = fs::remove_file(&output_path);
        let _ = fs::remove_file(output_path.with_extension("qa.json"));
        if let Some(caption_path) = caption_path.as_deref() {
            let _ = fs::remove_file(caption_path);
        }
        if let Some(review_path) = caption_quality.review_path.as_deref() {
            let _ = fs::remove_file(review_path);
        }
        return Err(RadcastStorageError::Cancelled);
    }

    report_progress(RadcastProcessingProgress {
        phase: RadcastProcessingPhase::SavingOutput,
        percent: 98,
    });

    let output = RadcastAudioOutput {
        id: output_id,
        source_id: source.id,
        filename: output_filename,
        path: result.output_path.to_string_lossy().into_owned(),
        duration_seconds: result.duration_seconds,
        output_format: result.output_format,
        cleanup_enabled,
        studio_qa_path,
        studio_qa_warnings,
        clip_start_seconds: request.clip_start_seconds,
        clip_end_seconds: request.clip_end_seconds,
        max_silence_seconds: request.max_silence_seconds,
        caption_path,
        caption_format,
        caption_quality_mode: request.caption_quality_mode,
        caption_glossary: request.caption_glossary,
        caption_review_path: caption_quality
            .review_path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned()),
        caption_review_required: caption_quality.review_recommended,
        caption_average_probability: caption_quality.average_probability,
        caption_low_confidence_segments: caption_quality.low_confidence_segment_count,
        caption_total_segments: caption_quality.total_segment_count,
        enhancement_model: request.enhancement_model,
        enhancement_quality: request.enhancement_quality,
        caption_segment_count,
        remove_filler_words: request.remove_filler_words,
        filler_removal_mode: request.filler_removal_mode,
        removed_filler_count,
        removed_pause_count,
        created_at: Utc::now().to_rfc3339(),
    };
    manifest.outputs.insert(0, output.clone());
    manifest.settings = project_settings;
    if let Err(error) = write_manifest(data_dir, project_id, &manifest) {
        let _ = fs::remove_file(output_path);
        cleanup_temporary_paths(&temporary_paths);
        if let Some(qa_path) = output.studio_qa_path.as_deref() {
            let _ = fs::remove_file(qa_path);
        }
        if let Some(caption_path) = output.caption_path.as_deref() {
            let _ = fs::remove_file(caption_path);
        }
        if let Some(review_path) = output.caption_review_path.as_deref() {
            let _ = fs::remove_file(review_path);
        }
        return Err(error);
    }
    Ok(output)
}

#[cfg(test)]
mod silence_plan_tests {
    use super::*;

    #[test]
    fn target_duration_scales_silence_reduction_and_reports_minimum() {
        let detected = vec![
            DetectedSilence {
                start_seconds: 0.0,
                end_seconds: 20.0,
            },
            DetectedSilence {
                start_seconds: 21.0,
                end_seconds: 29.0,
            },
        ];
        let (intervals, result) = silence_plan(&detected, 0.0, 30.0, 50.0, Some(1.5));
        let removed: f64 = intervals
            .iter()
            .map(|span| span.end_seconds - span.start_seconds)
            .sum();
        assert!((result.estimated_duration_seconds - 2.2).abs() < 0.01);
        assert!((removed - 27.8).abs() < 0.01);
        assert!(!result.target_reachable);
        assert!((result.shortest_duration_seconds - 2.2).abs() < 0.01);
    }

    #[test]
    fn keep_percentage_is_applied_proportionally_to_qualifying_silence() {
        let detected = vec![DetectedSilence {
            start_seconds: 1.0,
            end_seconds: 21.0,
        }];
        let (_, result) = silence_plan(&detected, 0.0, 25.0, 50.0, None);
        assert_eq!(result.qualifying_silence_count, 1);
        assert!((result.retained_silence_percent - 50.0).abs() < 0.01);
        assert!((result.estimated_duration_seconds - 15.0).abs() < 0.01);
    }
}

fn cleanup_temporary_paths(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn default_caption_language() -> String {
    "en".to_string()
}

fn project_root(data_dir: &Path, project_id: radsuite_core::ProjectId) -> PathBuf {
    data_dir
        .join(RADCAST_ROOT)
        .join("projects")
        .join(project_id.0.to_string())
}

fn manifest_path(data_dir: &Path, project_id: radsuite_core::ProjectId) -> PathBuf {
    project_root(data_dir, project_id).join("manifest.json")
}

fn load_manifest(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
) -> Result<RadcastManifest, RadcastStorageError> {
    let path = manifest_path(data_dir, project_id);
    if !path.is_file() {
        return Ok(RadcastManifest::default());
    }
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(RadcastStorageError::ManifestRead)
}

fn write_manifest(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
    manifest: &RadcastManifest,
) -> Result<(), RadcastStorageError> {
    let root = project_root(data_dir, project_id);
    fs::create_dir_all(&root)?;
    let contents =
        serde_json::to_string_pretty(manifest).map_err(RadcastStorageError::ManifestWrite)?;
    fs::write(manifest_path(data_dir, project_id), contents)?;
    Ok(())
}

fn safe_filename(filename: &str) -> String {
    let cleaned = filename
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if cleaned.is_empty() {
        "audio".to_string()
    } else {
        cleaned
    }
}

fn safe_stem(filename: &str) -> String {
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("audio");
    safe_filename(stem)
}

fn is_cloud_storage_path(path: &Path) -> bool {
    path.components()
        .any(|component| component.as_os_str().to_string_lossy() == "CloudStorage")
}

fn download_audio_link(raw_url: &str) -> Result<(PathBuf, String), RadcastStorageError> {
    let raw_url = raw_url.trim();
    if raw_url.is_empty() {
        return Err(RadcastStorageError::EmptyLink);
    }
    let url = Url::parse(raw_url).map_err(|_| RadcastStorageError::UnsupportedLink)?;
    if !is_supported_link(&url) {
        return Err(RadcastStorageError::UnsupportedLink);
    }

    let mut download_url = url.clone();
    if !download_url
        .query_pairs()
        .any(|(key, _value)| key.eq_ignore_ascii_case("download"))
    {
        download_url.query_pairs_mut().append_pair("download", "1");
    }

    let client = Client::builder()
        .user_agent("RADsuite RADcast local importer")
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| RadcastStorageError::LinkDownload(error.to_string()))?;
    let mut response = client
        .get(download_url)
        .send()
        .map_err(|error| RadcastStorageError::LinkDownload(error.to_string()))?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(RadcastStorageError::LinkRequiresSignIn);
    }
    if !response.status().is_success() {
        return Err(RadcastStorageError::LinkDownload(format!(
            "OneDrive returned HTTP {}",
            response.status()
        )));
    }
    if response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/html"))
    {
        return Err(RadcastStorageError::LinkRequiresSignIn);
    }

    let filename = response_filename(&response, &url);
    let temporary_path = std::env::temp_dir().join(format!(
        "radsuite-radcast-link-{}-{}",
        Uuid::new_v4(),
        safe_filename(&filename)
    ));
    let mut output = fs::File::create(&temporary_path).map_err(|error| {
        RadcastStorageError::LinkDownload(format!("could not create a temporary download: {error}"))
    })?;
    if let Err(error) = response.copy_to(&mut output) {
        let _ = fs::remove_file(&temporary_path);
        return Err(RadcastStorageError::LinkDownload(error.to_string()));
    }
    Ok((temporary_path, filename))
}

fn is_supported_link(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("1drv.ms")
        || host.eq_ignore_ascii_case("onedrive.live.com")
        || host.ends_with(".onedrive.live.com")
        || host.ends_with(".sharepoint.com")
}

fn response_filename(response: &Response, source_url: &Url) -> String {
    let header_filename = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value.split(';').find_map(|part| {
                let (key, filename) = part.trim().split_once('=')?;
                key.trim().eq_ignore_ascii_case("filename").then(|| {
                    filename
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string()
                })
            })
        })
        .filter(|value| !value.is_empty());
    let url_filename = source_url
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.contains(":/"))
        .map(|value| value.to_string());
    safe_filename(
        &header_filename
            .or(url_filename)
            .unwrap_or_else(|| "onedrive-audio".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::{effective_cleanup_enabled, is_cloud_storage_path, is_supported_link};
    use radsuite_engines::EnhancementModel;
    use std::path::Path;
    use url::Url;

    #[test]
    fn identifies_macos_cloud_storage_paths() {
        assert!(is_cloud_storage_path(Path::new(
            "/Users/example/Library/CloudStorage/OneDrive-Team/audio.wav"
        )));
        assert!(!is_cloud_storage_path(Path::new(
            "/Users/example/Documents/audio.wav"
        )));
    }

    #[test]
    fn accepts_onedrive_and_sharepoint_https_links_only() {
        assert!(is_supported_link(
            &Url::parse("https://1drv.ms/u/s!example").expect("parse OneDrive link")
        ));
        assert!(is_supported_link(
            &Url::parse("https://university.sharepoint.com/:u:/g/example")
                .expect("parse SharePoint link")
        ));
        assert!(!is_supported_link(
            &Url::parse("http://1drv.ms/u/s!example").expect("parse HTTP link")
        ));
        assert!(!is_supported_link(
            &Url::parse("https://example.com/audio.wav").expect("parse unrelated link")
        ));
    }

    #[test]
    fn enhanced_profiles_do_not_receive_generic_cleanup() {
        for model in [
            EnhancementModel::Resemble,
            EnhancementModel::DeepFilterNet,
            EnhancementModel::Studio,
            EnhancementModel::StudioV18,
            EnhancementModel::StudioV18Natural,
            EnhancementModel::StudioV18NaturalPlus,
            EnhancementModel::StudioV18NaturalDoublePlus,
        ] {
            assert!(!effective_cleanup_enabled(model, true));
        }
    }

    #[test]
    fn standard_processing_keeps_requested_generic_cleanup() {
        assert!(effective_cleanup_enabled(EnhancementModel::None, true));
        assert!(!effective_cleanup_enabled(EnhancementModel::None, false));
    }
}
