use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use radsuite_engines::{
    AudioOutputFormat, AudioProcessingRequest, AudioProcessor, CaptionFormat,
    CaptionProcessingRequest, CaptionProcessor, CaptionQualityMode, CaptionQualitySummary,
    CaptionTranscriptionRequest, EnhancementModel, EnhancementProcessingRequest,
    EnhancementProcessor, EnhancementQuality, FillerRemovalMode, RADCAST_OPTIMIZED_POSTFILTER,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRadcastAudioRequest {
    #[serde(default)]
    pub project_id: Option<radsuite_core::ProjectId>,
    pub path: String,
    pub original_filename: Option<String>,
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
    let removal_intervals = if request.remove_filler_words {
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::RemovingFillerWords,
            percent: 12,
        });
        caption_processor.filler_intervals(
            &CaptionTranscriptionRequest {
                input_path: source_path.clone(),
                language: request.caption_language.trim().to_string(),
                clip_start_seconds: request.clip_start_seconds,
                clip_end_seconds: request.clip_end_seconds,
            },
            request.filler_removal_mode,
        )?
    } else {
        Vec::new()
    };
    if is_cancelled() {
        return Err(RadcastStorageError::Cancelled);
    }
    let removed_filler_count = removal_intervals.len();
    let mut temporary_paths = Vec::new();
    let processing_input_path = if request.enhancement_model == EnhancementModel::StudioV18 {
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::PreparingEnhancement,
            percent: 20,
        });
        let prepared_path = output_path.with_file_name(format!(".{output_id}-prepared.wav"));
        let enhanced_path = output_path.with_file_name(format!(".{output_id}-enhanced.wav"));
        if let Err(error) = processor.process(AudioProcessingRequest {
            input_path: source_path.clone(),
            output_path: prepared_path.clone(),
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: request.clip_start_seconds,
            clip_end_seconds: request.clip_end_seconds,
            max_silence_seconds: None,
            remove_intervals: Vec::new(),
            cleanup_enabled: false,
        }) {
            cleanup_temporary_paths(&[prepared_path, enhanced_path]);
            return Err(error.into());
        }
        if is_cancelled() {
            cleanup_temporary_paths(&[prepared_path, enhanced_path]);
            return Err(RadcastStorageError::Cancelled);
        }
        report_progress(RadcastProcessingProgress {
            phase: RadcastProcessingPhase::EnhancingAudio,
            percent: 35,
        });
        if let Err(error) = enhancement_processor.process_with_quality(
            EnhancementProcessingRequest {
                input_path: prepared_path.clone(),
                output_path: enhanced_path.clone(),
            },
            request.enhancement_quality,
        ) {
            cleanup_temporary_paths(&[prepared_path, enhanced_path]);
            return Err(error.into());
        }
        if is_cancelled() {
            cleanup_temporary_paths(&[prepared_path, enhanced_path]);
            return Err(RadcastStorageError::Cancelled);
        }
        temporary_paths.extend([prepared_path, enhanced_path.clone()]);
        enhanced_path
    } else {
        source_path.clone()
    };
    report_progress(RadcastProcessingProgress {
        phase: RadcastProcessingPhase::RenderingAudio,
        percent: if request.enhancement_model == EnhancementModel::StudioV18 {
            78
        } else {
            35
        },
    });
    let clip_start_seconds = (request.enhancement_model == EnhancementModel::None)
        .then_some(request.clip_start_seconds)
        .flatten();
    let clip_end_seconds = (request.enhancement_model == EnhancementModel::None)
        .then_some(request.clip_end_seconds)
        .flatten();
    let additional_filter = (request.enhancement_model == EnhancementModel::StudioV18)
        .then_some(RADCAST_OPTIMIZED_POSTFILTER);
    let result = match processor.process_with_additional_filter(
        AudioProcessingRequest {
            input_path: processing_input_path,
            output_path: output_path.clone(),
            output_format: request.output_format,
            clip_start_seconds,
            clip_end_seconds,
            max_silence_seconds: request.max_silence_seconds,
            remove_intervals: removal_intervals,
            cleanup_enabled: request.cleanup_enabled,
        },
        additional_filter,
    ) {
        Ok(result) => result,
        Err(error) => {
            cleanup_temporary_paths(&temporary_paths);
            return Err(error.into());
        }
    };
    if is_cancelled() {
        let _ = fs::remove_file(&output_path);
        cleanup_temporary_paths(&temporary_paths);
        return Err(RadcastStorageError::Cancelled);
    }
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
        cleanup_enabled: request.cleanup_enabled,
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
        created_at: Utc::now().to_rfc3339(),
    };
    manifest.outputs.insert(0, output.clone());
    manifest.settings = project_settings;
    if let Err(error) = write_manifest(data_dir, project_id, &manifest) {
        let _ = fs::remove_file(output_path);
        cleanup_temporary_paths(&temporary_paths);
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

#[cfg(test)]
mod tests {
    use super::is_cloud_storage_path;
    use std::path::Path;

    #[test]
    fn identifies_macos_cloud_storage_paths() {
        assert!(is_cloud_storage_path(Path::new(
            "/Users/example/Library/CloudStorage/OneDrive-Team/audio.wav"
        )));
        assert!(!is_cloud_storage_path(Path::new(
            "/Users/example/Documents/audio.wav"
        )));
    }
}
