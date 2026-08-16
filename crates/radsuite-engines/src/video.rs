use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

use crate::{
    audio::resolve_tool,
    process::{ProcessError, run_process},
};

pub const VIDEO_WIDTH: u32 = 1280;
pub const VIDEO_HEIGHT: u32 = 720;
pub const VIDEO_FRAME_RATE: u32 = 30;
pub const WAVEFORM_WIDTH: u32 = 1280;
pub const WAVEFORM_HEIGHT: u32 = 220;
pub const WAVEFORM_Y: u32 = 500;
pub const VIDEO_AUDIO_TOLERANCE_SECONDS: f64 = 0.10;

const FILTER_GRAPH: &str = "[0:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720:(iw-1280)/2:(ih-720)/2[background];[background]drawbox=x=0:y=500:w=1280:h=220:color=black@0.70:t=fill[band];[1:a]showwaves=s=1280x220:mode=cline:rate=30:colors=white,format=rgba,colorkey=black:0.01:0.0[waveform];[band][waveform]overlay=0:500,fps=30,format=yuv420p[video]";

#[derive(Debug, Clone, PartialEq)]
pub struct VideoExportRequest {
    pub image_path: PathBuf,
    pub audio_path: PathBuf,
    pub output_path: PathBuf,
    pub audio_duration_seconds: f64,
}

impl VideoExportRequest {
    pub fn new(
        image_path: impl Into<PathBuf>,
        audio_path: impl Into<PathBuf>,
        output_path: impl Into<PathBuf>,
        audio_duration_seconds: f64,
    ) -> Self {
        Self {
            image_path: image_path.into(),
            audio_path: audio_path.into(),
            output_path: output_path.into(),
            audio_duration_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoExportResult {
    pub output_path: PathBuf,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoProbe {
    pub duration_seconds: f64,
}

#[derive(Debug, Error)]
pub enum VideoProbeError {
    #[error("ffprobe returned invalid JSON: {message}")]
    InvalidJson { message: String },
    #[error("video output must contain exactly one video and one audio stream")]
    InvalidStreamLayout,
    #[error("video stream does not satisfy the fixed MP4 contract: {message}")]
    InvalidVideoStream { message: String },
    #[error("audio stream does not satisfy the fixed MP4 contract: {message}")]
    InvalidAudioStream { message: String },
    #[error("video output duration is invalid")]
    InvalidDuration,
    #[error("video duration {actual:.3}s differs from audio duration {expected:.3}s")]
    DurationMismatch { expected: f64, actual: f64 },
}

#[derive(Debug, Error)]
pub enum VideoExportError {
    #[error("video image does not exist: {path}")]
    MissingImage { path: PathBuf },
    #[error("video audio does not exist: {path}")]
    MissingAudio { path: PathBuf },
    #[error("video image must be a PNG, JPEG, or WebP file: {path}")]
    UnsupportedImage { path: PathBuf },
    #[error("video output path has no parent directory: {path}")]
    MissingOutputParent { path: PathBuf },
    #[error("video audio duration is invalid: {duration}")]
    InvalidAudioDuration { duration: f64 },
    #[error("failed to prepare video output directory: {source}")]
    PrepareOutput {
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Probe(#[from] VideoProbeError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoExporter {
    ffmpeg_command: PathBuf,
    ffprobe_command: PathBuf,
}

impl Default for VideoExporter {
    fn default() -> Self {
        Self::from_commands(
            resolve_tool("RADSUITE_FFMPEG", "ffmpeg"),
            resolve_tool("RADSUITE_FFPROBE", "ffprobe"),
        )
    }
}

impl VideoExporter {
    pub fn from_commands(
        ffmpeg_command: impl Into<PathBuf>,
        ffprobe_command: impl Into<PathBuf>,
    ) -> Self {
        Self {
            ffmpeg_command: ffmpeg_command.into(),
            ffprobe_command: ffprobe_command.into(),
        }
    }

    pub fn ffmpeg_arguments(
        image_path: &Path,
        audio_path: &Path,
        output_path: &Path,
    ) -> Vec<OsString> {
        vec![
            OsString::from("-y"),
            OsString::from("-hide_banner"),
            OsString::from("-loglevel"),
            OsString::from("error"),
            OsString::from("-loop"),
            OsString::from("1"),
            OsString::from("-framerate"),
            OsString::from("30"),
            OsString::from("-i"),
            image_path.as_os_str().to_owned(),
            OsString::from("-i"),
            audio_path.as_os_str().to_owned(),
            OsString::from("-filter_complex"),
            OsString::from(FILTER_GRAPH),
            OsString::from("-map"),
            OsString::from("[video]"),
            OsString::from("-map"),
            OsString::from("1:a:0"),
            OsString::from("-c:v"),
            OsString::from("libx264"),
            OsString::from("-crf"),
            OsString::from("23"),
            OsString::from("-pix_fmt"),
            OsString::from("yuv420p"),
            OsString::from("-r"),
            OsString::from("30"),
            OsString::from("-c:a"),
            OsString::from("aac"),
            OsString::from("-b:a"),
            OsString::from("192k"),
            OsString::from("-shortest"),
            OsString::from("-movflags"),
            OsString::from("+faststart"),
            OsString::from("-progress"),
            OsString::from("pipe:1"),
            OsString::from("-nostats"),
            output_path.as_os_str().to_owned(),
        ]
    }

    pub fn filter_graph() -> &'static str {
        FILTER_GRAPH
    }

    pub fn export(
        &self,
        request: VideoExportRequest,
    ) -> Result<VideoExportResult, VideoExportError> {
        self.export_with_callbacks(request, || false, |_| {})
    }

    pub fn export_with_callbacks<C, P>(
        &self,
        request: VideoExportRequest,
        mut is_cancelled: C,
        mut on_progress: P,
    ) -> Result<VideoExportResult, VideoExportError>
    where
        C: FnMut() -> bool,
        P: FnMut(f64),
    {
        validate_request(&request)?;
        let parent =
            request
                .output_path
                .parent()
                .ok_or_else(|| VideoExportError::MissingOutputParent {
                    path: request.output_path.clone(),
                })?;
        fs::create_dir_all(parent).map_err(|source| VideoExportError::PrepareOutput { source })?;

        let args = Self::ffmpeg_arguments(
            &request.image_path,
            &request.audio_path,
            &request.output_path,
        );
        let result = run_process(&self.ffmpeg_command, &args, &mut is_cancelled, |line| {
            if let Some(progress) = parse_progress_line(line, request.audio_duration_seconds) {
                on_progress(progress);
            }
        });
        if let Err(error) = result {
            let _ = fs::remove_file(&request.output_path);
            return Err(VideoExportError::Process(error));
        }

        let probe = match self.probe_with_callbacks(
            &request.output_path,
            request.audio_duration_seconds,
            &mut is_cancelled,
        ) {
            Ok(probe) => probe,
            Err(error) => {
                let _ = fs::remove_file(&request.output_path);
                return Err(error);
            }
        };
        Ok(VideoExportResult {
            output_path: request.output_path,
            duration_seconds: probe.duration_seconds,
        })
    }

    pub fn probe(
        &self,
        path: &Path,
        audio_duration_seconds: f64,
    ) -> Result<VideoProbe, VideoExportError> {
        self.probe_with_callbacks(path, audio_duration_seconds, || false)
    }

    pub fn probe_with_callbacks<C>(
        &self,
        path: &Path,
        audio_duration_seconds: f64,
        mut is_cancelled: C,
    ) -> Result<VideoProbe, VideoExportError>
    where
        C: FnMut() -> bool,
    {
        if !audio_duration_seconds.is_finite() || audio_duration_seconds <= 0.0 {
            return Err(VideoExportError::InvalidAudioDuration {
                duration: audio_duration_seconds,
            });
        }

        let args = vec![
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-print_format"),
            OsString::from("json"),
            OsString::from("-show_streams"),
            OsString::from("-show_format"),
            path.as_os_str().to_owned(),
        ];
        let output = run_process(&self.ffprobe_command, &args, &mut is_cancelled, |_| {})?;
        let json = String::from_utf8_lossy(&output.stdout);
        Ok(Self::validate_probe_json(&json, audio_duration_seconds)?)
    }

    pub fn validate_probe_json(
        json: &str,
        audio_duration_seconds: f64,
    ) -> Result<VideoProbe, VideoProbeError> {
        let document: ProbeDocument =
            serde_json::from_str(json).map_err(|error| VideoProbeError::InvalidJson {
                message: error.to_string(),
            })?;
        if !audio_duration_seconds.is_finite() || audio_duration_seconds <= 0.0 {
            return Err(VideoProbeError::InvalidDuration);
        }
        if document.streams.len() != 2 {
            return Err(VideoProbeError::InvalidStreamLayout);
        }

        let video = document
            .streams
            .iter()
            .filter(|stream| stream.codec_type.as_deref() == Some("video"))
            .collect::<Vec<_>>();
        let audio = document
            .streams
            .iter()
            .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
            .collect::<Vec<_>>();
        if video.len() != 1 || audio.len() != 1 {
            return Err(VideoProbeError::InvalidStreamLayout);
        }

        let video = video[0];
        if video.codec_name.as_deref() != Some("h264") {
            return Err(VideoProbeError::InvalidVideoStream {
                message: "codec must be h264".to_string(),
            });
        }
        if video.width != Some(VIDEO_WIDTH) || video.height != Some(VIDEO_HEIGHT) {
            return Err(VideoProbeError::InvalidVideoStream {
                message: "dimensions must be 1280x720".to_string(),
            });
        }
        if video.pix_fmt.as_deref() != Some("yuv420p") {
            return Err(VideoProbeError::InvalidVideoStream {
                message: "pixel format must be yuv420p".to_string(),
            });
        }
        if video.r_frame_rate.as_deref() != Some("30/1")
            || video.avg_frame_rate.as_deref() != Some("30/1")
        {
            return Err(VideoProbeError::InvalidVideoStream {
                message: "frame rates must both be 30/1".to_string(),
            });
        }
        if audio[0].codec_name.as_deref() != Some("aac") {
            return Err(VideoProbeError::InvalidAudioStream {
                message: "codec must be aac".to_string(),
            });
        }

        let duration =
            parse_duration(&document.format.duration).ok_or(VideoProbeError::InvalidDuration)?;
        if (duration - audio_duration_seconds).abs() > VIDEO_AUDIO_TOLERANCE_SECONDS {
            return Err(VideoProbeError::DurationMismatch {
                expected: audio_duration_seconds,
                actual: duration,
            });
        }
        Ok(VideoProbe {
            duration_seconds: duration,
        })
    }
}

pub fn parse_progress_line(line: &str, audio_duration_seconds: f64) -> Option<f64> {
    let value = line.strip_prefix("out_time_us=")?.parse::<i64>().ok()?;
    if value < 0 || !audio_duration_seconds.is_finite() || audio_duration_seconds <= 0.0 {
        return None;
    }
    Some((value as f64 / (audio_duration_seconds * 1_000_000.0)).clamp(0.0, 1.0))
}

fn validate_request(request: &VideoExportRequest) -> Result<(), VideoExportError> {
    if !request.image_path.is_file() {
        return Err(VideoExportError::MissingImage {
            path: request.image_path.clone(),
        });
    }
    if !request.audio_path.is_file() {
        return Err(VideoExportError::MissingAudio {
            path: request.audio_path.clone(),
        });
    }
    if !is_supported_image(&request.image_path) {
        return Err(VideoExportError::UnsupportedImage {
            path: request.image_path.clone(),
        });
    }
    if !request.audio_duration_seconds.is_finite() || request.audio_duration_seconds <= 0.0 {
        return Err(VideoExportError::InvalidAudioDuration {
            duration: request.audio_duration_seconds,
        });
    }
    Ok(())
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
        .unwrap_or(false)
}

fn parse_duration(value: &serde_json::Value) -> Option<f64> {
    let duration = match value {
        serde_json::Value::Number(number) => number.as_f64(),
        serde_json::Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }?;
    duration
        .is_finite()
        .then_some(duration)
        .filter(|value| *value > 0.0)
}

#[derive(Debug, Deserialize)]
struct ProbeDocument {
    #[serde(default)]
    streams: Vec<ProbeStream>,
    format: ProbeFormat,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: serde_json::Value,
}
