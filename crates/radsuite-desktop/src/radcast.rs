use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use radsuite_engines::{
    AudioOutputFormat, AudioProcessingRequest, AudioProcessor, CaptionFormat,
    CaptionProcessingRequest, CaptionProcessor, CaptionTranscriptionRequest, FillerRemovalMode,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const RADCAST_ROOT: &str = "radcast";

fn default_filler_removal_mode() -> FillerRemovalMode {
    FillerRemovalMode::Aggressive
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
    #[serde(default)]
    pub remove_filler_words: bool,
    #[serde(default = "default_filler_removal_mode")]
    pub filler_removal_mode: FillerRemovalMode,
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
    #[serde(default)]
    pub caption_segment_count: usize,
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
}

#[derive(Debug, Error)]
pub enum RadcastStorageError {
    #[error("choose an audio file before importing it")]
    EmptyPath,
    #[error("could not determine the audio filename")]
    MissingFilename,
    #[error("audio source does not exist: {path}")]
    MissingInput { path: PathBuf },
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
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct RadcastManifest {
    sources: Vec<RadcastAudioSource>,
    outputs: Vec<RadcastAudioOutput>,
}

pub(crate) fn list_audio(
    data_dir: &Path,
    project_id: radsuite_core::ProjectId,
) -> Result<RadcastAudioListing, RadcastStorageError> {
    let manifest = load_manifest(data_dir, project_id)?;
    Ok(RadcastAudioListing {
        sources: manifest.sources,
        outputs: manifest.outputs,
    })
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
    fs::copy(&source_path, &destination)?;
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
    let mut manifest = load_manifest(data_dir, project_id)?;
    let source = manifest
        .sources
        .iter()
        .find(|source| source.id == request.source_id)
        .cloned()
        .ok_or_else(|| RadcastStorageError::MissingSource(request.source_id.clone()))?;
    let source_path = PathBuf::from(&source.path);
    if !source_path.is_file() {
        return Err(RadcastStorageError::MissingSource(source.id));
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
    let removed_filler_count = removal_intervals.len();
    let result = processor.process(AudioProcessingRequest {
        input_path: source_path,
        output_path: output_path.clone(),
        output_format: request.output_format,
        clip_start_seconds: request.clip_start_seconds,
        clip_end_seconds: request.clip_end_seconds,
        max_silence_seconds: request.max_silence_seconds,
        remove_intervals: removal_intervals,
        cleanup_enabled: request.cleanup_enabled,
    })?;

    let (caption_path, caption_format, caption_segment_count) =
        if let Some(format) = request.caption_format {
            let path = output_path.with_extension(format.extension());
            let caption_result = caption_processor.process(CaptionProcessingRequest {
                input_path: output_path.clone(),
                output_path: path.clone(),
                caption_format: format,
                language: request.caption_language.trim().to_string(),
                clip_start_seconds: None,
                clip_end_seconds: None,
            });
            match caption_result {
                Ok(caption_result) => (
                    Some(caption_result.output_path.to_string_lossy().into_owned()),
                    Some(caption_result.caption_format),
                    caption_result.segment_count,
                ),
                Err(error) => {
                    let _ = fs::remove_file(&output_path);
                    let _ = fs::remove_file(path);
                    return Err(error.into());
                }
            }
        } else {
            (None, None, 0)
        };

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
        caption_segment_count,
        remove_filler_words: request.remove_filler_words,
        filler_removal_mode: request.filler_removal_mode,
        removed_filler_count,
        created_at: Utc::now().to_rfc3339(),
    };
    manifest.outputs.insert(0, output.clone());
    if let Err(error) = write_manifest(data_dir, project_id, &manifest) {
        let _ = fs::remove_file(output_path);
        if let Some(caption_path) = output.caption_path.as_deref() {
            let _ = fs::remove_file(caption_path);
        }
        return Err(error);
    }
    Ok(output)
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
