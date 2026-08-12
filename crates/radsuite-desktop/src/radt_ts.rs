use std::{
    collections::HashMap,
    env, fs,
    io::Write,
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

use crate::DesktopState;

pub type RadtTsChildHandle = Arc<Mutex<Option<Child>>>;
pub type RadtTsTempFileSet = (PathBuf, Option<PathBuf>);

#[derive(Debug, Default)]
pub struct RadtTsLifecycleState {
    pub shutting_down: bool,
    pub temp_files: HashMap<String, RadtTsTempFileSet>,
}

pub type RadtTsLifecycleRegistry = Arc<Mutex<RadtTsLifecycleState>>;

const MAX_REFERENCE_AUDIO_BYTES: u64 = 250 * 1024 * 1024;
const MAX_OUTPUT_NAME_LENGTH: usize = 80;
const DEFAULT_MAX_NEW_TOKENS: u32 = 1200;
const MIN_MAX_NEW_TOKENS: u32 = 64;
const MAX_MAX_NEW_TOKENS: u32 = 8192;

fn default_max_new_tokens() -> u32 {
    DEFAULT_MAX_NEW_TOKENS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsQuality {
    Fast,
    High,
}

impl RadtTsQuality {
    pub(crate) fn as_cli_value(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::High => "quality",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsVoiceSource {
    #[default]
    Reference,
    Builtin,
}

impl RadtTsVoiceSource {
    fn as_cli_value(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Builtin => "builtin",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsChunkMode {
    Single,
    Sentence,
}

impl RadtTsChunkMode {
    fn as_cli_value(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Sentence => "sentence",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsOutputFormat {
    Mp3,
    Wav,
}

impl RadtTsOutputFormat {
    pub(crate) fn as_cli_value(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadtTsSynthesisRequest {
    pub project_id: ProjectId,
    pub text: String,
    pub voice_source: RadtTsVoiceSource,
    pub reference_audio_path: Option<PathBuf>,
    #[serde(default)]
    pub reference_text: Option<String>,
    #[serde(default)]
    pub built_in_speaker: Option<String>,
    #[serde(default)]
    pub built_in_instruct: Option<String>,
    pub quality: RadtTsQuality,
    pub chunk_mode: RadtTsChunkMode,
    pub pause_min_seconds: f64,
    pub pause_max_seconds: f64,
    pub pause_seed: Option<i64>,
    pub max_new_tokens: u32,
    pub output_format: RadtTsOutputFormat,
    pub output_name: String,
    pub acknowledge_voice_clone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RadtTsCapabilityStatus {
    pub available: bool,
    pub executable: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsJobState {
    Starting,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RadtTsProcessingPhase {
    Preparing,
    Generating,
    SavingOutput,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadtTsAudioOutput {
    pub id: String,
    pub filename: String,
    pub path: String,
    pub output_format: RadtTsOutputFormat,
    pub caption_paths: Vec<String>,
    pub duration_seconds: Option<f64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadtTsJobStatus {
    pub id: String,
    pub project_id: ProjectId,
    pub state: RadtTsJobState,
    pub phase: RadtTsProcessingPhase,
    pub percent: Option<u8>,
    pub output: Option<RadtTsAudioOutput>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListRadtTsOutputsRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartRadtTsSynthesisRequest {
    #[serde(default)]
    pub project_id: Option<ProjectId>,
    pub text: String,
    #[serde(default)]
    pub voice_source: RadtTsVoiceSource,
    #[serde(default)]
    pub reference_audio_path: Option<String>,
    #[serde(default)]
    pub reference_text: Option<String>,
    #[serde(default)]
    pub built_in_speaker: Option<String>,
    #[serde(default)]
    pub built_in_instruct: Option<String>,
    pub quality: RadtTsQuality,
    pub chunk_mode: RadtTsChunkMode,
    pub pause_min_seconds: f64,
    pub pause_max_seconds: f64,
    #[serde(default)]
    pub pause_seed: Option<i64>,
    #[serde(default = "default_max_new_tokens")]
    pub max_new_tokens: u32,
    pub output_format: RadtTsOutputFormat,
    pub output_name: String,
    pub acknowledge_voice_clone: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RadtTsOutputListing {
    pub outputs: Vec<RadtTsAudioOutput>,
}

#[derive(Debug, Error)]
pub enum RadtTsError {
    #[error("RADTTS voice generation is currently supported on macOS and Linux only")]
    UnsupportedPlatform,
    #[error("RADTTS is not available: {0}")]
    MissingCli(String),
    #[error("voice generation text cannot be empty")]
    EmptyText,
    #[error("voice-clone authorization is required before using reference audio")]
    MissingVoiceCloneAuthorization,
    #[error("a built-in speaker is required for built-in voice generation")]
    MissingBuiltInSpeaker,
    #[error("pause maximum must be greater than or equal to pause minimum")]
    InvalidPauseBounds,
    #[error("maximum new tokens must be between 64 and 8192")]
    InvalidMaxNewTokens,
    #[error("invalid output name: {0}")]
    InvalidOutputName(String),
    #[error("invalid reference audio: {0}")]
    InvalidReferenceAudio(String),
    #[error("RADTTS synthesis is already running for this project")]
    JobAlreadyRunning,
    #[error("RADTTS job {0} was not found")]
    MissingJob(String),
    #[error("RADTTS synthesis was cancelled")]
    Cancelled,
    #[error("RADTTS synthesis timed out")]
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
pub(crate) struct RadtTsCliResult {
    pub job_id: String,
    pub status: String,
    pub stage: String,
    pub outputs: RadtTsCliOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct RadtTsCliOutput {
    pub output_file: Option<String>,
    pub metadata_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct RadtTsOutputMetadata {
    output_file: String,
    duration_seconds: Option<f64>,
    output_format: Option<RadtTsOutputFormat>,
    created_at: Option<String>,
    captions: Option<HashMap<String, String>>,
    project_id: String,
    job_id: String,
}

const MAX_CAPTURED_OUTPUT_BYTES: usize = 512 * 1024;
const JOB_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const TERMINATION_GRACE: Duration = Duration::from_secs(2);

pub async fn start_radt_ts_synthesis(
    state: &DesktopState,
    request: RadtTsSynthesisRequest,
) -> Result<RadtTsJobStatus, RadtTsError> {
    if cfg!(windows) {
        return Err(RadtTsError::UnsupportedPlatform);
    }

    let capability = discover_radt_ts_cli();
    let executable = capability
        .executable
        .map(PathBuf::from)
        .ok_or(RadtTsError::MissingCli(capability.detail))?;
    let projects_root = state.paths.data_dir.join("radt-ts-projects");
    let project_root = ensure_project_root(&projects_root, request.project_id)?;
    build_synthesis_args(
        &request,
        projects_root.clone(),
        state.paths.cache_dir.join("pending.txt"),
        None,
    )?;

    {
        let mut active_projects = state
            .radt_ts_active_projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !active_projects.insert(request.project_id.to_string()) {
            return Err(RadtTsError::JobAlreadyRunning);
        }
    }

    let text_path = match create_temp_text_file(&state.paths.cache_dir, &request.text) {
        Ok(path) => path,
        Err(error) => {
            state
                .radt_ts_active_projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&request.project_id.to_string());
            return Err(error);
        }
    };
    let reference_text_path = if request.voice_source == RadtTsVoiceSource::Reference {
        match request
            .reference_text
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(reference_text) => {
                match create_temp_text_file(&state.paths.cache_dir, reference_text) {
                    Ok(path) => Some(path),
                    Err(error) => {
                        let _ = fs::remove_file(&text_path);
                        state
                            .radt_ts_active_projects
                            .lock()
                            .unwrap_or_else(|poisoned| poisoned.into_inner())
                            .remove(&request.project_id.to_string());
                        return Err(error);
                    }
                }
            }
            None => None,
        }
    } else {
        None
    };
    let args = match build_synthesis_args(
        &request,
        projects_root,
        text_path.clone(),
        reference_text_path.clone(),
    ) {
        Ok(args) => args,
        Err(error) => {
            remove_temp_text_files(&text_path, reference_text_path.as_deref());
            state
                .radt_ts_active_projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&request.project_id.to_string());
            return Err(error);
        }
    };
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
            remove_temp_text_files(&text_path, reference_text_path.as_deref());
            state
                .radt_ts_active_projects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&request.project_id.to_string());
            return Err(RadtTsError::Io(error));
        }
    };
    let child_handle = Arc::new(Mutex::new(Some(child)));
    let job_id = Uuid::new_v4().to_string();
    let initial = RadtTsJobStatus {
        id: job_id.clone(),
        project_id: request.project_id,
        state: RadtTsJobState::Starting,
        phase: RadtTsProcessingPhase::Preparing,
        percent: None,
        output: None,
        error: None,
    };
    let lifecycle = state.radt_ts_lifecycle.clone();
    let mut lifecycle_guard = lifecycle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if lifecycle_guard.shutting_down {
        drop(lifecycle_guard);
        request_process_termination(&child_handle, true);
        remove_temp_text_files(&text_path, reference_text_path.as_deref());
        state
            .radt_ts_active_projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&request.project_id.to_string());
        return Err(RadtTsError::Cancelled);
    }
    state
        .radt_ts_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.clone(), initial.clone());
    state
        .radt_ts_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.clone(), child_handle.clone());
    state
        .radt_ts_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&job_id);
    lifecycle_guard.temp_files.insert(
        job_id.clone(),
        (text_path.clone(), reference_text_path.clone()),
    );
    drop(lifecycle_guard);

    let jobs = state.radt_ts_jobs.clone();
    let children = state.radt_ts_children.clone();
    let cancellations = state.radt_ts_cancel_requests.clone();
    let active_projects = state.radt_ts_active_projects.clone();
    let lifecycle = state.radt_ts_lifecycle.clone();
    let context = RadtTsJobContext {
        request,
        project_root,
        child_handle,
        jobs,
        children,
        cancellations,
        active_projects,
        lifecycle,
    };
    tokio::spawn(async move { run_radt_ts_job(job_id, context).await });

    Ok(initial)
}

pub fn get_radt_ts_job(state: &DesktopState, job_id: &str) -> Result<RadtTsJobStatus, RadtTsError> {
    state
        .radt_ts_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(job_id)
        .cloned()
        .ok_or_else(|| RadtTsError::MissingJob(job_id.to_string()))
}

pub fn cancel_radt_ts_job(
    state: &DesktopState,
    job_id: &str,
) -> Result<RadtTsJobStatus, RadtTsError> {
    let current = get_radt_ts_job(state, job_id)?;
    if matches!(
        current.state,
        RadtTsJobState::Completed | RadtTsJobState::Failed | RadtTsJobState::Cancelled
    ) {
        return Ok(current);
    }

    state
        .radt_ts_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(job_id.to_string());
    if let Some(handle) = state
        .radt_ts_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(job_id)
        .cloned()
    {
        request_process_termination(&handle, false);
    }
    let mut jobs = state
        .radt_ts_jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let updated = jobs
        .get_mut(job_id)
        .ok_or_else(|| RadtTsError::MissingJob(job_id.to_string()))?;
    updated.state = RadtTsJobState::Cancelled;
    updated.error = Some("Voice generation cancelled.".to_string());
    Ok(updated.clone())
}

pub fn list_radt_ts_outputs_for_project(
    state: &DesktopState,
    project_id: ProjectId,
) -> Result<RadtTsOutputListing, RadtTsError> {
    let root = ensure_project_root(&state.paths.data_dir.join("radt-ts-projects"), project_id)?;
    let outputs_path = root.join("manifests").join("outputs.json");
    let raw = match fs::read(&outputs_path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(RadtTsOutputListing {
                outputs: Vec::new(),
            });
        }
        Err(error) => return Err(RadtTsError::Io(error)),
    };
    let metadata: Vec<RadtTsOutputMetadata> = serde_json::from_slice(&raw)
        .map_err(|error| RadtTsError::InvalidCliResult(error.to_string()))?;
    let outputs = metadata
        .into_iter()
        .filter_map(|item| output_from_metadata(&root, &item).ok())
        .collect();
    Ok(RadtTsOutputListing { outputs })
}

pub fn shutdown_radt_ts_jobs(state: &DesktopState) {
    let mut lifecycle_guard = state
        .radt_ts_lifecycle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    lifecycle_guard.shutting_down = true;
    let handles = state
        .radt_ts_children
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .map(|(job_id, handle)| (job_id.clone(), handle.clone()))
        .collect::<Vec<_>>();
    state
        .radt_ts_cancel_requests
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .extend(handles.iter().map(|(job_id, _)| job_id.clone()));
    for (job_id, _) in &handles {
        if let Some(job) = state
            .radt_ts_jobs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(job_id)
            && matches!(
                job.state,
                RadtTsJobState::Starting | RadtTsJobState::Running
            )
        {
            job.state = RadtTsJobState::Cancelled;
            job.error = Some("Voice generation stopped because RADsuite is closing.".to_string());
        }
    }
    for (_, handle) in handles {
        request_process_termination(&handle, true);
    }
    let temp_files = lifecycle_guard
        .temp_files
        .drain()
        .map(|(_, paths)| paths)
        .collect::<Vec<_>>();
    drop(lifecycle_guard);
    for (text_path, reference_text_path) in temp_files {
        remove_temp_text_files(&text_path, reference_text_path.as_deref());
    }
}

struct RadtTsJobContext {
    request: RadtTsSynthesisRequest,
    project_root: PathBuf,
    child_handle: RadtTsChildHandle,
    jobs: Arc<Mutex<HashMap<String, RadtTsJobStatus>>>,
    children: Arc<Mutex<HashMap<String, RadtTsChildHandle>>>,
    cancellations: Arc<Mutex<std::collections::HashSet<String>>>,
    active_projects: Arc<Mutex<std::collections::HashSet<String>>>,
    lifecycle: RadtTsLifecycleRegistry,
}

async fn run_radt_ts_job(job_id: String, context: RadtTsJobContext) {
    let RadtTsJobContext {
        request,
        project_root,
        child_handle,
        jobs,
        children,
        cancellations,
        active_projects,
        lifecycle,
    } = context;
    update_job(&jobs, &job_id, |job| {
        job.state = RadtTsJobState::Running;
        job.phase = RadtTsProcessingPhase::Generating;
    });

    let (stdout, stderr) = {
        let mut guard = child_handle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let child = match guard.as_mut() {
            Some(child) => child,
            None => {
                finish_radt_ts_job(
                    &jobs,
                    &children,
                    &cancellations,
                    &active_projects,
                    &job_id,
                    Err(RadtTsError::CliFailed(
                        "process handle disappeared".to_string(),
                    )),
                );
                cleanup_registered_radt_ts_temp_files(&lifecycle, &job_id);
                return;
            }
        };
        (child.stdout.take(), child.stderr.take())
    };
    let stdout_task = tokio::spawn(read_limited(stdout));
    let stderr_task = tokio::spawn(read_limited(stderr));
    let started = Instant::now();
    let mut termination_deadline: Option<Instant> = None;
    let mut termination_reason: Option<RadtTsError> = None;

    loop {
        let cancelled = cancellations
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .contains(&job_id);
        if termination_deadline.is_none() && (cancelled || started.elapsed() >= JOB_TIMEOUT) {
            termination_reason = Some(if cancelled {
                RadtTsError::Cancelled
            } else {
                RadtTsError::TimedOut
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
                if let Some(reason) = termination_reason {
                    finish_radt_ts_job(
                        &jobs,
                        &children,
                        &cancellations,
                        &active_projects,
                        &job_id,
                        Err(reason),
                    );
                } else if !status.success() {
                    let detail = String::from_utf8_lossy(&stderr).trim().to_string();
                    finish_radt_ts_job(
                        &jobs,
                        &children,
                        &cancellations,
                        &active_projects,
                        &job_id,
                        Err(RadtTsError::CliFailed(if detail.is_empty() {
                            format!("exit status {status}")
                        } else {
                            detail
                        })),
                    );
                } else {
                    let result = parse_cli_result(&stdout).and_then(|result| {
                        output_from_cli_result(&project_root, request.project_id, result)
                    });
                    finish_radt_ts_job(
                        &jobs,
                        &children,
                        &cancellations,
                        &active_projects,
                        &job_id,
                        result,
                    );
                }
                break;
            }
            Ok(None) => tokio::time::sleep(Duration::from_millis(100)).await,
            Err(error) => {
                finish_radt_ts_job(
                    &jobs,
                    &children,
                    &cancellations,
                    &active_projects,
                    &job_id,
                    Err(RadtTsError::Io(error)),
                );
                break;
            }
        }
    }
    cleanup_registered_radt_ts_temp_files(&lifecycle, &job_id);
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

fn finish_radt_ts_job(
    jobs: &Arc<Mutex<HashMap<String, RadtTsJobStatus>>>,
    children: &Arc<Mutex<HashMap<String, RadtTsChildHandle>>>,
    cancellations: &Arc<Mutex<std::collections::HashSet<String>>>,
    active_projects: &Arc<Mutex<std::collections::HashSet<String>>>,
    job_id: &str,
    result: Result<RadtTsAudioOutput, RadtTsError>,
) {
    let project_id = if let Some(job) = jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(job_id)
    {
        let project_id = job.project_id.to_string();
        match result {
            Ok(output) => {
                job.state = RadtTsJobState::Completed;
                job.phase = RadtTsProcessingPhase::SavingOutput;
                job.percent = Some(100);
                job.output = Some(output);
                job.error = None;
            }
            Err(error) => {
                job.state = if matches!(&error, RadtTsError::Cancelled) {
                    RadtTsJobState::Cancelled
                } else {
                    RadtTsJobState::Failed
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
    if let Some(project_id) = project_id {
        active_projects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&project_id);
    }
    cancellations
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(job_id);
}

fn update_job<F>(jobs: &Arc<Mutex<HashMap<String, RadtTsJobStatus>>>, job_id: &str, update: F)
where
    F: FnOnce(&mut RadtTsJobStatus),
{
    if let Some(job) = jobs
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get_mut(job_id)
    {
        update(job);
    }
}

fn request_process_termination(handle: &RadtTsChildHandle, force: bool) {
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
        // The child is placed in a dedicated process group before spawn.
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

pub(crate) fn ensure_project_root(
    projects_root: &Path,
    project_id: ProjectId,
) -> Result<PathBuf, RadtTsError> {
    let project_root = projects_root.join(project_id.to_string());
    for relative in [
        "assets/source_audio",
        "assets/reference_audio",
        "assets/generated_audio",
        "transcripts",
        "captions",
        "manifests",
    ] {
        fs::create_dir_all(project_root.join(relative))?;
    }
    for (name, value) in [("jobs.json", "[]"), ("outputs.json", "[]")] {
        let path = project_root.join("manifests").join(name);
        if !path.exists() {
            fs::write(path, value)?;
        }
    }
    Ok(project_root.canonicalize()?)
}

fn create_temp_text_file(cache_dir: &Path, text: &str) -> Result<PathBuf, RadtTsError> {
    fs::create_dir_all(cache_dir)?;
    for _ in 0..5 {
        let path = cache_dir.join(format!("radt-ts-{}.txt", Uuid::new_v4()));
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(text.as_bytes()) {
                    let _ = fs::remove_file(&path);
                    return Err(RadtTsError::Io(error));
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(RadtTsError::Io(error)),
        }
    }
    Err(RadtTsError::Io(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not create a unique RADTTS text file",
    )))
}

fn remove_temp_text_files(text_path: &Path, reference_text_path: Option<&Path>) {
    let _ = fs::remove_file(text_path);
    if let Some(path) = reference_text_path {
        let _ = fs::remove_file(path);
    }
}

fn cleanup_registered_radt_ts_temp_files(lifecycle: &RadtTsLifecycleRegistry, job_id: &str) {
    let paths = lifecycle
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .temp_files
        .remove(job_id);
    if let Some((text_path, reference_text_path)) = paths {
        remove_temp_text_files(&text_path, reference_text_path.as_deref());
    }
}

fn output_from_cli_result(
    project_root: &Path,
    project_id: ProjectId,
    result: RadtTsCliResult,
) -> Result<RadtTsAudioOutput, RadtTsError> {
    if result.status != "completed" {
        return Err(RadtTsError::CliFailed(result.status.to_string()));
    }
    let metadata_path =
        result.outputs.metadata_path.as_deref().ok_or_else(|| {
            RadtTsError::InvalidOutput("metadata path was not returned".to_string())
        })?;
    let metadata_path = contained_file(project_root, Path::new(metadata_path))?;
    let metadata: RadtTsOutputMetadata = serde_json::from_slice(&fs::read(metadata_path)?)
        .map_err(|error| RadtTsError::InvalidCliResult(error.to_string()))?;
    if metadata.project_id != project_id.to_string() || metadata.job_id != result.job_id {
        return Err(RadtTsError::InvalidOutput(
            "output identity did not match the active project".to_string(),
        ));
    }
    output_from_metadata(project_root, &metadata)
}

fn output_from_metadata(
    project_root: &Path,
    metadata: &RadtTsOutputMetadata,
) -> Result<RadtTsAudioOutput, RadtTsError> {
    let output_path = contained_file(project_root, Path::new(&metadata.output_file))?;
    let filename = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| RadtTsError::InvalidOutput(metadata.output_file.clone()))?
        .to_string();
    let caption_paths = metadata
        .captions
        .as_ref()
        .into_iter()
        .flat_map(|captions| captions.values())
        .filter_map(|path| contained_file(project_root, Path::new(path)).ok())
        .map(|path| path.display().to_string())
        .collect();
    let output_format = metadata.output_format.unwrap_or_else(|| {
        if output_path.extension().and_then(|value| value.to_str()) == Some("wav") {
            RadtTsOutputFormat::Wav
        } else {
            RadtTsOutputFormat::Mp3
        }
    });
    Ok(RadtTsAudioOutput {
        id: metadata.job_id.clone(),
        filename,
        path: output_path.display().to_string(),
        output_format,
        caption_paths,
        duration_seconds: metadata.duration_seconds,
        created_at: metadata.created_at.clone(),
    })
}

pub(crate) fn contained_file(root: &Path, path: &Path) -> Result<PathBuf, RadtTsError> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|_| RadtTsError::InvalidOutput(path.display().to_string()))?;
    if !canonical.is_file() || !canonical.starts_with(root) {
        return Err(RadtTsError::InvalidOutput(path.display().to_string()));
    }
    Ok(canonical)
}

pub fn validate_output_name(value: &str) -> Result<(), RadtTsError> {
    if value.is_empty() || value.len() > MAX_OUTPUT_NAME_LENGTH || value == "." || value == ".." {
        return Err(RadtTsError::InvalidOutputName(value.to_string()));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        return Err(RadtTsError::InvalidOutputName(value.to_string()));
    }
    Ok(())
}

pub fn validate_reference_audio(path: &Path) -> Result<PathBuf, RadtTsError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        RadtTsError::InvalidReferenceAudio(format!("{} ({error})", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RadtTsError::InvalidReferenceAudio(
            "choose a regular audio file, not a folder or symlink".to_string(),
        ));
    }
    if metadata.len() > MAX_REFERENCE_AUDIO_BYTES {
        return Err(RadtTsError::InvalidReferenceAudio(
            "the reference audio must be 250 MB or smaller".to_string(),
        ));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    if !matches!(
        extension.as_deref(),
        Some("wav" | "mp3" | "m4a" | "flac" | "ogg" | "webm" | "aac")
    ) {
        return Err(RadtTsError::InvalidReferenceAudio(
            "use WAV, MP3, M4A, FLAC, OGG, WEBM, or AAC audio".to_string(),
        ));
    }
    path.canonicalize().map_err(|error| {
        RadtTsError::InvalidReferenceAudio(format!("{} ({error})", path.display()))
    })
}

pub fn build_synthesis_args(
    request: &RadtTsSynthesisRequest,
    projects_root: PathBuf,
    text_file: PathBuf,
    reference_text_file: Option<PathBuf>,
) -> Result<Vec<String>, RadtTsError> {
    if request.text.trim().is_empty() {
        return Err(RadtTsError::EmptyText);
    }
    if request.voice_source == RadtTsVoiceSource::Reference && !request.acknowledge_voice_clone {
        return Err(RadtTsError::MissingVoiceCloneAuthorization);
    }
    if request.pause_min_seconds <= 0.0 || request.pause_max_seconds < request.pause_min_seconds {
        return Err(RadtTsError::InvalidPauseBounds);
    }
    if !(MIN_MAX_NEW_TOKENS..=MAX_MAX_NEW_TOKENS).contains(&request.max_new_tokens) {
        return Err(RadtTsError::InvalidMaxNewTokens);
    }
    validate_output_name(&request.output_name)?;
    let mut args = vec![
        "--projects-root".to_string(),
        projects_root.display().to_string(),
        "synthesize".to_string(),
        "--project-id".to_string(),
        request.project_id.to_string(),
        "--text-file".to_string(),
        text_file.display().to_string(),
        "--voice-source".to_string(),
        request.voice_source.as_cli_value().to_string(),
        "--mode".to_string(),
        request.quality.as_cli_value().to_string(),
        "--max-new-tokens".to_string(),
        request.max_new_tokens.to_string(),
        "--chunk-mode".to_string(),
        request.chunk_mode.as_cli_value().to_string(),
        "--pause-min".to_string(),
        request.pause_min_seconds.to_string(),
        "--pause-max".to_string(),
        request.pause_max_seconds.to_string(),
        "--output-format".to_string(),
        request.output_format.as_cli_value().to_string(),
        "--output-name".to_string(),
        request.output_name.clone(),
    ];
    match request.voice_source {
        RadtTsVoiceSource::Reference => {
            let reference_audio_path =
                request.reference_audio_path.as_deref().ok_or_else(|| {
                    RadtTsError::InvalidReferenceAudio("reference audio is required".to_string())
                })?;
            let reference_audio_path = validate_reference_audio(reference_audio_path)?;
            args.extend([
                "--reference-audio".to_string(),
                reference_audio_path.display().to_string(),
                "--ack-voice-clone".to_string(),
            ]);
        }
        RadtTsVoiceSource::Builtin => {
            let speaker = request
                .built_in_speaker
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or(RadtTsError::MissingBuiltInSpeaker)?;
            args.extend(["--built-in-speaker".to_string(), speaker.to_string()]);
            if let Some(instruct) = request
                .built_in_instruct
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                args.extend(["--built-in-instruct".to_string(), instruct.to_string()]);
            }
        }
    }
    if let Some(pause_seed) = request.pause_seed {
        args.extend(["--pause-seed".to_string(), pause_seed.to_string()]);
    }
    if request.voice_source == RadtTsVoiceSource::Reference
        && let Some(reference_text_file) = reference_text_file
    {
        args.extend([
            "--reference-text-file".to_string(),
            reference_text_file.display().to_string(),
        ]);
    }
    Ok(args)
}

pub(crate) fn parse_cli_result(stdout: &[u8]) -> Result<RadtTsCliResult, RadtTsError> {
    serde_json::from_slice(stdout).map_err(|error| RadtTsError::InvalidCliResult(error.to_string()))
}

pub fn discover_radt_ts_cli() -> RadtTsCapabilityStatus {
    if cfg!(windows) {
        return RadtTsCapabilityStatus {
            available: false,
            executable: None,
            detail: "Voice generation will be available on Windows after process cleanup support is added.".to_string(),
        };
    }

    let override_path = env::var_os("RADSUITE_RADTTS_CLI").map(PathBuf::from);
    let home_path = env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("RADTTS").join(".venv").join("bin").join("radtts"));
    let candidates = override_path
        .into_iter()
        .chain(find_on_path("radtts"))
        .chain(home_path)
        .collect::<Vec<_>>();
    let executable = candidates.into_iter().find(|candidate| candidate.is_file());
    match executable {
        Some(path) => RadtTsCapabilityStatus {
            available: true,
            executable: Some(path.display().to_string()),
            detail: "Local RADTTS voice generation is available on this computer.".to_string(),
        },
        None => RadtTsCapabilityStatus {
            available: false,
            executable: None,
            detail: "Install RADTTS locally or set RADSUITE_RADTTS_CLI to its executable."
                .to_string(),
        },
    }
}

fn find_on_path(name: &str) -> Vec<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use radsuite_core::ProjectId;

    use super::{
        RadtTsChunkMode, RadtTsOutputFormat, RadtTsQuality, RadtTsSynthesisRequest,
        RadtTsVoiceSource, StartRadtTsSynthesisRequest, build_synthesis_args, contained_file,
        parse_cli_result, remove_temp_text_files, shutdown_radt_ts_jobs, validate_output_name,
        validate_reference_audio,
    };
    use crate::state::DesktopState;

    #[test]
    fn rejects_path_like_output_names() {
        for value in ["", ".", "..", "../audio", "folder/audio", "/tmp/audio"] {
            assert!(validate_output_name(value).is_err(), "accepted {value:?}");
        }
        assert!(validate_output_name("lecture_v2").is_ok());
        assert!(validate_output_name("lecture-v2.wav").is_ok());
    }

    #[test]
    fn builds_reference_voice_cli_arguments_without_shell_interpolation() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let reference_audio_path = root.join("reference audio.wav");
        fs::write(&reference_audio_path, [0_u8; 8]).expect("create test audio");
        let request = RadtTsSynthesisRequest {
            project_id: ProjectId::new(),
            text: "A short script.".to_string(),
            voice_source: RadtTsVoiceSource::Reference,
            reference_audio_path: Some(reference_audio_path.clone()),
            reference_text: Some("Reference voice transcript.".to_string()),
            built_in_speaker: None,
            built_in_instruct: None,
            quality: RadtTsQuality::High,
            chunk_mode: RadtTsChunkMode::Single,
            pause_min_seconds: 0.25,
            pause_max_seconds: 0.75,
            pause_seed: Some(42),
            max_new_tokens: 1200,
            output_format: RadtTsOutputFormat::Wav,
            output_name: "intro_v2".to_string(),
            acknowledge_voice_clone: true,
        };

        let args = build_synthesis_args(
            &request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            Some(PathBuf::from("/tmp/reference.txt")),
        )
        .expect("valid request should build");
        assert_eq!(args[0], "--projects-root");
        assert!(args.contains(&"synthesize".to_string()));
        assert!(args.contains(&"--text-file".to_string()));
        assert!(args.contains(&"--reference-audio".to_string()));
        assert!(args.contains(&"--reference-text-file".to_string()));
        assert!(args.contains(&"/tmp/reference.txt".to_string()));
        assert!(
            args.contains(
                &reference_audio_path
                    .canonicalize()
                    .unwrap()
                    .display()
                    .to_string()
            )
        );
        assert!(args.contains(&"--mode".to_string()));
        assert!(args.contains(&"quality".to_string()));
        assert!(args.contains(&"--max-new-tokens".to_string()));
        assert!(args.contains(&"1200".to_string()));
        assert!(args.contains(&"--pause-seed".to_string()));
        assert!(args.contains(&"42".to_string()));
        assert!(args.contains(&"--ack-voice-clone".to_string()));
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn omits_reference_transcript_cli_arguments_when_not_supplied() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let reference_audio_path = root.join("reference.wav");
        fs::write(&reference_audio_path, [0_u8; 8]).expect("create test audio");
        let request = RadtTsSynthesisRequest {
            project_id: ProjectId::new(),
            text: "A short script.".to_string(),
            voice_source: RadtTsVoiceSource::Reference,
            reference_audio_path: Some(reference_audio_path),
            reference_text: None,
            built_in_speaker: None,
            built_in_instruct: None,
            quality: RadtTsQuality::Fast,
            chunk_mode: RadtTsChunkMode::Sentence,
            pause_min_seconds: 0.25,
            pause_max_seconds: 0.75,
            pause_seed: None,
            max_new_tokens: 1200,
            output_format: RadtTsOutputFormat::Mp3,
            output_name: "intro".to_string(),
            acknowledge_voice_clone: true,
        };

        let args = build_synthesis_args(
            &request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect("valid request should build");
        assert!(!args.contains(&"--reference-text-file".to_string()));
        assert!(!args.contains(&"--pause-seed".to_string()));
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn builds_builtin_voice_cli_arguments_without_reference_audio() {
        let request = RadtTsSynthesisRequest {
            project_id: ProjectId::new(),
            text: "A short script.".to_string(),
            voice_source: RadtTsVoiceSource::Builtin,
            reference_audio_path: None,
            reference_text: None,
            built_in_speaker: Some("Vivian".to_string()),
            built_in_instruct: Some("Warm and clear".to_string()),
            quality: RadtTsQuality::Fast,
            chunk_mode: RadtTsChunkMode::Sentence,
            pause_min_seconds: 0.25,
            pause_max_seconds: 0.75,
            pause_seed: None,
            max_new_tokens: 1200,
            output_format: RadtTsOutputFormat::Mp3,
            output_name: "intro".to_string(),
            acknowledge_voice_clone: false,
        };

        let args = build_synthesis_args(
            &request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect("valid built-in request should build");

        assert!(
            args.windows(2)
                .any(|pair| pair == ["--voice-source", "builtin"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--built-in-speaker", "Vivian"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--built-in-instruct", "Warm and clear"])
        );
        assert!(!args.contains(&"--reference-audio".to_string()));
        assert!(args.contains(&"fast".to_string()));
    }

    #[test]
    fn rejects_builtin_voice_requests_without_a_speaker() {
        let request = RadtTsSynthesisRequest {
            project_id: ProjectId::new(),
            text: "A short script.".to_string(),
            voice_source: RadtTsVoiceSource::Builtin,
            reference_audio_path: None,
            reference_text: None,
            built_in_speaker: None,
            built_in_instruct: None,
            quality: RadtTsQuality::High,
            chunk_mode: RadtTsChunkMode::Single,
            pause_min_seconds: 0.25,
            pause_max_seconds: 0.75,
            pause_seed: None,
            max_new_tokens: 1200,
            output_format: RadtTsOutputFormat::Mp3,
            output_name: "intro".to_string(),
            acknowledge_voice_clone: false,
        };

        let error = build_synthesis_args(
            &request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect_err("a built-in speaker is required");
        assert!(error.to_string().contains("built-in speaker"));
    }

    #[test]
    fn rejects_generation_budgets_outside_the_supported_range() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let reference_audio_path = root.join("reference.wav");
        fs::write(&reference_audio_path, [0_u8; 8]).expect("create test audio");
        let request = RadtTsSynthesisRequest {
            project_id: ProjectId::new(),
            text: "A short script.".to_string(),
            voice_source: RadtTsVoiceSource::Reference,
            reference_audio_path: Some(reference_audio_path),
            reference_text: None,
            built_in_speaker: None,
            built_in_instruct: None,
            quality: RadtTsQuality::Fast,
            chunk_mode: RadtTsChunkMode::Sentence,
            pause_min_seconds: 0.25,
            pause_max_seconds: 0.75,
            pause_seed: None,
            max_new_tokens: 63,
            output_format: RadtTsOutputFormat::Mp3,
            output_name: "intro".to_string(),
            acknowledge_voice_clone: true,
        };

        let error = build_synthesis_args(
            &request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect_err("too-small generation budget should be rejected");
        assert!(error.to_string().contains("between 64 and 8192"));

        let lower_request = RadtTsSynthesisRequest {
            max_new_tokens: 64,
            ..request.clone()
        };
        let lower_args = build_synthesis_args(
            &lower_request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect("the lower generation budget boundary should be accepted");
        assert!(lower_args.contains(&"64".to_string()));

        let upper_request = RadtTsSynthesisRequest {
            max_new_tokens: 8192,
            ..request.clone()
        };
        let upper_args = build_synthesis_args(
            &upper_request,
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect("the upper generation budget boundary should be accepted");
        assert!(upper_args.contains(&"8192".to_string()));

        let error = build_synthesis_args(
            &RadtTsSynthesisRequest {
                max_new_tokens: 8193,
                ..request
            },
            PathBuf::from("/tmp/project"),
            PathBuf::from("/tmp/script.txt"),
            None,
        )
        .expect_err("too-large generation budget should be rejected");
        assert!(error.to_string().contains("between 64 and 8192"));
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn accepts_legacy_synthesis_requests_without_reference_text() {
        let request: StartRadtTsSynthesisRequest = serde_json::from_str(
            r#"{
              "text": "A short script.",
              "reference_audio_path": "/tmp/reference.wav",
              "quality": "high",
              "chunk_mode": "sentence",
              "pause_min_seconds": 0.25,
              "pause_max_seconds": 0.75,
              "output_format": "mp3",
              "output_name": "intro",
              "acknowledge_voice_clone": true
            }"#,
        )
        .expect("legacy request should deserialize");
        assert_eq!(request.reference_text, None);
        assert_eq!(request.pause_seed, None);
        assert_eq!(request.max_new_tokens, 1200);
    }

    #[test]
    fn removes_script_and_optional_reference_text_files_together() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let script = root.join("script.txt");
        let reference = root.join("reference.txt");
        fs::write(&script, "script").expect("create script file");
        fs::write(&reference, "reference").expect("create reference file");

        remove_temp_text_files(&script, Some(&reference));

        assert!(!script.exists());
        assert!(!reference.exists());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[tokio::test]
    async fn shutdown_removes_registered_radt_ts_temp_files() {
        let state = DesktopState::for_tests();
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let script = root.join("script.txt");
        let reference = root.join("reference.txt");
        fs::write(&script, "script").expect("create script file");
        fs::write(&reference, "reference").expect("create reference file");
        state.radt_ts_lifecycle.lock().unwrap().temp_files.insert(
            "job-1".to_string(),
            (script.clone(), Some(reference.clone())),
        );

        shutdown_radt_ts_jobs(&state);

        assert!(!script.exists());
        assert!(!reference.exists());
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[tokio::test]
    async fn shutdown_marks_radt_ts_lifecycle_closed() {
        let state = DesktopState::for_tests();

        shutdown_radt_ts_jobs(&state);

        assert!(state.radt_ts_lifecycle.lock().unwrap().shutting_down);
    }

    #[test]
    fn parses_structured_cli_result() {
        let result = parse_cli_result(
            br#"{
              "job_id": "job-1",
              "status": "completed",
              "stage": "completed",
              "outputs": {
                "output_file": "/tmp/project/assets/generated_audio/intro.mp3",
                "metadata_path": "/tmp/project/manifests/intro.metadata.json"
              }
            }"#,
        )
        .expect("valid CLI JSON should parse");
        assert_eq!(result.job_id, "job-1");
        assert_eq!(result.status, "completed");
        assert_eq!(
            result.outputs.output_file.as_deref(),
            Some("/tmp/project/assets/generated_audio/intro.mp3")
        );
    }

    #[test]
    fn rejects_malformed_cli_result() {
        assert!(parse_cli_result(b"not json").is_err());
        assert!(parse_cli_result(br#"{"status":"completed"}"#).is_err());
    }

    #[test]
    fn only_accepts_generated_files_inside_project_root() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        let generated = root.join("assets/generated_audio");
        fs::create_dir_all(&generated).expect("create generated audio directory");
        let output = generated.join("lesson.mp3");
        fs::write(&output, [0_u8; 8]).expect("create generated audio");
        let project_root = root.canonicalize().expect("canonicalize project root");
        let outside = root
            .parent()
            .expect("temporary directory has a parent")
            .join(format!("radsuite-radt-ts-outside-{}", uuid::Uuid::new_v4()));
        fs::write(&outside, [0_u8; 8]).expect("create outside file");

        let accepted = contained_file(
            &project_root,
            PathBuf::from("assets/generated_audio/lesson.mp3").as_path(),
        )
        .expect("relative project output should be accepted");
        assert_eq!(
            accepted,
            output.canonicalize().expect("canonicalize output")
        );
        assert!(contained_file(&project_root, &outside).is_err());

        fs::remove_file(outside).expect("remove outside file");
        fs::remove_dir_all(root).expect("remove test directory");
    }

    #[test]
    fn validates_reference_audio_as_a_regular_supported_file() {
        let root = std::env::temp_dir().join(format!("radsuite-radt-ts-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).expect("create test directory");
        let audio = root.join("reference.wav");
        fs::write(&audio, [0_u8; 8]).expect("create test audio");
        assert!(validate_reference_audio(&audio).is_ok());
        let text = root.join("reference.txt");
        fs::write(&text, [0_u8; 8]).expect("create text file");
        assert!(validate_reference_audio(&text).is_err());
        fs::remove_dir_all(root).expect("remove test directory");
    }
}
