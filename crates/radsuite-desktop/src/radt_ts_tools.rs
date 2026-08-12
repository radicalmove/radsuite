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
    process::{Child, Command},
};
use uuid::Uuid;

use crate::{
    DesktopState,
    radt_ts::{
        RadtTsCapabilityStatus, RadtTsOutputFormat, contained_file, discover_radt_ts_cli,
        ensure_project_root, validate_output_name,
    },
};

pub type RadtTsMediaChildHandle = Arc<Mutex<Option<Child>>>;

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
    pub warnings: Vec<String>,
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
    pub output_format: RadtTsOutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsVerificationMode {
    Strict,
    Lenient,
}

#[derive(Debug, Error)]
pub enum RadtTsMediaError {
    #[error("RADTTS media tools are currently supported on macOS and Linux only")]
    UnsupportedPlatform,
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
}

#[derive(Debug, Clone, Copy)]
enum MediaCommandKind {
    Transcription,
    Clip,
}

struct MediaJobContext {
    project_id: ProjectId,
    kind: MediaCommandKind,
    project_root: PathBuf,
    child_handle: RadtTsMediaChildHandle,
    jobs: Arc<Mutex<std::collections::HashMap<String, RadtTsMediaJobStatus>>>,
    children: Arc<Mutex<std::collections::HashMap<String, RadtTsMediaChildHandle>>>,
    cancellations: Arc<Mutex<std::collections::HashSet<String>>>,
    active_projects: Arc<Mutex<std::collections::HashSet<String>>>,
}

pub fn get_radt_ts_media_capabilities() -> RadtTsCapabilityStatus {
    discover_radt_ts_cli()
}

pub async fn start_radt_ts_transcription(
    state: &DesktopState,
    request: StartRadtTsTranscriptionRequest,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    if cfg!(windows) {
        return Err(RadtTsMediaError::UnsupportedPlatform);
    }
    let project_id = request.project_id.unwrap_or_default();
    let executable = media_executable()?;
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
        project_id,
        MediaCommandKind::Transcription,
        project_root,
        executable,
        args,
    )
    .await
}

pub async fn start_radt_ts_clip(
    state: &DesktopState,
    request: StartRadtTsClipRequest,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
    if cfg!(windows) {
        return Err(RadtTsMediaError::UnsupportedPlatform);
    }
    let project_id = request.project_id.unwrap_or_default();
    let executable = media_executable()?;
    let projects_root = state.paths.data_dir.join("radt-ts-projects");
    let project_root = ensure_project_root(&projects_root, project_id)
        .map_err(|error| RadtTsMediaError::Io(std::io::Error::other(error.to_string())))?;
    let audio_path = validate_audio_input(Path::new(&request.audio_path))?;
    let segments_path = validate_segments_input(Path::new(&request.segments_json_path))?;
    validate_clip_request(&request)?;
    let args = build_clip_args(
        &request,
        project_id,
        projects_root,
        audio_path,
        segments_path,
    )?;
    start_media_process(
        state,
        project_id,
        MediaCommandKind::Clip,
        project_root,
        executable,
        args,
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
    outputs.extend(list_clips(&root)?);
    outputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(RadtTsMediaOutputListing { outputs })
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
        request.output_format.as_cli_value().to_string(),
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
    project_id: ProjectId,
    kind: MediaCommandKind,
    project_root: PathBuf,
    executable: PathBuf,
    args: Vec<String>,
) -> Result<RadtTsMediaJobStatus, RadtTsMediaError> {
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
    configure_process_group(&mut command);
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            remove_active_project(state, project_id);
            return Err(RadtTsMediaError::Io(error));
        }
    };
    let child_handle = Arc::new(Mutex::new(Some(child)));
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
        project_root,
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
        project_root,
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
        let Some(child) = guard.as_mut() else {
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
                .and_then(|child| child.try_wait().transpose())
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
                    parse_media_output(kind, &stdout, &project_root, project_id, &job_id)
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
                warnings: Vec::new(),
            })
        }
        MediaCommandKind::Clip => {
            let result = parse_clip_result(stdout)?;
            let clip_path = contained_media_file(project_root, Path::new(&result.clip_path))?;
            let report_path = contained_media_file(project_root, Path::new(&result.report_path))?;
            let mut warnings = result.warnings;
            if let Ok(report) =
                serde_json::from_slice::<ClipBoundaryReport>(&fs::read(&report_path)?)
            {
                warnings.extend(report.warnings);
            }
            warnings.sort();
            warnings.dedup();
            let name = clip_path
                .file_stem()
                .and_then(|value| value.to_str())
                .ok_or_else(|| RadtTsMediaError::InvalidOutput(result.clip_path.clone()))?
                .to_string();
            Ok(RadtTsMediaOutput {
                id: job_id.to_string(),
                kind: RadtTsMediaJobKind::Clip,
                name,
                primary_path: clip_path.display().to_string(),
                artifacts: vec![RadtTsMediaArtifact {
                    label: "Boundary report".to_string(),
                    path: report_path.display().to_string(),
                }],
                output_format: clip_path
                    .extension()
                    .and_then(|value| value.to_str())
                    .and_then(|value| match value {
                        "wav" => Some(RadtTsOutputFormat::Wav),
                        "mp3" => Some(RadtTsOutputFormat::Mp3),
                        _ => None,
                    }),
                warnings,
            })
        }
    }
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

fn media_executable() -> Result<PathBuf, RadtTsMediaError> {
    let capability = discover_radt_ts_cli();
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
        outputs.push(RadtTsMediaOutput {
            id: format!("clip:{name}"),
            kind: RadtTsMediaJobKind::Clip,
            name: name.to_string(),
            primary_path: clip_path.display().to_string(),
            artifacts: vec![RadtTsMediaArtifact {
                label: "Boundary report".to_string(),
                path: report_path.display().to_string(),
            }],
            output_format: clip_path
                .extension()
                .and_then(|value| value.to_str())
                .and_then(|value| match value {
                    "wav" => Some(RadtTsOutputFormat::Wav),
                    "mp3" => Some(RadtTsOutputFormat::Mp3),
                    _ => None,
                }),
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
    let Some(child) = guard.as_mut() else { return };
    if let Some(pid) = child.id() {
        terminate_process_group(pid, force);
    }
    if force {
        let _ = child.start_kill();
    }
}

fn terminate_process_group(pid: u32, force: bool) {
    #[cfg(unix)]
    {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        unsafe {
            let _ = libc::kill(-(pid as libc::pid_t), signal);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, force);
    }
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    #[cfg(not(unix))]
    {
        let _ = command;
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

    use radsuite_core::ProjectId;

    use super::{
        RadtTsMediaError, RadtTsOutputFormat, RadtTsVerificationMode, StartRadtTsClipRequest,
        StartRadtTsTranscriptionRequest, build_clip_args, build_transcription_args,
        parse_clip_result, parse_transcription_result, validate_clip_request,
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
        };
        let args = build_clip_args(
            &request,
            ProjectId::new(),
            PathBuf::from("/tmp/projects"),
            PathBuf::from("/tmp/lecture.mp3"),
            PathBuf::from("/tmp/lecture.segments.json"),
        )
        .expect("valid clip request should build");
        assert!(args.contains(&"--start-phrase".to_string()));
        assert!(args.contains(&"Welcome to the course".to_string()));
        assert!(args.contains(&"lenient".to_string()));
        assert!(args.contains(&"wav".to_string()));
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
        };
        assert!(super::validate_segments_input(&file).is_err());
        assert!(
            build_clip_args(
                &request,
                ProjectId::new(),
                PathBuf::from("/tmp/projects"),
                PathBuf::from("/tmp/audio.mp3"),
                file,
            )
            .is_ok()
        );
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
