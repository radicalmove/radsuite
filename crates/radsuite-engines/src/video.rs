use std::{
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process,
    sync::atomic::{AtomicU64, Ordering},
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
pub const PRESENTER_WIDTH: u32 = 540;
pub const WAVEFORM_PANEL_WIDTH: u32 = 740;
pub const WAVEFORM_WIDTH: u32 = 660;
pub const WAVEFORM_HEIGHT: u32 = 240;
pub const WAVEFORM_X: u32 = 40;
pub const WAVEFORM_Y: u32 = 240;
pub const VIDEO_AUDIO_TOLERANCE_SECONDS: f64 = 0.10;

const FILTER_GRAPH: &str = "[0:v]scale=540:720:force_original_aspect_ratio=increase,crop=540:720:(iw-540)/2:(ih-720)/2[presenter];color=c=0x101214:s=740x720:r=30[panel];[1:a]showwaves=s=660x240:mode=cline:rate=30:colors=white:draw=full,format=rgba,colorkey=black:0.01:0.0[waveform];[panel][waveform]overlay=40:240[wave_panel];[presenter][wave_panel]hstack=inputs=2,fps=30,format=yuv420p[video]";
static PARTIAL_COUNTER: AtomicU64 = AtomicU64::new(0);

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupFailure {
    pub path: PathBuf,
    pub message: String,
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
    #[error("video output already exists: {path}")]
    OutputExists { path: PathBuf },
    #[error(transparent)]
    Process(#[from] ProcessError),
    #[error(transparent)]
    Probe(#[from] VideoProbeError),
    #[error("failed to promote partial video output: {source}")]
    PromoteOutput {
        #[source]
        source: std::io::Error,
    },
    #[error(
        "video export cleanup failed after promotion (partial retained at {partial_path}): {cleanup_failures:?}"
    )]
    CleanupAfterPromotion {
        partial_path: PathBuf,
        cleanup_failures: Vec<CleanupFailure>,
    },
    #[error("{cause}; cleanup failures: {cleanup_failures:?}")]
    Cleanup {
        #[source]
        cause: Box<VideoExportError>,
        cleanup_failures: Vec<CleanupFailure>,
    },
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
        if request.output_path.exists() {
            return Err(VideoExportError::OutputExists {
                path: request.output_path,
            });
        }
        let parent =
            request
                .output_path
                .parent()
                .ok_or_else(|| VideoExportError::MissingOutputParent {
                    path: request.output_path.clone(),
                })?;
        fs::create_dir_all(parent).map_err(|source| VideoExportError::PrepareOutput { source })?;
        let partial_path = unique_partial_path(&request.output_path);

        let args = Self::ffmpeg_arguments(&request.image_path, &request.audio_path, &partial_path);
        let result = run_process(&self.ffmpeg_command, &args, &mut is_cancelled, |line| {
            if let Some(progress) = parse_progress_line(line, request.audio_duration_seconds) {
                on_progress(progress);
            }
        });
        if let Err(error) = result {
            return Err(with_cleanup(
                VideoExportError::Process(error),
                &[partial_path.as_path()],
            ));
        }

        let probe = match self.probe_with_callbacks(
            &partial_path,
            request.audio_duration_seconds,
            &mut is_cancelled,
        ) {
            Ok(probe) => probe,
            Err(error) => {
                return Err(with_cleanup(error, &[partial_path.as_path()]));
            }
        };
        if let Err(error) = ensure_export_not_cancelled(&mut is_cancelled, &self.ffprobe_command) {
            return Err(with_cleanup(error, &[partial_path.as_path()]));
        }
        promote_partial_output(&partial_path, &request.output_path)?;
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

fn unique_partial_path(output_path: &Path) -> PathBuf {
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    loop {
        let counter = PARTIAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = output_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{stem}.partial-{}-{counter}.mp4", process::id()));
        if !path.exists() {
            return path;
        }
    }
}

fn unique_promotion_path(output_path: &Path) -> PathBuf {
    let filename = output_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("video.mp4");
    loop {
        let counter = PARTIAL_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = output_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!(
                ".{filename}.promotion-{}-{counter}.tmp",
                process::id()
            ));
        if !path.exists() {
            return path;
        }
    }
}

fn promote_partial_output(partial_path: &Path, output_path: &Path) -> Result<(), VideoExportError> {
    if output_path.exists() {
        return Err(with_cleanup(
            VideoExportError::OutputExists {
                path: output_path.to_path_buf(),
            },
            &[partial_path],
        ));
    }

    let promotion_path = stage_partial_output(partial_path, output_path)?;

    if output_path.exists() {
        return Err(with_cleanup(
            VideoExportError::OutputExists {
                path: output_path.to_path_buf(),
            },
            &[promotion_path.as_path(), partial_path],
        ));
    }

    // A hard link is an atomic, no-replace directory-entry operation on the same filesystem.
    // The staged copy remains available if this operation fails, so the canonical output is
    // never exposed until the complete file has been promoted.
    if let Err(source) = fs::hard_link(&promotion_path, output_path) {
        let cause = if source.kind() == io::ErrorKind::AlreadyExists {
            VideoExportError::OutputExists {
                path: output_path.to_path_buf(),
            }
        } else {
            VideoExportError::PromoteOutput {
                source: io::Error::new(
                    source.kind(),
                    format!(
                        "atomic no-replace promotion of partial video {} failed: {source}",
                        partial_path.display()
                    ),
                ),
            }
        };
        let paths = if matches!(&cause, VideoExportError::OutputExists { .. }) {
            vec![promotion_path.as_path(), partial_path]
        } else {
            vec![promotion_path.as_path()]
        };
        return Err(with_cleanup(cause, &paths));
    }

    finish_promotion(&promotion_path, partial_path, output_path)
}

fn stage_partial_output(
    partial_path: &Path,
    output_path: &Path,
) -> Result<PathBuf, VideoExportError> {
    let promotion_path = unique_promotion_path(output_path);
    let mut destination = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&promotion_path)
    {
        Ok(destination) => destination,
        Err(error) => {
            return Err(VideoExportError::PromoteOutput {
                source: io::Error::new(
                    error.kind(),
                    format!(
                        "failed to create promotion sibling for partial video {}: {error}",
                        partial_path.display()
                    ),
                ),
            });
        }
    };

    let copy_result = (|| -> io::Result<()> {
        let mut source = fs::File::open(partial_path)?;
        io::copy(&mut source, &mut destination)?;
        destination.sync_all()
    })();
    if let Err(error) = copy_result {
        drop(destination);
        let cause = VideoExportError::PromoteOutput {
            source: io::Error::new(
                error.kind(),
                format!(
                    "failed to stage partial video {} in {}: {error}",
                    partial_path.display(),
                    promotion_path.display()
                ),
            ),
        };
        return Err(with_cleanup(cause, &[promotion_path.as_path()]));
    }

    drop(destination);
    Ok(promotion_path)
}

fn finish_promotion(
    promotion_path: &Path,
    partial_path: &Path,
    output_path: &Path,
) -> Result<(), VideoExportError> {
    let mut cleanup_failures = Vec::new();
    for path in [promotion_path, partial_path] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => cleanup_failures.push(CleanupFailure {
                path: path.to_path_buf(),
                message: error.to_string(),
            }),
        }
    }
    if cleanup_failures.is_empty() {
        return Ok(());
    }

    // The canonical entry was created by this exporter. Roll it back on every post-promotion
    // cleanup failure so a retry cannot be blocked by an output that was reported as failed.
    match fs::remove_file(output_path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => cleanup_failures.push(CleanupFailure {
            path: output_path.to_path_buf(),
            message: format!("canonical output rollback failed: {error}"),
        }),
    }
    Err(VideoExportError::CleanupAfterPromotion {
        partial_path: partial_path.to_path_buf(),
        cleanup_failures,
    })
}

fn with_cleanup(cause: VideoExportError, paths: &[&Path]) -> VideoExportError {
    let cleanup_failures = cleanup_owned_paths(paths);
    if cleanup_failures.is_empty() {
        cause
    } else {
        VideoExportError::Cleanup {
            cause: Box::new(cause),
            cleanup_failures,
        }
    }
}

fn ensure_export_not_cancelled<C>(
    is_cancelled: &mut C,
    executable: &Path,
) -> Result<(), VideoExportError>
where
    C: FnMut() -> bool,
{
    if is_cancelled() {
        Err(VideoExportError::Process(ProcessError::Cancelled {
            executable: executable.to_path_buf(),
            diagnostics: Vec::new(),
        }))
    } else {
        Ok(())
    }
}

fn cleanup_owned_paths(paths: &[&Path]) -> Vec<CleanupFailure> {
    paths
        .iter()
        .filter_map(|path| match fs::remove_file(path) {
            Ok(()) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Some(CleanupFailure {
                path: (*path).to_path_buf(),
                message: error.to_string(),
            }),
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_probe_cancellation_gate_returns_a_cancelled_process_error() {
        let error = ensure_export_not_cancelled(&mut || true, Path::new("ffprobe"))
            .expect_err("cancelled export should not be promoted");

        assert!(matches!(
            error,
            VideoExportError::Process(ProcessError::Cancelled { .. })
        ));
    }

    #[test]
    fn no_replace_copy_preserves_partial_source_when_copy_fails() {
        let suffix = process::id();
        let directory = std::env::temp_dir().join(format!("radsuite-video-promotion-{suffix}"));
        fs::create_dir_all(&directory).expect("create promotion test directory");
        let partial = directory.join("output.partial.mp4");
        let output = directory.join("output.mp4");
        fs::create_dir(&partial).expect("create invalid partial source");

        let result = stage_partial_output(&partial, &output);

        assert!(matches!(
            result,
            Err(VideoExportError::PromoteOutput { .. })
        ));
        assert!(partial.exists());
        assert!(!output.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn promotion_staging_failure_reports_the_partial_source() {
        let suffix = process::id();
        let directory =
            std::env::temp_dir().join(format!("radsuite-video-promotion-report-{suffix}"));
        fs::create_dir_all(&directory).expect("create promotion test directory");
        let partial = directory.join("output.partial.mp4");
        let output = directory.join("output.mp4");
        fs::create_dir(&partial).expect("create invalid partial source");

        let error = promote_partial_output(&partial, &output).expect_err("staging should fail");

        assert!(
            error
                .to_string()
                .contains(partial.to_string_lossy().as_ref())
        );
        assert!(partial.exists());
        assert!(!output.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn post_promotion_cleanup_failure_rolls_back_canonical_output() {
        let suffix = process::id();
        let directory =
            std::env::temp_dir().join(format!("radsuite-video-promotion-rollback-{suffix}"));
        fs::create_dir_all(&directory).expect("create promotion test directory");
        let promotion = directory.join(".output.promotion.tmp");
        let partial = directory.join("output.partial.mp4");
        let output = directory.join("output.mp4");
        fs::write(&promotion, b"complete staged output").expect("write staged output");
        fs::create_dir(&partial).expect("create cleanup-failing partial path");
        fs::write(&output, b"complete canonical output").expect("write canonical output");

        let error = finish_promotion(&promotion, &partial, &output)
            .expect_err("partial cleanup should fail");

        assert!(matches!(
            error,
            VideoExportError::CleanupAfterPromotion { .. }
        ));
        assert!(!output.exists(), "canonical output must be rolled back");
        assert!(!promotion.exists());
        assert!(partial.exists());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn post_promotion_sibling_cleanup_failure_rolls_back_canonical_output() {
        let suffix = process::id();
        let directory =
            std::env::temp_dir().join(format!("radsuite-video-promotion-sibling-{suffix}"));
        fs::create_dir_all(&directory).expect("create promotion test directory");
        let promotion = directory.join(".output.promotion.tmp");
        let partial = directory.join("output.partial.mp4");
        let output = directory.join("output.mp4");
        fs::create_dir(&promotion).expect("create cleanup-failing promotion path");
        fs::write(&partial, b"complete partial output").expect("write partial output");
        fs::write(&output, b"complete canonical output").expect("write canonical output");

        let error = finish_promotion(&promotion, &partial, &output)
            .expect_err("promotion sibling cleanup should fail");

        assert!(matches!(
            error,
            VideoExportError::CleanupAfterPromotion { .. }
        ));
        assert!(!output.exists(), "canonical output must be rolled back");
        assert!(promotion.exists());
        assert!(!partial.exists());
        let _ = fs::remove_dir_all(directory);
    }
}
