use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use radsuite_core::ProjectId;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};
use uuid::Uuid;

use crate::{
    DesktopState, MediaOutputFormat,
    media_assets::ProjectMediaStore,
    process_group::{ManagedChild, ManagedProcessGroup},
    radt_ts::{
        RadtTsCapabilityStatus, RadtTsOutputFormat, contained_file, discover_radt_ts_cli,
        ensure_project_root, validate_output_name,
    },
};

pub type RadtTsMediaChildHandle = Arc<Mutex<Option<ManagedChild>>>;

const MAX_CAPTURED_OUTPUT_BYTES: usize = 512 * 1024;
const MEDIA_JOB_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const TERMINATION_GRACE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsMediaJobKind {
    Transcription,
    Clip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsMediaProcessingPhase {
    Preparing,
    Transcribing,
    ExtractingClip,
    RenderingVideo,
    SavingOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadtTsMediaArtifact {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadtTsMediaOutput {
    pub id: String,
    pub kind: RadtTsMediaJobKind,
    pub name: String,
    pub primary_path: String,
    pub artifacts: Vec<RadtTsMediaArtifact>,
    pub output_format: Option<RadtTsOutputFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_format: Option<MediaOutputFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    pub warnings: Vec<String>,
}

impl RadtTsMediaOutput {
    pub fn normalized_media_format(&self) -> MediaOutputFormat {
        MediaOutputFormat::from_request(self.media_format, self.output_format)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsMediaJobState {
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadtTsMediaJobStatus {
    pub id: String,
    pub project_id: ProjectId,
    pub kind: RadtTsMediaJobKind,
    pub state: RadtTsMediaJobState,
    pub phase: RadtTsMediaProcessingPhase,
    pub percent: Option<u8>,
    pub output: Option<RadtTsMediaOutput>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadtTsMediaOutputListing {
    pub outputs: Vec<RadtTsMediaOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StartRadtTsTranscriptionRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub audio_path: String,
    pub name: String,
    pub model: String,
    pub language: Option<String>,
    pub beam_size: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartRadtTsClipRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub audio_path: String,
    pub segments_json_path: String,
    pub output_name: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub start_phrase: Option<String>,
    pub end_phrase: Option<String>,
    pub verification_mode: RadtTsVerificationMode,
    #[serde(default = "default_output_format")]
    pub output_format: RadtTsOutputFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_format: Option<MediaOutputFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presenter_image_path: Option<String>,
    #[serde(default)]
    pub save_presenter_image_as_project_default: bool,
}

fn default_output_format() -> RadtTsOutputFormat {
    RadtTsOutputFormat::Mp3
}

impl StartRadtTsClipRequest {
    pub fn normalized_media_format(&self) -> MediaOutputFormat {
        MediaOutputFormat::from_request(self.media_format, Some(self.output_format))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsVerificationMode {
    Strict,
    Lenient,
}

#[derive(Debug, Error)]
pub enum RadtTsMediaError {
    #[error("RADTTS is not available: {0}")]
    MissingCli(String),
    #[error("audio input is invalid: {0}")]
    InvalidAudio(String),
    #[error("transcript segments file is invalid: {0}")]
    InvalidSegments(String),
    #[error("transcript name cannot be empty")]
    EmptyName,
    #[error("invalid output name: {0}")]
    InvalidOutputName(String),
    #[error("invalid transcription model: {0}")]
    InvalidModel(String),
    #[error("invalid transcription language: {0}")]
    InvalidLanguage(String),
    #[error("beam size must be between 1 and 10")]
    InvalidBeamSize,
    #[error("clip start requires a time or phrase")]
    MissingStartBoundary,
    #[error("clip end requires a time or phrase")]
    MissingEndBoundary,
    #[error("clip end must be greater than clip start")]
    InvalidClipBounds,
    #[error("RADTTS media processing is already running for this project")]
    JobAlreadyRunning,
    #[error("RADTTS media job {0} was not found")]
    MissingJob(String),
    #[error("RADTTS media processing was cancelled")]
    Cancelled,
    #[error("RADTTS media processing timed out")]
    TimedOut,
    #[error("RADTTS exited unsuccessfully: {0}")]
    CliFailed(String),
    #[error("RADTTS returned invalid JSON: {0}")]
    InvalidCliResult(String),
    #[error("RADTTS output is missing or outside the project: {0}")]
    InvalidOutput(String),
    #[error("RADTTS local storage error: {0}")]
    Io(#[from] std::io::Error),
    #[error("presenter image error: {0}")]
    MediaAsset(#[from] crate::media_assets::MediaAssetError),
    #[error("could not inspect clip audio: {0}")]
    AudioProbe(#[from] radsuite_engines::AudioProcessingError),
    #[error("could not render clip video: {0}")]
    VideoExport(#[from] radsuite_engines::VideoExportError),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct TranscriptionCliResult {
    segments_json_path: String,
    txt_path: String,
    srt_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ClipCliResult {
    clip_path: String,
    report_path: String,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ClipBoundaryReport {
    #[serde(default)]
    warnings: Vec<String>,
    #[serde(default)]
    media_format: Option<MediaOutputFormat>,
}

#[derive(Debug, Clone, Copy)]
enum MediaCommandKind {
    Transcription,
    Clip,
}

struct MediaJobContext {
    project_id: ProjectId,
    kind: MediaCommandKind,
    media_format: Option<MediaOutputFormat>,
    project_root: PathBuf,
    data_dir: PathBuf,
    clip_request: Option<StartRadtTsClipRequest>,
    child_handle: RadtTsMediaChildHandle,
    jobs: Arc<Mutex<std::collections::HashMap<String, RadtTsMediaJobStatus>>>,
    children: Arc<Mutex<std::collections::HashMap<String, RadtTsMediaChildHandle>>>,
    cancellations: Arc<Mutex<std::collections::HashSet<String>>>,
    active_projects: Arc<Mutex<std::collections::HashSet<String>>>,
}

struct MediaProcessRequest {
    project_id: ProjectId,
    kind: MediaCommandKind,
    media_format: Option<MediaOutputFormat>,
    project_root: PathBuf,
    executable: PathBuf,
    args: Vec<String>,
    clip_request: Option<StartRadtTsClipRequest>,
}

struct ClipVideoInput<'a> {
    data_dir: &'a Path,
    project_root: &'a Path,
    project_id: ProjectId,
    request: &'a StartRadtTsClipRequest,
    audio: RadtTsMediaOutput,
}

struct ClipVideoTools {
    audio_processor: radsuite_engines::AudioProcessor,
    video_exporter: radsuite_engines::VideoExporter,
}

pub fn get_radt_ts_media_capabilities() -> RadtTsCapabilityStatus {
    discover_radt_ts_cli()
}

pub async fn start_radt_ts_transcription(
    state: &DesktopState,
    request: StartRadtTsTranscriptionRequest,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    let project_id = request.project_id.unwrap_or_default();
    let executable = media_executable().await?;
    let projects_root = state.paths.data_dir.join("radt-ts-projects");
    let project_root = ensure_project_root(&projects_root, project_id)
        .map_err(|error| RadtTsMediaError::Io(std::io::Error::other(error.to_string())))?;
    let audio_path = validate_audio_input(Path::new(&request.audio_path))?;
    validate_transcription_name(&request.name)?;
    validate_model(&request.model)?;
    validate_language(request.language.as_deref())?;
    if !(1..=10).contains(&request.beam_size) {
        return Err(RadtTsMediaError::InvalidBeamSize);
    }
    let args = build_transcription_args(&request, project_id, projects_root, audio_path)?;
    start_media_process(
        state,
        MediaProcessRequest {
            project_id,
            kind: MediaCommandKind::Transcription,
            media_format: None,
            project_root,
            executable,
            args,
            clip_request: None,
        },
    )
    .await
}

pub async fn start_radt_ts_clip(
    state: &DesktopState,
    request: StartRadtTsClipRequest,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    let project_id = request.project_id.unwrap_or_default();
    let executable = media_executable().await?;
    let projects_root = state.paths.data_dir.join("radt-ts-projects");
    let project_root = ensure_project_root(&projects_root, project_id)
        .map_err(|error| RadtTsMediaError::Io(std::io::Error::other(error.to_string())))?;
    let audio_path = validate_audio_input(Path::new(&request.audio_path))?;
    let segments_path = validate_segments_input(Path::new(&request.segments_json_path))?;
    validate_clip_request(&request)?;
    let media_format = request.normalized_media_format();
    let args = build_clip_args(
        &request,
        project_id,
        projects_root,
        audio_path,
        segments_path,
        media_format.into(),
    )?;
    start_media_process(
        state,
        MediaProcessRequest {
            project_id,
            kind: MediaCommandKind::Clip,
            media_format: Some(media_format),
            project_root,
            executable,
            args,
            clip_request: Some(request),
        },
    )
    .await
}

pub fn get_radt_ts_media_job(
    state: &DesktopState,
    job_id: &str,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    state
        .radt_ts_media_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(job_id)
        .cloned()
        .ok_or_else(|| RadtTsMediaError::MissingJob(job_id.to_string()))
}

pub fn cancel_radt_ts_media_job(
    state: &DesktopState,
    job_id: &str,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    let current = get_radt_ts_media_job(state, job_id)?;
    if matches!(
        current.state,
        RadtTsMediaJobState::Completed
            | RadtTsMediaJobState::Failed
            | RadtTsMediaJobState::Cancelled
    ) {
        return Ok(current);
    }
    state
        .radt_ts_media_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.to_string());
    if let Some(handle) = state
        .radt_ts_media_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(job_id)
        .cloned()
    {
        request_process_termination(&handle, false);
    }
    let mut jobs = state
        .radt_ts_media_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let updated = jobs
        .get_mut(job_id)
        .ok_or_else(|| RadtTsMediaError::MissingJob(job_id.to_string()))?;
    updated.state = RadtTsMediaJobState::Cancelled;
    updated.error = Some("RADTTS media processing cancelled.".to_string());
    Ok(updated.clone())
}

pub fn list_radt_ts_media_outputs(
    state: &DesktopState,
    project_id: ProjectId,
) -> Result<RadtTsMediaOutputListing, RadtTsMediaError> {
    let root = ensure_project_root(&state.paths.data_dir.join("radt-ts-projects"), project_id)
        .map_err(|error| RadtTsMediaError::Io(std::io::Error::other(error.to_string())))?;
    let mut outputs = list_transcripts(&root)?;
    let persisted_clips = read_persisted_clip_outputs(&root)?;
    let persisted_names = persisted_clips
        .iter()
        .map(|output| output.name.as_str())
        .collect::<std::collections::HashSet<_>>();
    outputs.extend(persisted_clips.clone());
    outputs.extend(
        list_clips(&root)?
            .into_iter()
            .filter(|output| !persisted_names.contains(output.name.as_str())),
    );
    outputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(RadtTsMediaOutputListing { outputs })
}

pub fn delete_radt_ts_clip_video(
    state: &DesktopState,
    project_id: ProjectId,
    output_id: &str,
) -> Result<(), RadtTsMediaError> {
    let root = ensure_project_root(&state.paths.data_dir.join("radt-ts-projects"), project_id)
        .map_err(|error| RadtTsMediaError::Io(std::io::Error::other(error.to_string())))?;
    delete_radt_ts_clip_video_from_root(&root, output_id)
}

fn delete_radt_ts_clip_video_from_root(
    root: &Path,
    output_id: &str,
) -> Result<(), RadtTsMediaError> {
    let manifest = root.join("manifests/clip-media-outputs.json");
    let mut outputs = read_persisted_clip_outputs(root)?;
    let index = outputs
        .iter()
        .position(|output| output.id == output_id)
        .ok_or_else(|| {
            RadtTsMediaError::InvalidOutput(format!("output {output_id} was not found"))
        })?;
    let output = outputs.remove(index);
    let mut owned = vec![contained_clip_deletion_file(
        root,
        Path::new(&output.primary_path),
    )?];
    for artifact in &output.artifacts {
        owned.push(contained_clip_deletion_file(
            root,
            Path::new(&artifact.path),
        )?);
    }
    let renamed = stage_clip_files_for_deletion(&owned, output_id)?;
    if let Err(error) = write_media_json_atomic(&manifest, &outputs) {
        restore_clip_files(&renamed);
        return Err(error);
    }
    for (_, staged) in renamed {
        if staged.exists() {
            fs::remove_file(staged)?;
        }
    }
    Ok(())
}

fn contained_clip_deletion_file(root: &Path, path: &Path) -> Result<PathBuf, RadtTsMediaError> {
    let canonical_root = root.canonicalize()?;
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|_| RadtTsMediaError::InvalidOutput(path.display().to_string()))?;
    let contained = canonical.starts_with(&canonical_root)
        || macos_private_alias(&canonical).starts_with(macos_private_alias(&canonical_root));
    if !canonical.is_file() || !contained {
        return Err(RadtTsMediaError::InvalidOutput(path.display().to_string()));
    }
    Ok(canonical)
}

fn macos_private_alias(path: &Path) -> &Path {
    path.strip_prefix("/private").unwrap_or(path)
}

fn stage_clip_files_for_deletion(
    paths: &[PathBuf],
    output_id: &str,
) -> Result<Vec<(PathBuf, PathBuf)>, RadtTsMediaError> {
    let mut renamed = Vec::new();
    for path in paths.iter().filter(|path| path.exists()) {
        let staged = path.with_extension(format!("delete-{output_id}"));
        if let Err(error) = fs::rename(path, &staged) {
            restore_clip_files(&renamed);
            return Err(error.into());
        }
        renamed.push((path.clone(), staged));
    }
    Ok(renamed)
}

fn restore_clip_files(paths: &[(PathBuf, PathBuf)]) {
    for (original, staged) in paths.iter().rev() {
        if staged.exists() {
            let _ = fs::rename(staged, original);
        }
    }
}

pub fn shutdown_radt_ts_media_jobs(state: &DesktopState) {
    let handles = state
        .radt_ts_media_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .map(|(job_id, handle)| (job_id.clone(), handle.clone()))
        .collect::<Vec<_>>();
    state
        .radt_ts_media_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .extend(handles.iter().map(|(job_id, _)| job_id.clone()));
    for (job_id, _) in &handles {
        if let Some(job) = state
            .radt_ts_media_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(job_id)
            && matches!(
                job.state,
                RadtTsMediaJobState::Starting | RadtTsMediaJobState::Running
            )
        {
            job.state = RadtTsMediaJobState::Cancelled;
            job.error = Some("RADsuite stopped media processing while closing.".to_string());
        }
    }
    for (_, handle) in handles {
        request_process_termination(&handle, true);
    }
}

pub(crate) fn build_transcription_args(
    request: &StartRadtTsTranscriptionRequest,
    project_id: ProjectId,
    projects_root: PathBuf,
    audio_path: PathBuf,
) -> Result<Vec<String>, RadtTsMediaError> {
    validate_transcription_name(&request.name)?;
    validate_model(&request.model)?;
    validate_language(request.language.as_deref())?;
    if !(1..=10).contains(&request.beam_size) {
        return Err(RadtTsMediaError::InvalidBeamSize);
    }
    let mut args = vec![
        "--projects-root".to_string(),
        projects_root.display().to_string(),
        "transcribe".to_string(),
        "--project-id".to_string(),
        project_id.to_string(),
        "--audio-path".to_string(),
        audio_path.display().to_string(),
        "--name".to_string(),
        request.name.trim().to_string(),
        "--model".to_string(),
        request.model.trim().to_string(),
        "--beam-size".to_string(),
        request.beam_size.to_string(),
    ];
    if let Some(language) = request
        .language
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        args.extend(["--language".to_string(), language.trim().to_string()]);
    }
    Ok(args)
}

pub(crate) fn build_clip_args(
    request: &StartRadtTsClipRequest,
    project_id: ProjectId,
    projects_root: PathBuf,
    audio_path: PathBuf,
    segments_path: PathBuf,
    output_format: RadtTsOutputFormat,
) -> Result<Vec<String>, RadtTsMediaError> {
    validate_clip_request(request)?;
    let mut args = vec![
        "--projects-root".to_string(),
        projects_root.display().to_string(),
        "clip".to_string(),
        "--project-id".to_string(),
        project_id.to_string(),
        "--audio-path".to_string(),
        audio_path.display().to_string(),
        "--segments-json".to_string(),
        segments_path.display().to_string(),
        "--output-name".to_string(),
        request.output_name.trim().to_string(),
    ];
    if let Some(value) = request.start_time {
        args.extend(["--start-time".to_string(), value.to_string()]);
    } else if let Some(value) = request.start_phrase.as_deref() {
        args.extend(["--start-phrase".to_string(), value.trim().to_string()]);
    }
    if let Some(value) = request.end_time {
        args.extend(["--end-time".to_string(), value.to_string()]);
    } else if let Some(value) = request.end_phrase.as_deref() {
        args.extend(["--end-phrase".to_string(), value.trim().to_string()]);
    }
    args.extend([
        "--verification-mode".to_string(),
        match request.verification_mode {
            RadtTsVerificationMode::Strict => "strict".to_string(),
            RadtTsVerificationMode::Lenient => "lenient".to_string(),
        },
        "--output-format".to_string(),
        output_format.as_cli_value().to_string(),
    ]);
    Ok(args)
}

fn parse_transcription_result(stdout: &[u8]) -> Result<TranscriptionCliResult, RadtTsMediaError> {
    serde_json::from_slice(stdout)
        .map_err(|error| RadtTsMediaError::InvalidCliResult(error.to_string()))
}

fn parse_clip_result(stdout: &[u8]) -> Result<ClipCliResult, RadtTsMediaError> {
    serde_json::from_slice(stdout)
        .map_err(|error| RadtTsMediaError::InvalidCliResult(error.to_string()))
}

async fn start_media_process(
    state: &DesktopState,
    request: MediaProcessRequest,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    let MediaProcessRequest {
        project_id,
        kind,
        media_format,
        project_root,
        executable,
        args,
        clip_request,
    } = request;
    {
        let mut active = state
            .radt_ts_active_projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !active.insert(project_id.to_string()) {
            return Err(RadtTsMediaError::JobAlreadyRunning);
        }
    }
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let process_group = ManagedProcessGroup::attach(&mut command).map_err(RadtTsMediaError::Io)?;
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            remove_active_project(state, project_id);
            return Err(RadtTsMediaError::Io(error));
        }
    };
    if let Err(error) = process_group.register_child(&child) {
        let _ = child.start_kill();
        remove_active_project(state, project_id);
        return Err(RadtTsMediaError::Io(error));
    }
    let child_handle = Arc::new(Mutex::new(Some((child, process_group))));
    let job_id = Uuid::new_v4().to_string();
    let initial = RadtTsMediaJobStatus {
        id: job_id.clone(),
        project_id,
        kind: match kind {
            MediaCommandKind::Transcription => RadtTsMediaJobKind::Transcription,
            MediaCommandKind::Clip => RadtTsMediaJobKind::Clip,
        },
        state: RadtTsMediaJobState::Starting,
        phase: RadtTsMediaProcessingPhase::Preparing,
        percent: None,
        output: None,
        error: None,
    };
    state
        .radt_ts_media_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.clone(), initial.clone());
    state
        .radt_ts_media_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.clone(), child_handle.clone());
    state
        .radt_ts_media_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&job_id);
    let context = MediaJobContext {
        project_id,
        kind,
        media_format,
        project_root,
        data_dir: state.paths.data_dir.clone(),
        clip_request,
        child_handle,
        jobs: state.radt_ts_media_jobs.clone(),
        children: state.radt_ts_media_children.clone(),
        cancellations: state.radt_ts_media_cancel_requests.clone(),
        active_projects: state.radt_ts_active_projects.clone(),
    };
    tokio::spawn(async move { run_media_job(job_id, context).await });
    Ok(initial)
}

async fn run_media_job(job_id: String, context: MediaJobContext) {
    let MediaJobContext {
        project_id,
        kind,
        media_format,
        project_root,
        data_dir,
        clip_request,
        child_handle,
        jobs,
        children,
        cancellations,
        active_projects,
    } = context;
    update_job(&jobs, &job_id, |job| {
        job.state = RadtTsMediaJobState::Running;
        job.phase = match kind {
            MediaCommandKind::Transcription => RadtTsMediaProcessingPhase::Transcribing,
            MediaCommandKind::Clip => RadtTsMediaProcessingPhase::ExtractingClip,
        };
    });
    let (stdout, stderr) = {
        let mut guard = child_handle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some((child, _)) = guard.as_mut() else {
            finish_media_job(
                &jobs,
                &children,
                &cancellations,
                &active_projects,
                &job_id,
                Err(RadtTsMediaError::CliFailed(
                    "process handle disappeared".to_string(),
                )),
            );
            return;
        };
        (child.stdout.take(), child.stderr.take())
    };
    let stdout_task = tokio::spawn(read_limited(stdout));
    let stderr_task = tokio::spawn(read_limited(stderr));
    let started = Instant::now();
    let mut termination_deadline = None;
    let mut termination_reason = None;
    loop {
        let cancelled = cancellations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&job_id);
        if termination_deadline.is_none() && (cancelled || started.elapsed() >= MEDIA_JOB_TIMEOUT) {
            termination_reason = Some(if cancelled {
                RadtTsMediaError::Cancelled
            } else {
                RadtTsMediaError::TimedOut
            });
            request_process_termination(&child_handle, false);
            termination_deadline = Some(Instant::now() + TERMINATION_GRACE);
        } else if termination_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            request_process_termination(&child_handle, true);
        }
        let exited = {
            let mut guard = child_handle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            guard
                .as_mut()
                .and_then(|child| child.0.try_wait().transpose())
                .transpose()
        };
        match exited {
            Ok(Some(status)) => {
                let stdout = stdout_task.await.unwrap_or_default();
                let stderr = stderr_task.await.unwrap_or_default();
                let result = if let Some(reason) = termination_reason {
                    Err(reason)
                } else if !status.success() {
                    let detail = String::from_utf8_lossy(&stderr).trim().to_string();
                    Err(RadtTsMediaError::CliFailed(if detail.is_empty() {
                        format!("exit status {status}")
                    } else {
                        detail
                    }))
                } else {
                    let parsed = parse_media_output(
                        kind,
                        &stdout,
                        &project_root,
                        project_id,
                        &job_id,
                        if media_format == Some(MediaOutputFormat::Mp4) {
                            Some(MediaOutputFormat::Wav)
                        } else {
                            media_format
                        },
                    );
                    match (parsed, clip_request.as_ref()) {
                        (Ok(output), Some(request))
                            if request.normalized_media_format() == MediaOutputFormat::Mp4 =>
                        {
                            update_job(&jobs, &job_id, |job| {
                                job.phase = RadtTsMediaProcessingPhase::RenderingVideo;
                                job.percent = Some(90);
                            });
                            finalize_clip_video(
                                ClipVideoInput {
                                    data_dir: &data_dir,
                                    project_root: &project_root,
                                    project_id,
                                    request,
                                    audio: output,
                                },
                                ClipVideoTools {
                                    audio_processor: radsuite_engines::AudioProcessor::default(),
                                    video_exporter: radsuite_engines::VideoExporter::default(),
                                },
                                || {
                                    cancellations
                                        .lock()
                                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                                        .contains(&job_id)
                                },
                                |progress| {
                                    update_job(&jobs, &job_id, |job| {
                                        job.percent =
                                            Some((90.0 + progress.clamp(0.0, 1.0) * 9.0).round()
                                                as u8);
                                    });
                                },
                            )
                        }
                        (result, _) => result,
                    }
                };
                finish_media_job(
                    &jobs,
                    &children,
                    &cancellations,
                    &active_projects,
                    &job_id,
                    result,
                );
                break;
            }
            Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(error) => {
                finish_media_job(
                    &jobs,
                    &children,
                    &cancellations,
                    &active_projects,
                    &job_id,
                    Err(RadtTsMediaError::Io(error)),
                );
                break;
            }
        }
    }
}

fn parse_media_output(
    kind: MediaCommandKind,
    stdout: &[u8],
    project_root: &Path,
    _project_id: ProjectId,
    job_id: &str,
    requested_media_format: Option<MediaOutputFormat>,
) -> Result<RadtTsMediaOutput, RadtTsMediaError> {
    match kind {
        MediaCommandKind::Transcription => {
            let result = parse_transcription_result(stdout)?;
            let txt_path = contained_media_file(project_root, Path::new(&result.txt_path))?;
            let srt_path = contained_media_file(project_root, Path::new(&result.srt_path))?;
            let segments_path =
                contained_media_file(project_root, Path::new(&result.segments_json_path))?;
            let name = txt_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| RadtTsMediaError::InvalidOutput(result.txt_path.clone()))?
                .to_string();
            Ok(RadtTsMediaOutput {
                id: job_id.to_string(),
                kind: RadtTsMediaJobKind::Transcription,
                name,
                primary_path: txt_path.display().to_string(),
                artifacts: vec![
                    RadtTsMediaArtifact {
                        label: "SRT captions".to_string(),
                        path: srt_path.display().to_string(),
                    },
                    RadtTsMediaArtifact {
                        label: "Timed segments".to_string(),
                        path: segments_path.display().to_string(),
                    },
                ],
                output_format: None,
                media_format: None,
                image_path: None,
                warnings: Vec::new(),
            })
        }
        MediaCommandKind::Clip => {
            let result = parse_clip_result(stdout)?;
            let clip_path = contained_media_file(project_root, Path::new(&result.clip_path))?;
            let report_path = contained_media_file(project_root, Path::new(&result.report_path))?;
            let mut warnings = result.warnings;
            let report =
                serde_json::from_slice::<ClipBoundaryReport>(&fs::read(&report_path)?).ok();
            let report_media_format = report.as_ref().and_then(|report| report.media_format);
            if let Some(report) = report {
                warnings.extend(report.warnings);
            }
            warnings.sort();
            warnings.dedup();
            let name = clip_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| RadtTsMediaError::InvalidOutput(result.clip_path.clone()))?
                .to_string();
            let output_format = clip_path
                .extension()
                .and_then(|value| value.to_str())
                .and_then(|value| match value {
                    "wav" => Some(RadtTsOutputFormat::Wav),
                    "mp3" => Some(RadtTsOutputFormat::Mp3),
                    _ => None,
                });
            Ok(RadtTsMediaOutput {
                id: job_id.to_string(),
                kind: RadtTsMediaJobKind::Clip,
                name,
                primary_path: clip_path.display().to_string(),
                artifacts: vec![RadtTsMediaArtifact {
                    label: "Boundary report".to_string(),
                    path: report_path.display().to_string(),
                }],
                media_format: Some(MediaOutputFormat::from_request(
                    requested_media_format.or(report_media_format),
                    output_format,
                )),
                image_path: None,
                output_format,
                warnings,
            })
        }
    }
}

fn finalize_clip_video<C, P>(
    input: ClipVideoInput<'_>,
    tools: ClipVideoTools,
    mut is_cancelled: C,
    mut on_progress: P,
) -> Result<RadtTsMediaOutput, RadtTsMediaError>
where
    C: FnMut() -> bool,
    P: FnMut(f64),
{
    let ClipVideoInput {
        data_dir,
        project_root,
        project_id,
        request,
        audio,
    } = input;
    let ClipVideoTools {
        audio_processor,
        video_exporter,
    } = tools;
    let audio_path = PathBuf::from(&audio.primary_path);
    let duration = audio_processor.probe_duration(&audio_path)?;
    let store = ProjectMediaStore::new(data_dir);
    let selected_image = request
        .presenter_image_path
        .clone()
        .map(PathBuf::from)
        .or_else(|| {
            store
                .saved_cover(&project_id.to_string())
                .ok()
                .flatten()
                .map(|image| image.path().to_path_buf())
        })
        .ok_or_else(|| {
            RadtTsMediaError::InvalidOutput("MP4 output requires a presenter image".to_string())
        })?;
    let staged = store.stage_image(&project_id.to_string(), &selected_image)?;
    let video_path = audio_path.with_extension("mp4");
    let rendered = match video_exporter.export_with_callbacks(
        radsuite_engines::VideoExportRequest::new(
            staged.path(),
            &audio_path,
            &video_path,
            duration,
        ),
        &mut is_cancelled,
        &mut on_progress,
    ) {
        Ok(rendered) => rendered,
        Err(error) => {
            let _ = fs::remove_file(staged.path());
            let _ = fs::remove_file(&video_path);
            return Err(error.into());
        }
    };
    let output_id = Uuid::parse_str(&audio.id).unwrap_or_else(|_| Uuid::new_v4());
    let pending = match store.prepare_commit(
        staged,
        output_id,
        request.save_presenter_image_as_project_default,
    ) {
        Ok(pending) => pending,
        Err(error) => {
            let _ = fs::remove_file(&video_path);
            return Err(error.into());
        }
    };
    let mut output = audio.clone();
    output.primary_path = rendered.output_path.to_string_lossy().into_owned();
    output.output_format = Some(RadtTsOutputFormat::Wav);
    output.media_format = Some(MediaOutputFormat::Mp4);
    output.image_path = Some(pending.managed().path().to_string_lossy().into_owned());

    let manifest_path = project_root.join("manifests/clip-media-outputs.json");
    let mut outputs = read_persisted_clip_outputs(project_root)?;
    outputs.retain(|item| item.id != output.id && item.name != output.name);
    outputs.insert(0, output.clone());
    if let Err(error) = write_media_json_atomic(&manifest_path, &outputs) {
        let _ = store.rollback_commit(pending);
        let _ = fs::remove_file(&video_path);
        return Err(error);
    }
    store.finalize_commit(pending)?;
    let _ = fs::remove_file(audio_path);
    Ok(output)
}

fn read_persisted_clip_outputs(
    project_root: &Path,
) -> Result<Vec<RadtTsMediaOutput>, RadtTsMediaError> {
    let path = project_root.join("manifests/clip-media-outputs.json");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut outputs: Vec<RadtTsMediaOutput> = serde_json::from_slice(&fs::read(&path)?)
        .map_err(|error| RadtTsMediaError::InvalidCliResult(error.to_string()))?;
    outputs.retain(|output| Path::new(&output.primary_path).is_file());
    Ok(outputs)
}

fn write_media_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), RadtTsMediaError> {
    let temporary = path.with_extension(format!("partial-{}", Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(value)
            .map_err(|error| RadtTsMediaError::InvalidCliResult(error.to_string()))?,
    )?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn finish_media_job(
    jobs: &Arc<Mutex<std::collections::HashMap<String, RadtTsMediaJobStatus>>>,
    children: &Arc<Mutex<std::collections::HashMap<String, RadtTsMediaChildHandle>>>,
    cancellations: &Arc<Mutex<std::collections::HashSet<String>>>,
    active_projects: &Arc<Mutex<std::collections::HashSet<String>>>,
    job_id: &str,
    result: Result<RadtTsMediaOutput, RadtTsMediaError>,
) {
    let project_id = if let Some(job) = jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(job_id)
    {
        let project_id = job.project_id.to_string();
        match result {
            Ok(output) => {
                job.state = RadtTsMediaJobState::Completed;
                job.phase = RadtTsMediaProcessingPhase::SavingOutput;
                job.percent = Some(100);
                job.output = Some(output);
                job.error = None;
            }
            Err(error) => {
                job.state = if matches!(&error, RadtTsMediaError::Cancelled) {
                    RadtTsMediaJobState::Cancelled
                } else {
                    RadtTsMediaJobState::Failed
                };
                job.error = Some(error.to_string());
            }
        }
        Some(project_id)
    } else {
        None
    };
    children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(job_id);
    cancellations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(job_id);
    if let Some(project_id) = project_id {
        active_projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&project_id);
    }
}

fn update_job<F>(
    jobs: &Arc<Mutex<std::collections::HashMap<String, RadtTsMediaJobStatus>>>,
    job_id: &str,
    update: F,
) where
    F: FnOnce(&mut RadtTsMediaJobStatus),
{
    if let Some(job) = jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(job_id)
    {
        update(job);
    }
}

async fn media_executable() -> Result<PathBuf, RadtTsMediaError> {
    let capability = crate::radt_ts::discover_radt_ts_cli_async().await;
    capability
        .executable
        .map(PathBuf::from)
        .ok_or(RadtTsMediaError::MissingCli(capability.detail))
}

fn validate_transcription_name(value: &str) -> Result<(), RadtTsMediaError> {
    if value.trim().is_empty() {
        return Err(RadtTsMediaError::EmptyName);
    }
    validate_output_name(value.trim())
        .map_err(|error| RadtTsMediaError::InvalidOutputName(error.to_string()))
}

fn validate_model(value: &str) -> Result<(), RadtTsMediaError> {
    if value.is_empty()
        || value.len() > 32
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(RadtTsMediaError::InvalidModel(value.to_string()));
    }
    Ok(())
}

fn validate_language(value: Option<&str>) -> Result<(), RadtTsMediaError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if value.len() > 16
        || !value
            .chars()
            .all(|character| character.is_ascii_alphabetic() || character == '-')
    {
        return Err(RadtTsMediaError::InvalidLanguage(value.to_string()));
    }
    Ok(())
}

fn validate_clip_request(request: &StartRadtTsClipRequest) -> Result<(), RadtTsMediaError> {
    validate_output_name(request.output_name.trim())
        .map_err(|error| RadtTsMediaError::InvalidOutputName(error.to_string()))?;
    let has_start_time = request.start_time.is_some();
    let has_start_phrase = request
        .start_phrase
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_end_time = request.end_time.is_some();
    let has_end_phrase = request
        .end_phrase
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    if !has_start_time && !has_start_phrase {
        return Err(RadtTsMediaError::MissingStartBoundary);
    }
    if !has_end_time && !has_end_phrase {
        return Err(RadtTsMediaError::MissingEndBoundary);
    }
    if request
        .start_time
        .is_some_and(|value| !value.is_finite() || value < 0.0)
        || request
            .end_time
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err(RadtTsMediaError::InvalidClipBounds);
    }
    if has_start_time
        && has_end_time
        && request.end_time.unwrap_or_default() <= request.start_time.unwrap_or_default()
    {
        return Err(RadtTsMediaError::InvalidClipBounds);
    }
    Ok(())
}

fn validate_audio_input(path: &Path) -> Result<PathBuf, RadtTsMediaError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| RadtTsMediaError::InvalidAudio(format!("{} ({error})", path.display())))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RadtTsMediaError::InvalidAudio(
            "choose a regular audio file, not a folder or symlink".to_string(),
        ));
    }
    let supported = matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("wav" | "mp3" | "m4a" | "flac" | "ogg" | "webm" | "aac")
    );
    if !supported {
        return Err(RadtTsMediaError::InvalidAudio(
            "use WAV, MP3, M4A, FLAC, OGG, WEBM, or AAC audio".to_string(),
        ));
    }
    path.canonicalize()
        .map_err(|error| RadtTsMediaError::InvalidAudio(format!("{} ({error})", path.display())))
}

fn validate_segments_input(path: &Path) -> Result<PathBuf, RadtTsMediaError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        RadtTsMediaError::InvalidSegments(format!("{} ({error})", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RadtTsMediaError::InvalidSegments(
            "choose a regular segments JSON file".to_string(),
        ));
    }
    if path.extension().and_then(|value| value.to_str()) != Some("json") {
        return Err(RadtTsMediaError::InvalidSegments(
            "choose a .json transcript segments file".to_string(),
        ));
    }
    path.canonicalize()
        .map_err(|error| RadtTsMediaError::InvalidSegments(format!("{} ({error})", path.display())))
}

fn list_transcripts(root: &Path) -> Result<Vec<RadtTsMediaOutput>, RadtTsMediaError> {
    let directory = root.join("transcripts");
    let mut outputs = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(name) = file_name.strip_suffix(".segments.json") else {
            continue;
        };
        let segments = contained_media_file(root, &path)?;
        let txt =
            contained_media_file(root, &root.join("transcripts").join(format!("{name}.txt")))?;
        let srt =
            contained_media_file(root, &root.join("transcripts").join(format!("{name}.srt")))?;
        outputs.push(RadtTsMediaOutput {
            id: format!("transcription:{name}"),
            kind: RadtTsMediaJobKind::Transcription,
            name: name.to_string(),
            primary_path: txt.display().to_string(),
            artifacts: vec![
                RadtTsMediaArtifact {
                    label: "SRT captions".to_string(),
                    path: srt.display().to_string(),
                },
                RadtTsMediaArtifact {
                    label: "Timed segments".to_string(),
                    path: segments.display().to_string(),
                },
            ],
            output_format: None,
            media_format: None,
            image_path: None,
            warnings: Vec::new(),
        });
    }
    Ok(outputs)
}

fn list_clips(root: &Path) -> Result<Vec<RadtTsMediaOutput>, RadtTsMediaError> {
    let directory = root.join("manifests");
    let mut outputs = Vec::new();
    for entry in fs::read_dir(directory)? {
        let report_path = entry?.path();
        let Some(file_name) = report_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(name) = file_name.strip_suffix(".clip.boundary.json") else {
            continue;
        };
        let report = serde_json::from_slice::<ClipBoundaryReport>(&fs::read(&report_path)?)
            .map_err(|error| RadtTsMediaError::InvalidCliResult(error.to_string()))?;
        let clip_path = ["mp3", "wav"]
            .iter()
            .map(|extension| {
                root.join("assets/source_audio")
                    .join(format!("{name}.{extension}"))
            })
            .find(|path| path.is_file());
        let Some(clip_path) = clip_path else { continue };
        let clip_path = contained_media_file(root, &clip_path)?;
        let report_path = contained_media_file(root, &report_path)?;
        let output_format = clip_path
            .extension()
            .and_then(|value| value.to_str())
            .and_then(|value| match value {
                "wav" => Some(RadtTsOutputFormat::Wav),
                "mp3" => Some(RadtTsOutputFormat::Mp3),
                _ => None,
            });
        outputs.push(RadtTsMediaOutput {
            id: format!("clip:{name}"),
            kind: RadtTsMediaJobKind::Clip,
            name: name.to_string(),
            primary_path: clip_path.display().to_string(),
            artifacts: vec![RadtTsMediaArtifact {
                label: "Boundary report".to_string(),
                path: report_path.display().to_string(),
            }],
            output_format,
            media_format: Some(MediaOutputFormat::from_request(
                report.media_format,
                output_format,
            )),
            image_path: None,
            warnings: report.warnings,
        });
    }
    Ok(outputs)
}

fn contained_media_file(root: &Path, path: &Path) -> Result<PathBuf, RadtTsMediaError> {
    contained_file(root, path).map_err(|error| RadtTsMediaError::InvalidOutput(error.to_string()))
}

async fn read_limited<R>(reader: Option<R>) -> Vec<u8>
where
    R: AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Vec::new();
    };
    let mut output = Vec::new();
    let mut buffer = [0_u8; 8192];
    while let Ok(read) = reader.read(&mut buffer).await {
        if read == 0 {
            break;
        }
        if output.len() < MAX_CAPTURED_OUTPUT_BYTES {
            let remaining = MAX_CAPTURED_OUTPUT_BYTES - output.len();
            output.extend_from_slice(&buffer[..read.min(remaining)]);
        }
    }
    output
}

fn request_process_termination(handle: &RadtTsMediaChildHandle, force: bool) {
    let mut guard = handle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some((child, process_group)) = guard.as_mut() else {
        return;
    };
    let _ = process_group.terminate(child, force);
    if force {
        let _ = child.start_kill();
    }
}

fn remove_active_project(state: &DesktopState, project_id: ProjectId) {
    state
        .radt_ts_active_projects
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&project_id.to_string());
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use radsuite_core::ProjectId;

    use super::{
        ClipVideoInput, ClipVideoTools, MediaCommandKind, MediaOutputFormat, RadtTsMediaArtifact,
        RadtTsMediaError, RadtTsMediaJobKind, RadtTsMediaOutput, RadtTsOutputFormat,
        RadtTsVerificationMode, StartRadtTsClipRequest, StartRadtTsTranscriptionRequest,
        build_clip_args, build_transcription_args, delete_radt_ts_clip_video_from_root,
        finalize_clip_video, list_clips, parse_clip_result, parse_media_output,
        parse_transcription_result, read_persisted_clip_outputs, validate_clip_request,
    };

    #[test]
    fn builds_transcription_arguments_without_shell_interpolation() {
        let request = StartRadtTsTranscriptionRequest {
            project_id: None,
            audio_path: "/tmp/lecture recording.mp3".to_string(),
            name: "lecture-1".to_string(),
            model: "small".to_string(),
            language: Some("en".to_string()),
            beam_size: 5,
        };
        let args = build_transcription_args(
            &request,
            ProjectId::new(),
            PathBuf::from("/tmp/projects"),
            PathBuf::from("/tmp/lecture recording.mp3"),
        )
        .expect("valid transcription request should build");
        assert!(args.contains(&"transcribe".to_string()));
        assert!(args.contains(&"--language".to_string()));
        assert!(args.contains(&"/tmp/lecture recording.mp3".to_string()));
    }

    #[test]
    fn builds_phrase_based_clip_arguments() {
        let request = StartRadtTsClipRequest {
            project_id: None,
            audio_path: "/tmp/lecture.mp3".to_string(),
            segments_json_path: "/tmp/lecture.segments.json".to_string(),
            output_name: "opening-clip".to_string(),
            start_time: None,
            end_time: None,
            start_phrase: Some("Welcome to the course".to_string()),
            end_phrase: Some("That is all for today".to_string()),
            verification_mode: RadtTsVerificationMode::Lenient,
            output_format: RadtTsOutputFormat::Wav,
            media_format: None,
            presenter_image_path: None,
            save_presenter_image_as_project_default: false,
        };
        let args = build_clip_args(
            &request,
            ProjectId::new(),
            PathBuf::from("/tmp/projects"),
            PathBuf::from("/tmp/lecture.mp3"),
            PathBuf::from("/tmp/lecture.segments.json"),
            RadtTsOutputFormat::Wav,
        )
        .expect("valid clip request should build");
        assert!(args.contains(&"--start-phrase".to_string()));
        assert!(args.contains(&"Welcome to the course".to_string()));
        assert!(args.contains(&"lenient".to_string()));
        assert!(args.contains(&"wav".to_string()));
    }

    #[test]
    fn maps_mp4_clip_requests_to_wav_for_the_audio_only_cli() {
        let request = StartRadtTsClipRequest {
            project_id: None,
            audio_path: "/tmp/lecture.mp3".to_string(),
            segments_json_path: "/tmp/lecture.segments.json".to_string(),
            output_name: "video-clip".to_string(),
            start_time: Some(0.0),
            end_time: Some(1.0),
            start_phrase: None,
            end_phrase: None,
            verification_mode: RadtTsVerificationMode::Strict,
            output_format: RadtTsOutputFormat::Mp3,
            media_format: Some(MediaOutputFormat::Mp4),
            presenter_image_path: None,
            save_presenter_image_as_project_default: false,
        };
        let args = build_clip_args(
            &request,
            ProjectId::new(),
            PathBuf::from("/tmp/projects"),
            PathBuf::from("/tmp/lecture.mp3"),
            PathBuf::from("/tmp/lecture.segments.json"),
            RadtTsOutputFormat::Wav,
        )
        .expect("valid MP4 clip request should build");
        assert!(args.contains(&"wav".to_string()));
        assert!(!args.contains(&"mp4".to_string()));
    }

    #[test]
    fn rejects_clip_without_boundaries() {
        let request = StartRadtTsClipRequest {
            project_id: None,
            audio_path: "/tmp/lecture.mp3".to_string(),
            segments_json_path: "/tmp/lecture.segments.json".to_string(),
            output_name: "clip".to_string(),
            start_time: None,
            end_time: Some(4.0),
            start_phrase: None,
            end_phrase: None,
            verification_mode: RadtTsVerificationMode::Strict,
            output_format: RadtTsOutputFormat::Mp3,
            media_format: None,
            presenter_image_path: None,
            save_presenter_image_as_project_default: false,
        };
        assert!(matches!(
            validate_clip_request(&request),
            Err(RadtTsMediaError::MissingStartBoundary)
        ));
    }

    #[test]
    fn rejects_non_finite_or_reversed_clip_times() {
        let request = StartRadtTsClipRequest {
            project_id: None,
            audio_path: "/tmp/lecture.mp3".to_string(),
            segments_json_path: "/tmp/lecture.segments.json".to_string(),
            output_name: "clip".to_string(),
            start_time: Some(f64::NAN),
            end_time: Some(1.0),
            start_phrase: None,
            end_phrase: None,
            verification_mode: RadtTsVerificationMode::Strict,
            output_format: RadtTsOutputFormat::Mp3,
            media_format: None,
            presenter_image_path: None,
            save_presenter_image_as_project_default: false,
        };
        assert!(matches!(
            validate_clip_request(&request),
            Err(RadtTsMediaError::InvalidClipBounds)
        ));
        let reversed = StartRadtTsClipRequest {
            start_time: Some(5.0),
            end_time: Some(2.0),
            ..request
        };
        assert!(matches!(
            validate_clip_request(&reversed),
            Err(RadtTsMediaError::InvalidClipBounds)
        ));
    }

    #[test]
    fn parses_cli_artifacts() {
        let transcript = parse_transcription_result(
            br#"{"segments_json_path":"/tmp/a.json","txt_path":"/tmp/a.txt","srt_path":"/tmp/a.srt"}"#,
        )
        .expect("transcription result should parse");
        assert_eq!(transcript.txt_path, "/tmp/a.txt");
        let clip = parse_clip_result(
            br#"{"clip_path":"/tmp/a.mp3","report_path":"/tmp/a.json","warnings":["short"]}"#,
        )
        .expect("clip result should parse");
        assert_eq!(clip.warnings, vec!["short"]);
    }

    #[test]
    fn does_not_rewrite_legacy_clip_reports_when_reconstructing_mp4() {
        let root =
            std::env::temp_dir().join(format!("radsuite-radt-ts-tools-{}", uuid::Uuid::new_v4()));
        let clip_path = root.join("assets/source_audio/video-clip.wav");
        let report_path = root.join("manifests/video-clip.clip.boundary.json");
        fs::create_dir_all(clip_path.parent().expect("clip parent")).expect("create clip dir");
        fs::create_dir_all(report_path.parent().expect("report parent"))
            .expect("create report dir");
        fs::write(&clip_path, [0_u8; 8]).expect("create clip artifact");
        let report_bytes = br#"{"warnings":[],"media_format":"wav"}"#.to_vec();
        fs::write(&report_path, &report_bytes).expect("create boundary report");
        let root = root.canonicalize().expect("canonicalize test directory");
        let stdout = br#"{"clip_path":"assets/source_audio/video-clip.wav","report_path":"manifests/video-clip.clip.boundary.json","warnings":[]}"#;

        let output = parse_media_output(
            MediaCommandKind::Clip,
            stdout,
            &root,
            ProjectId::new(),
            "job-1",
            Some(MediaOutputFormat::Mp4),
        )
        .expect("clip output should reconstruct");

        assert_eq!(output.output_format, Some(RadtTsOutputFormat::Wav));
        assert_eq!(output.media_format, Some(MediaOutputFormat::Mp4));
        assert_eq!(
            fs::read(root.join("manifests/video-clip.clip.boundary.json"))
                .expect("read boundary report"),
            report_bytes
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn lists_persisted_mp4_clip_format_instead_of_inferring_from_wav_path() {
        let root =
            std::env::temp_dir().join(format!("radsuite-radt-ts-tools-{}", uuid::Uuid::new_v4()));
        let clip_path = root.join("assets/source_audio/video-clip.wav");
        let report_path = root.join("manifests/video-clip.clip.boundary.json");
        fs::create_dir_all(clip_path.parent().expect("clip parent")).expect("create clip dir");
        fs::create_dir_all(report_path.parent().expect("report parent"))
            .expect("create report dir");
        fs::write(&clip_path, [0_u8; 8]).expect("create clip artifact");
        fs::write(&report_path, r#"{"warnings":[],"media_format":"mp4"}"#)
            .expect("create boundary report");
        let root = root.canonicalize().expect("canonicalize test directory");

        let outputs = list_clips(&root).expect("clips should list");
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].output_format, Some(RadtTsOutputFormat::Wav));
        assert_eq!(outputs[0].media_format, Some(MediaOutputFormat::Mp4));
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn validates_transcript_input_extension() {
        let root =
            std::env::temp_dir().join(format!("radsuite-radt-ts-tools-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let file = root.join("segments.txt");
        fs::write(&file, "[]").expect("write test file");
        let request = StartRadtTsClipRequest {
            project_id: None,
            audio_path: file.display().to_string(),
            segments_json_path: file.display().to_string(),
            output_name: "clip".to_string(),
            start_time: Some(0.0),
            end_time: Some(1.0),
            start_phrase: None,
            end_phrase: None,
            verification_mode: RadtTsVerificationMode::Strict,
            output_format: RadtTsOutputFormat::Mp3,
            media_format: None,
            presenter_image_path: None,
            save_presenter_image_as_project_default: false,
        };
        assert!(super::validate_segments_input(&file).is_err());
        assert!(
            build_clip_args(
                &request,
                ProjectId::new(),
                PathBuf::from("/tmp/projects"),
                PathBuf::from("/tmp/audio.mp3"),
                file,
                RadtTsOutputFormat::Mp3,
            )
            .is_ok()
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[cfg(unix)]
    #[test]
    fn renders_and_persists_verified_clip_mp4() {
        let data = std::env::temp_dir().join(format!(
            "radsuite-radt-ts-clip-video-{}",
            uuid::Uuid::new_v4()
        ));
        let project_id = ProjectId::new();
        let root = data.join("radt-ts-projects").join(project_id.to_string());
        let audio = root.join("assets/source_audio/opening.wav");
        let report = root.join("manifests/opening.clip.boundary.json");
        let image = data.join("presenter.png");
        fs::create_dir_all(audio.parent().unwrap()).unwrap();
        fs::create_dir_all(report.parent().unwrap()).unwrap();
        fs::write(&audio, b"wav").unwrap();
        fs::write(&report, br#"{"warnings":[]}"#).unwrap();
        fs::write(&image, b"\x89PNG\r\n\x1a\n").unwrap();
        let probe = write_test_executable(&data, "audio-probe.sh", "#!/bin/sh\nprintf '4.0\\n'\n");
        let ffmpeg = write_test_executable(
            &data,
            "video-ffmpeg.sh",
            "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf video > \"$output\"\nprintf 'out_time_us=4000000\\nprogress=end\\n'\n",
        );
        let ffprobe = write_test_executable(
            &data,
            "video-ffprobe.sh",
            "#!/bin/sh\nprintf '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":1280,\"height\":720,\"pix_fmt\":\"yuv420p\",\"r_frame_rate\":\"30/1\",\"avg_frame_rate\":\"30/1\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}],\"format\":{\"duration\":4.0}}'\n",
        );
        let request = StartRadtTsClipRequest {
            project_id: Some(project_id),
            audio_path: audio.to_string_lossy().into_owned(),
            segments_json_path: report.to_string_lossy().into_owned(),
            output_name: "opening".to_string(),
            start_time: Some(0.0),
            end_time: Some(4.0),
            start_phrase: None,
            end_phrase: None,
            verification_mode: RadtTsVerificationMode::Strict,
            output_format: RadtTsOutputFormat::Wav,
            media_format: Some(MediaOutputFormat::Mp4),
            presenter_image_path: Some(image.to_string_lossy().into_owned()),
            save_presenter_image_as_project_default: true,
        };
        let source = RadtTsMediaOutput {
            id: uuid::Uuid::new_v4().to_string(),
            kind: RadtTsMediaJobKind::Clip,
            name: "opening".to_string(),
            primary_path: audio.to_string_lossy().into_owned(),
            artifacts: vec![RadtTsMediaArtifact {
                label: "Boundary report".to_string(),
                path: report.to_string_lossy().into_owned(),
            }],
            output_format: Some(RadtTsOutputFormat::Wav),
            media_format: Some(MediaOutputFormat::Wav),
            image_path: None,
            warnings: Vec::new(),
        };

        let output = finalize_clip_video(
            ClipVideoInput {
                data_dir: &data,
                project_root: &root,
                project_id,
                request: &request,
                audio: source,
            },
            ClipVideoTools {
                audio_processor: radsuite_engines::AudioProcessor::from_commands(&ffmpeg, probe),
                video_exporter: radsuite_engines::VideoExporter::from_commands(ffmpeg, ffprobe),
            },
            || false,
            |_| {},
        )
        .unwrap();

        assert!(output.primary_path.ends_with(".mp4"));
        assert!(PathBuf::from(&output.primary_path).is_file());
        assert!(!audio.exists());
        let image_path = PathBuf::from(output.image_path.as_ref().unwrap());
        assert!(image_path.is_file());
        assert_eq!(
            read_persisted_clip_outputs(&root).unwrap(),
            vec![output.clone()]
        );
        delete_radt_ts_clip_video_from_root(&root, &output.id).unwrap();
        assert!(!PathBuf::from(&output.primary_path).exists());
        assert!(!report.exists());
        assert!(image_path.exists());
        assert!(read_persisted_clip_outputs(&root).unwrap().is_empty());
        fs::remove_dir_all(data).unwrap();
    }

    #[cfg(unix)]
    fn write_test_executable(root: &std::path::Path, name: &str, body: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(root).unwrap();
        fs::write(&path, body).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).unwrap();
        path
    }
}
