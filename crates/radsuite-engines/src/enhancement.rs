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
    Resemble,
    DeepFilterNet,
    Studio,
    StudioV18,
}

impl EnhancementModel {
    pub const fn all() -> [Self; 5] {
        [
            Self::None,
            Self::Resemble,
            Self::DeepFilterNet,
            Self::Studio,
            Self::StudioV18,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "Standard cleanup",
            Self::Resemble => "Resemble Enhance",
            Self::DeepFilterNet => "DeepFilterNet3",
            Self::Studio => "Studio Cleanup",
            Self::StudioV18 => "RADcast Optimized",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::None => {
                "Keeps the original audio quality and applies only the selected cleanup options."
            }
            Self::Resemble => {
                "Strong speech enhancement that can sound more processed on some recordings."
            }
            Self::DeepFilterNet => {
                "Natural-sounding speech enhancement using the official DeepFilterNet3 model."
            }
            Self::Studio => {
                "Custom room-tail suppression followed by Resemble Enhance for a drier voice."
            }
            Self::StudioV18 => {
                "RADcast's tuned lecture-cleanup path with chunked dereverb and speech restoration."
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EnhancementQuality {
    Fast,
    #[default]
    Standard,
    High,
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
    resemble_command: PathBuf,
    deepfilternet_command: PathBuf,
    studio_command: PathBuf,
    optimized_command: PathBuf,
}

impl Default for EnhancementProcessor {
    fn default() -> Self {
        Self::from_commands(
            resolve_command(
                &["RADSUITE_RESEMBLE_COMMAND", "RADCAST_ENHANCE_COMMAND"],
                "radcast-enhance",
            ),
            resolve_command(
                &[
                    "RADSUITE_DEEPFILTERNET_COMMAND",
                    "RADCAST_DEEPFILTERNET_COMMAND",
                ],
                "deepFilter",
            ),
            resolve_command(
                &["RADSUITE_STUDIO_COMMAND", "RADCAST_STUDIO_COMMAND"],
                "radcast-studio-enhance",
            ),
            resolve_command(
                &["RADSUITE_STUDIO_COMMAND", "RADCAST_STUDIO_COMMAND"],
                "radcast-studio-enhance",
            ),
        )
    }
}

impl EnhancementProcessor {
    pub fn from_command(command: impl Into<PathBuf>) -> Self {
        let command = command.into();
        Self::from_commands(command.clone(), command.clone(), command.clone(), command)
    }

    pub fn from_commands(
        resemble_command: impl Into<PathBuf>,
        deepfilternet_command: impl Into<PathBuf>,
        studio_command: impl Into<PathBuf>,
        optimized_command: impl Into<PathBuf>,
    ) -> Self {
        Self {
            resemble_command: resemble_command.into(),
            deepfilternet_command: deepfilternet_command.into(),
            studio_command: studio_command.into(),
            optimized_command: optimized_command.into(),
        }
    }

    pub fn is_available(&self) -> bool {
        self.is_model_available(EnhancementModel::StudioV18)
    }

    pub fn is_model_available(&self, model: EnhancementModel) -> bool {
        match self.command_for_model(model) {
            None => true,
            Some(command) => command.is_file(),
        }
    }

    pub fn command_path(&self) -> &Path {
        &self.optimized_command
    }

    pub fn command_path_for(&self, model: EnhancementModel) -> Option<&Path> {
        self.command_for_model(model)
    }

    pub fn helper_arguments(
        &self,
        request: &EnhancementProcessingRequest,
    ) -> Result<Vec<OsString>, EnhancementProcessingError> {
        self.helper_arguments_with_quality(request, EnhancementQuality::High)
    }

    pub fn helper_arguments_with_quality(
        &self,
        request: &EnhancementProcessingRequest,
        quality: EnhancementQuality,
    ) -> Result<Vec<OsString>, EnhancementProcessingError> {
        self.helper_arguments_for_model(request, EnhancementModel::StudioV18, quality)
    }

    pub fn helper_arguments_for_model(
        &self,
        request: &EnhancementProcessingRequest,
        model: EnhancementModel,
        quality: EnhancementQuality,
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
        match model {
            EnhancementModel::None => Ok(Vec::new()),
            EnhancementModel::Resemble | EnhancementModel::Studio => Ok(vec![
                input_dir.into_os_string(),
                output_dir.into_os_string(),
                OsString::from("--suffix"),
                OsString::from(suffix),
                OsString::from("--device"),
                OsString::from(general_device()),
                OsString::from("--nfe"),
                OsString::from(quality.nfe()),
                OsString::from("--lambd"),
                OsString::from("0.7"),
                OsString::from("--tau"),
                OsString::from("0.5"),
            ]),
            EnhancementModel::DeepFilterNet => Ok(vec![
                OsString::from("--output-dir"),
                output_dir.into_os_string(),
                OsString::from("--model-base-dir"),
                OsString::from(deepfilternet_model()),
                OsString::from("--log-level"),
                OsString::from("info"),
                OsString::from("--no-suffix"),
                request.input_path.clone().into_os_string(),
            ]),
            EnhancementModel::StudioV18 => Ok(vec![
                input_dir.into_os_string(),
                output_dir.into_os_string(),
                OsString::from("--suffix"),
                OsString::from(suffix),
                OsString::from("--device"),
                OsString::from(optimized_device()),
                OsString::from("--nfe"),
                OsString::from(quality.nfe()),
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
            ]),
        }
    }

    pub fn process(
        &self,
        request: EnhancementProcessingRequest,
    ) -> Result<EnhancementProcessingResult, EnhancementProcessingError> {
        self.process_with_quality(request, EnhancementQuality::High)
    }

    pub fn process_with_quality(
        &self,
        request: EnhancementProcessingRequest,
        quality: EnhancementQuality,
    ) -> Result<EnhancementProcessingResult, EnhancementProcessingError> {
        self.process_model_with_quality(request, EnhancementModel::StudioV18, quality)
    }

    pub fn process_model_with_quality(
        &self,
        request: EnhancementProcessingRequest,
        model: EnhancementModel,
        quality: EnhancementQuality,
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
        let Some(command) = self.command_path_for(model) else {
            return Ok(EnhancementProcessingResult {
                output_path: request.output_path,
            });
        };
        if !command.is_file() {
            return Err(EnhancementProcessingError::MissingCommand {
                path: command.to_path_buf(),
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
        let args = self.helper_arguments_for_model(&staged_request, model, quality)?;
        let thread_count = local_thread_count();
        let result = Command::new(command)
            .args(&args)
            .env("OMP_NUM_THREADS", &thread_count)
            .env("MKL_NUM_THREADS", &thread_count)
            .env("OPENBLAS_NUM_THREADS", &thread_count)
            .env("VECLIB_MAXIMUM_THREADS", &thread_count)
            .env("PYTORCH_ENABLE_MPS_FALLBACK", "1")
            .output()
            .map_err(|source| EnhancementProcessingError::StartCommand {
                command: command.display().to_string(),
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

    fn command_for_model(&self, model: EnhancementModel) -> Option<&Path> {
        match model {
            EnhancementModel::None => None,
            EnhancementModel::Resemble => Some(&self.resemble_command),
            EnhancementModel::DeepFilterNet => Some(&self.deepfilternet_command),
            EnhancementModel::Studio => Some(&self.studio_command),
            EnhancementModel::StudioV18 => Some(&self.optimized_command),
        }
    }
}

impl EnhancementQuality {
    fn nfe(self) -> &'static str {
        match self {
            Self::Fast => "8",
            Self::Standard => "16",
            Self::High => "32",
        }
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

fn general_device() -> String {
    std::env::var("RADSUITE_RADCAST_DEVICE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cpu".to_string())
}

fn deepfilternet_model() -> String {
    std::env::var("RADSUITE_DEEPFILTERNET_MODEL")
        .or_else(|_| std::env::var("RADCAST_DEEPFILTERNET_MODEL"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "DeepFilterNet3".to_string())
}

fn local_thread_count() -> String {
    std::thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1)
        .to_string()
}

fn resolve_command(environment_variables: &[&str], command: &str) -> PathBuf {
    for environment_variable in environment_variables {
        if let Ok(value) = std::env::var(environment_variable) {
            let path = PathBuf::from(value.trim());
            if !path.as_os_str().is_empty() {
                return path;
            }
        }
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let candidates = match command {
        "radcast-enhance" => vec![
            home.join(".radcast/venv311/bin/radcast-enhance"),
            home.join(".radcast/venv/bin/radcast-enhance"),
            PathBuf::from("/opt/homebrew/bin/radcast-enhance"),
            PathBuf::from("/usr/local/bin/radcast-enhance"),
        ],
        "deepFilter" => vec![
            home.join(".radcast/venv311/bin/deepFilter"),
            home.join(".radcast/venv/bin/deepFilter"),
            PathBuf::from("/opt/homebrew/bin/deepFilter"),
            PathBuf::from("/usr/local/bin/deepFilter"),
        ],
        _ => vec![
            home.join(".radcast/venv311/bin/radcast-studio-enhance"),
            home.join(".radcast/venv/bin/radcast-studio-enhance"),
            PathBuf::from("/opt/homebrew/bin/radcast-studio-enhance"),
            PathBuf::from("/usr/local/bin/radcast-studio-enhance"),
        ],
    };
    for candidate in candidates {
        if candidate.is_file() {
            return candidate;
        }
    }
    PathBuf::from(command)
}
