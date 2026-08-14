use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::runtime::windows_ffmpeg_path;

const CLEANUP_FILTER: &str = "highpass=f=80,lowpass=f=12000,afftdn,loudnorm=I=-16:TP=-1.5:LRA=11";

pub const RADCAST_OPTIMIZED_POSTFILTER: &str = "highpass=f=65,equalizer=f=142:t=q:w=1.05:g=4.05,equalizer=f=200:t=q:w=1.0:g=1.75,equalizer=f=315:t=q:w=1.0:g=-0.55,equalizer=f=455:t=q:w=1.0:g=-0.2,equalizer=f=2350:t=q:w=1.0:g=-2.35,equalizer=f=3000:t=q:w=1.0:g=-1.70,equalizer=f=3850:t=q:w=1.0:g=-0.30,deesser=i=0.045:m=0.18:f=0.5:s=o,equalizer=f=5700:t=q:w=1.0:g=-1.40,equalizer=f=6400:t=q:w=1.0:g=-1.20,loudnorm=I=-20.75:TP=-1.5:LRA=8,lowpass=f=7550";
pub const RADCAST_NATURAL_POSTFILTER: &str = "highpass=f=65,equalizer=f=142:t=q:w=1.05:g=3.35,equalizer=f=200:t=q:w=1.0:g=1.4,equalizer=f=315:t=q:w=1.0:g=-0.4,equalizer=f=455:t=q:w=1.0:g=-0.1,equalizer=f=2350:t=q:w=1.0:g=-1.10,equalizer=f=3000:t=q:w=1.0:g=-0.60,equalizer=f=3850:t=q:w=1.0:g=-0.05,deesser=i=0.012:m=0.08:f=0.5:s=o,equalizer=f=5700:t=q:w=1.0:g=-0.45,equalizer=f=6400:t=q:w=1.0:g=-0.35,loudnorm=I=-20.75:TP=-1.5:LRA=8,lowpass=f=8200";
pub const RADCAST_NATURAL_PLUS_POSTFILTER: &str = "highpass=f=65,equalizer=f=142:t=q:w=1.05:g=3.15,equalizer=f=200:t=q:w=1.0:g=1.25,equalizer=f=315:t=q:w=1.0:g=-0.3,equalizer=f=455:t=q:w=1.0:g=-0.05,equalizer=f=2350:t=q:w=1.0:g=-0.70,equalizer=f=3000:t=q:w=1.0:g=-0.30,equalizer=f=3850:t=q:w=1.0:g=0,deesser=i=0.006:m=0.04:f=0.5:s=o,equalizer=f=5700:t=q:w=1.0:g=-0.20,equalizer=f=6400:t=q:w=1.0:g=-0.15,loudnorm=I=-20.75:TP=-1.5:LRA=8,lowpass=f=8800";
pub const RADCAST_NATURAL_DOUBLE_PLUS_POSTFILTER: &str = "highpass=f=70,equalizer=f=130:t=q:w=1.0:g=2.2,equalizer=f=280:t=q:w=1.1:g=-1.2,equalizer=f=520:t=q:w=1.0:g=-0.5,equalizer=f=1650:t=q:w=1.0:g=0.7,equalizer=f=3000:t=q:w=1.0:g=0.5,acompressor=threshold=0.12:ratio=1.55:attack=16:release=190:makeup=1.45,loudnorm=I=-20.75:TP=-1.5:LRA=8,lowpass=f=10000";

pub const RADCAST_STANDARD_PREFILTER: &str = "highpass=f=85,agate=threshold=0.027:ratio=1.26:attack=8:release=280:range=0.56:knee=4,afftdn=nr=4:nf=-48:tn=1,equalizer=f=380:t=q:w=1.0:g=-1.0,equalizer=f=6800:t=q:w=1.2:g=-1.3";
pub const RADCAST_STANDARD_POSTFILTER: &str = "highpass=f=65,equalizer=f=150:t=q:w=1.05:g=2.8,equalizer=f=320:t=q:w=1.0:g=-1.2,equalizer=f=520:t=q:w=1.0:g=-0.9,equalizer=f=2800:t=q:w=1.0:g=0.4,deesser=i=0.06:m=0.25:f=0.5:s=o,loudnorm=I=-20.5:TP=-1.5:LRA=8,equalizer=f=6200:t=q:w=1.2:g=-2.5,lowpass=f=6800";
pub const RADCAST_STUDIO_POSTFILTER: &str = "highpass=f=65,equalizer=f=150:t=q:w=1.05:g=2.2,equalizer=f=320:t=q:w=1.0:g=-1.0,equalizer=f=520:t=q:w=1.0:g=-0.8,equalizer=f=2600:t=q:w=1.0:g=-2.0,equalizer=f=3400:t=q:w=1.0:g=-1.4,deesser=i=0.03:m=0.18:f=0.5:s=o,loudnorm=I=-20.5:TP=-1.5:LRA=8,equalizer=f=7000:t=q:w=1.0:g=0.8,lowpass=f=9500";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioOutputFormat {
    Mp3,
    Wav,
}

impl AudioOutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTimeInterval {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioProcessingRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub output_format: AudioOutputFormat,
    pub clip_start_seconds: Option<f64>,
    pub clip_end_seconds: Option<f64>,
    pub cleanup_enabled: bool,
    pub max_silence_seconds: Option<f64>,
    pub remove_intervals: Vec<AudioTimeInterval>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioProcessingResult {
    pub output_path: PathBuf,
    pub duration_seconds: f64,
    pub output_format: AudioOutputFormat,
}

#[derive(Debug, Error)]
pub enum AudioProcessingError {
    #[error("audio input does not exist: {path}")]
    MissingInput { path: PathBuf },
    #[error("audio output path has no parent directory: {path}")]
    MissingOutputParent { path: PathBuf },
    #[error("invalid audio clip range: start {start:?}, end {end:?}")]
    InvalidClipRange {
        start: Option<f64>,
        end: Option<f64>,
    },
    #[error("invalid maximum silence duration: {seconds:?}")]
    InvalidMaxSilence { seconds: Option<f64> },
    #[error("invalid audio removal interval: start {start_seconds}, end {end_seconds}")]
    InvalidRemovalInterval {
        start_seconds: f64,
        end_seconds: f64,
    },
    #[error("could not start {command}: {source}")]
    StartCommand {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{command} failed: {message}")]
    CommandFailed { command: String, message: String },
    #[error("{command} returned an invalid duration")]
    InvalidDuration { command: String },
    #[error("failed to prepare audio output directory")]
    PrepareOutput(#[source] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProcessor {
    ffmpeg_command: PathBuf,
    ffprobe_command: PathBuf,
}

impl Default for AudioProcessor {
    fn default() -> Self {
        Self::from_commands(
            resolve_tool("RADSUITE_FFMPEG", "ffmpeg"),
            resolve_tool("RADSUITE_FFPROBE", "ffprobe"),
        )
    }
}

impl AudioProcessor {
    pub fn from_commands(
        ffmpeg_command: impl Into<PathBuf>,
        ffprobe_command: impl Into<PathBuf>,
    ) -> Self {
        Self {
            ffmpeg_command: ffmpeg_command.into(),
            ffprobe_command: ffprobe_command.into(),
        }
    }

    pub fn validate_request(request: &AudioProcessingRequest) -> Result<(), AudioProcessingError> {
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

        let valid_max_silence = request
            .max_silence_seconds
            .is_none_or(|value| value.is_finite() && value >= 0.0);

        if !valid_start || !valid_end || !valid_order {
            return Err(AudioProcessingError::InvalidClipRange {
                start: request.clip_start_seconds,
                end: request.clip_end_seconds,
            });
        }
        if !valid_max_silence {
            return Err(AudioProcessingError::InvalidMaxSilence {
                seconds: request.max_silence_seconds,
            });
        }

        let mut previous_end = 0.0;
        for interval in &request.remove_intervals {
            let valid_interval = interval.start_seconds.is_finite()
                && interval.end_seconds.is_finite()
                && interval.start_seconds >= 0.0
                && interval.end_seconds > interval.start_seconds
                && interval.start_seconds >= previous_end;
            if !valid_interval {
                return Err(AudioProcessingError::InvalidRemovalInterval {
                    start_seconds: interval.start_seconds,
                    end_seconds: interval.end_seconds,
                });
            }
            previous_end = interval.end_seconds;
        }

        Ok(())
    }

    pub fn ffmpeg_arguments(
        request: &AudioProcessingRequest,
    ) -> Result<Vec<OsString>, AudioProcessingError> {
        Self::ffmpeg_arguments_with_additional_filter(request, None)
    }

    pub fn ffmpeg_arguments_with_additional_filter(
        request: &AudioProcessingRequest,
        additional_filter: Option<&str>,
    ) -> Result<Vec<OsString>, AudioProcessingError> {
        Self::validate_request(request)?;

        let mut args = vec![
            OsString::from("-y"),
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
        ];

        args.push(OsString::from("-i"));
        args.push(request.input_path.clone().into_os_string());

        // Keep seeking after input so trim points are sample-accurate for encoded and WAV audio.
        if let Some(start) = request.clip_start_seconds {
            args.push(OsString::from("-ss"));
            args.push(OsString::from(format!("{start:.3}")));
        }

        if let Some(end) = request.clip_end_seconds {
            let start = request.clip_start_seconds.unwrap_or(0.0);
            args.push(OsString::from("-t"));
            args.push(OsString::from(format!("{:.3}", end - start)));
        }

        let mut filters = Vec::new();
        if let Some(additional_filter) = additional_filter
            .map(str::trim)
            .filter(|filter| !filter.is_empty())
        {
            filters.push(additional_filter.to_string());
        }
        if request.cleanup_enabled {
            filters.push(CLEANUP_FILTER.to_string());
        }
        if let Some(max_silence_seconds) = request.max_silence_seconds {
            filters.push(format!(
                "silenceremove=stop_periods=-1:stop_duration={max_silence_seconds:.3}:stop_threshold=-50dB:stop_silence={max_silence_seconds:.3}"
            ));
        }

        if !request.remove_intervals.is_empty() {
            let mut graph = Self::removal_filter_graph(&request.remove_intervals);
            let output_label = if filters.is_empty() {
                "[outa]"
            } else {
                graph.push_str(&format!(";[outa]{}[outa_filtered]", filters.join(",")));
                "[outa_filtered]"
            };
            args.push(OsString::from("-filter_complex"));
            args.push(OsString::from(graph));
            args.extend([OsString::from("-map"), OsString::from(output_label)]);
        } else if !filters.is_empty() {
            args.push(OsString::from("-af"));
            args.push(OsString::from(filters.join(",")));
        }

        match request.output_format {
            AudioOutputFormat::Mp3 => {
                args.extend([
                    OsString::from("-codec:a"),
                    OsString::from("libmp3lame"),
                    OsString::from("-q:a"),
                    OsString::from("2"),
                ]);
            }
            AudioOutputFormat::Wav => {
                args.extend([OsString::from("-codec:a"), OsString::from("pcm_s16le")]);
            }
        }

        args.push(request.output_path.clone().into_os_string());
        Ok(args)
    }

    pub fn removal_filter_graph(intervals: &[AudioTimeInterval]) -> String {
        let mut graph = Vec::new();
        let mut previous_end = 0.0;
        let mut segment_count = 0;

        for interval in intervals {
            if interval.start_seconds > previous_end {
                graph.push(format!(
                    "[0:a]atrim=start={previous_end:.3}:end={start:.3},asetpts=PTS-STARTPTS[a{segment_count}]",
                    start = interval.start_seconds,
                ));
                segment_count += 1;
            }
            previous_end = interval.end_seconds;
        }

        graph.push(format!(
            "[0:a]atrim=start={previous_end:.3},asetpts=PTS-STARTPTS[a{segment_count}]"
        ));
        segment_count += 1;
        let inputs = (0..segment_count)
            .map(|index| format!("[a{index}]"))
            .collect::<String>();
        graph.push(format!("{inputs}concat=n={segment_count}:v=0:a=1[outa]"));
        graph.join(";")
    }

    pub fn process(
        &self,
        request: AudioProcessingRequest,
    ) -> Result<AudioProcessingResult, AudioProcessingError> {
        self.process_with_additional_filter(request, None)
    }

    pub fn process_with_additional_filter(
        &self,
        request: AudioProcessingRequest,
        additional_filter: Option<&str>,
    ) -> Result<AudioProcessingResult, AudioProcessingError> {
        Self::validate_request(&request)?;
        if !request.input_path.is_file() {
            return Err(AudioProcessingError::MissingInput {
                path: request.input_path,
            });
        }

        let Some(parent) = request.output_path.parent() else {
            return Err(AudioProcessingError::MissingOutputParent {
                path: request.output_path,
            });
        };
        fs::create_dir_all(parent).map_err(AudioProcessingError::PrepareOutput)?;

        let args = Self::ffmpeg_arguments_with_additional_filter(&request, additional_filter)?;
        let result = Command::new(&self.ffmpeg_command)
            .args(&args)
            .output()
            .map_err(|source| AudioProcessingError::StartCommand {
                command: self.ffmpeg_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            return Err(AudioProcessingError::CommandFailed {
                command: self.ffmpeg_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        let duration_seconds = self.probe_duration(&request.output_path)?;
        Ok(AudioProcessingResult {
            output_path: request.output_path,
            duration_seconds,
            output_format: request.output_format,
        })
    }

    pub fn probe_duration(&self, path: &Path) -> Result<f64, AudioProcessingError> {
        let result = Command::new(&self.ffprobe_command)
            .args([
                OsString::from("-v"),
                OsString::from("error"),
                OsString::from("-show_entries"),
                OsString::from("format=duration"),
                OsString::from("-of"),
                OsString::from("default=noprint_wrappers=1:nokey=1"),
                path.to_path_buf().into_os_string(),
            ])
            .output()
            .map_err(|source| AudioProcessingError::StartCommand {
                command: self.ffprobe_command.display().to_string(),
                source,
            })?;
        if !result.status.success() {
            return Err(AudioProcessingError::CommandFailed {
                command: self.ffprobe_command.display().to_string(),
                message: command_output(&result.stdout, &result.stderr),
            });
        }

        let raw = String::from_utf8_lossy(&result.stdout).trim().to_string();
        let duration_seconds = raw
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| AudioProcessingError::InvalidDuration {
                command: self.ffprobe_command.display().to_string(),
            })?;
        Ok(duration_seconds)
    }
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

    if let Some(path) = windows_ffmpeg_path(
        std::env::var_os("LOCALAPPDATA").as_deref().map(Path::new),
        command,
        cfg!(windows),
    ) {
        return path;
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
