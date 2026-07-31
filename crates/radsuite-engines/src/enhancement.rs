use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementModel {
    None,
    StudioV18,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancementProcessingRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancementProcessingResult {
    pub output_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum EnhancementProcessingError {
    #[error("enhancement input does not exist: {path}")]
    MissingInput { path: PathBuf },
    #[error("enhancement output path has no parent directory: {path}")]
    MissingOutputParent { path: PathBuf },
    #[error("enhancement helper is not available: {path}")]
    MissingCommand { path: PathBuf },
    #[error("could not start enhancement helper {command}: {source}")]
    StartCommand {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("enhancement helper failed: {message}")]
    CommandFailed { message: String },
    #[error("enhancement helper did not create the expected output: {path}")]
    MissingOutput { path: PathBuf },
    #[error("failed to prepare enhancement workspace")]
    PrepareWorkspace(#[source] std::io::Error),
    #[error("failed to copy enhanced output")]
    CopyOutput(#[source] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnhancementProcessor {
    command: PathBuf,
}

impl Default for EnhancementProcessor {
    fn default() -> Self {
        Self::from_command(resolve_command())
    }
}

impl EnhancementProcessor {
    pub fn from_command(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        self.command.is_file()
    }

    pub fn command_path(&self) -> &Path {
        &self.command
    }

    pub fn helper_arguments(
        &self,
        request: &EnhancementProcessingRequest,
    ) -> Result<Vec<OsString>, EnhancementProcessingError> {
        let input_dir = request
            .input_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let output_dir = request
            .output_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let suffix = audio_suffix(&request.input_path);
        Ok(vec![
            input_dir.into_os_string(),
            output_dir.into_os_string(),
            OsString::from("--suffix"),
            OsString::from(suffix),
            OsString::from("--device"),
            OsString::from(optimized_device()),
            OsString::from("--nfe"),
            OsString::from("32"),
            OsString::from("--lambd"),
            OsString::from("0.62"),
            OsString::from("--tau"),
            OsString::from("0.45"),
            OsString::from("--dereverb-method"),
            OsString::from("nara"),
            OsString::from("--nara-chunk-seconds"),
            OsString::from("8"),
            OsString::from("--nara-overlap-seconds"),
            OsString::from("1"),
            OsString::from("--nara-taps"),
            OsString::from("6"),
            OsString::from("--nara-delay"),
            OsString::from("2"),
            OsString::from("--nara-iterations"),
            OsString::from("1"),
            OsString::from("--nara-psd-context"),
            OsString::from("1"),
        ])
    }

    pub fn process(
        &self,
        request: EnhancementProcessingRequest,
    ) -> Result<EnhancementProcessingResult, EnhancementProcessingError> {
        if !request.input_path.is_file() {
            return Err(EnhancementProcessingError::MissingInput {
                path: request.input_path,
            });
        }
        let Some(parent) = request.output_path.parent() else {
            return Err(EnhancementProcessingError::MissingOutputParent {
                path: request.output_path,
            });
        };
        if !self.command.is_file() {
            return Err(EnhancementProcessingError::MissingCommand {
                path: self.command.clone(),
            });
        }
        fs::create_dir_all(parent).map_err(EnhancementProcessingError::PrepareWorkspace)?;

        let workspace = temporary_workspace();
        let input_dir = workspace.join("input");
        let output_dir = workspace.join("output");
        fs::create_dir_all(&input_dir).map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::create_dir_all(&output_dir).map_err(EnhancementProcessingError::PrepareWorkspace)?;

        let filename = format!("input{}", audio_suffix(&request.input_path));
        let staged_input = input_dir.join(&filename);
        let helper_output = output_dir.join(&filename);
        fs::copy(&request.input_path, &staged_input)
            .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        let staged_request = EnhancementProcessingRequest {
            input_path: staged_input,
            output_path: helper_output.clone(),
        };
        let args = self.helper_arguments(&staged_request)?;
        let thread_count = local_thread_count();
        let result = Command::new(&self.command)
            .args(&args)
            .env("OMP_NUM_THREADS", &thread_count)
            .env("MKL_NUM_THREADS", &thread_count)
            .env("OPENBLAS_NUM_THREADS", &thread_count)
            .env("VECLIB_MAXIMUM_THREADS", &thread_count)
            .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
            .output()
            .map_err(|source| EnhancementProcessingError::StartCommand {
                command: self.command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            let _ = fs::remove_dir_all(&workspace);
            return Err(EnhancementProcessingError::CommandFailed {
                message: command_output(&result.stdout, &result.stderr),
            });
        }
        if !helper_output.is_file() {
            let _ = fs::remove_dir_all(&workspace);
            return Err(EnhancementProcessingError::MissingOutput {
                path: helper_output,
            });
        }
        fs::copy(&helper_output, &request.output_path)
            .map_err(EnhancementProcessingError::CopyOutput)?;
        let _ = fs::remove_dir_all(workspace);
        Ok(EnhancementProcessingResult {
            output_path: request.output_path,
        })
    }
}

fn audio_suffix(path: &Path) -> String {
    path.extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| !extension.trim().is_empty())
        .map(|extension| format!(".{extension}"))
        .unwrap_or_else(|| ".wav".to_string())
}

fn temporary_workspace() -> PathBuf {
    let suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    std::env::temp_dir().join(format!(
        "radsuite-enhancement-{}-{suffix}",
        std::process::id()
    ))
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

fn optimized_device() -> String {
    std::env::var("RADSUITE_RADCAST_DEVICE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cpu".to_string())
}

fn local_thread_count() -> String {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
        .to_string()
}

fn resolve_command() -> PathBuf {
    if let Ok(value) = std::env::var("RADSUITE_STUDIO_COMMAND") {
        let path = PathBuf::from(value.trim());
        if !path.as_os_str().is_empty() {
            return path;
        }
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    for candidate in [
        home.join(".radcast/venv311/bin/radcast-studio-enhance"),
        home.join(".radcast/venv/bin/radcast-studio-enhance"),
        PathBuf::from("/opt/homebrew/bin/radcast-studio-enhance"),
        PathBuf::from("/usr/local/bin/radcast-studio-enhance"),
    ] {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from("radcast-studio-enhance")
}
