use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::process::{ProcessError, run_process};
use crate::runtime::windows_ffmpeg_path;

const CLEANUP_FILTER: &str = "highpass=f=80,lowpass=f=12000,afftdn,loudnorm=I=-16:TP=-1.5:LRA=11";
const CUT_SAMPLE_RATE: f64 = 48000.0;
const CUT_CROSSFADE_SAMPLES: u64 = 576; // 12 ms at the working sample rate.

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

fn cut_overlap(accumulated_samples: u64, next_samples: u64) -> u64 {
    CUT_CROSSFADE_SAMPLES
        .min(accumulated_samples)
        .min(next_samples)
}

fn retained_sample_ranges(
    intervals: &[AudioTimeInterval],
    duration: Option<f64>,
) -> Vec<(u64, Option<u64>)> {
    let samples = |seconds: f64| (seconds * CUT_SAMPLE_RATE).round() as u64;
    let end = duration.map(samples);
    let mut cursor = 0;
    let mut ranges = Vec::new();
    for interval in intervals {
        let start = end.map_or_else(
            || samples(interval.start_seconds),
            |end| samples(interval.start_seconds).min(end),
        );
        let stop = end.map_or_else(
            || samples(interval.end_seconds),
            |end| samples(interval.end_seconds).min(end),
        );
        if start > cursor {
            ranges.push((cursor, Some(start)));
        }
        cursor = cursor.max(stop);
    }
    if end.is_none_or(|end| end > cursor) {
        ranges.push((cursor, end));
    }
    ranges
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectedSilence {
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
    #[error("{command} was cancelled")]
    Cancelled { command: String },
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
        if request.remove_intervals.is_empty()
            && let Some(start) = request.clip_start_seconds
        {
            args.push(OsString::from("-ss"));
            args.push(OsString::from(format!("{start:.3}")));
        }

        if request.remove_intervals.is_empty()
            && let Some(end) = request.clip_end_seconds
        {
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
            let mut graph = Self::removal_filter_graph_for_clip(
                &request.remove_intervals,
                request.clip_start_seconds.unwrap_or(0.0),
                request.clip_end_seconds,
            );
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

    /// Studio is decoded/resampled once to 48 kHz floating-point PCM. Final MP3
    /// encoding is allowed only when explicitly exporting the mastered WAV.
    pub fn studio_ffmpeg_arguments(
        request: &AudioProcessingRequest,
        additional_filter: Option<&str>,
    ) -> Result<Vec<OsString>, AudioProcessingError> {
        let mut args = Self::ffmpeg_arguments_with_additional_filter(request, additional_filter)?;
        if request.output_format == AudioOutputFormat::Wav {
            for arg in &mut args {
                if arg == "pcm_s16le" {
                    *arg = OsString::from("pcm_f32le");
                }
            }
        }
        let output = args.pop().expect("validated output argument");
        args.extend([OsString::from("-ar"), OsString::from("48000"), output]);
        Ok(args)
    }

    pub fn process_studio_with_additional_filter(
        &self,
        request: AudioProcessingRequest,
        additional_filter: Option<&str>,
    ) -> Result<AudioProcessingResult, AudioProcessingError> {
        self.process_studio_with_additional_filter_with_callbacks(
            request,
            additional_filter,
            || false,
            |_| {},
        )
    }

    pub fn removal_filter_graph(intervals: &[AudioTimeInterval]) -> String {
        Self::removal_filter_graph_for_clip(intervals, 0.0, None)
    }

    /// Total timeline reduction, including bounded crossfades, for export QA.
    pub fn removal_duration_seconds(intervals: &[AudioTimeInterval], duration: f64) -> f64 {
        if intervals.is_empty() {
            return 0.0;
        }
        let ranges = retained_sample_ranges(intervals, Some(duration));
        let mut output_samples = 0;
        for (index, (start, end)) in ranges.iter().enumerate() {
            let samples = end.expect("bounded retained range") - start;
            output_samples += samples
                - if index == 0 {
                    0
                } else {
                    cut_overlap(output_samples, samples)
                };
        }
        ((duration * CUT_SAMPLE_RATE).round() as u64).saturating_sub(output_samples) as f64
            / CUT_SAMPLE_RATE
    }

    fn removal_filter_graph_for_clip(
        intervals: &[AudioTimeInterval],
        clip_start: f64,
        clip_end: Option<f64>,
    ) -> String {
        let duration = clip_end.map(|end| (end - clip_start).max(0.0));
        let ranges = retained_sample_ranges(intervals, duration);
        if ranges.is_empty() {
            return "[0:a]atrim=start=0:end=0,asetpts=PTS-STARTPTS[outa]".to_string();
        }
        let mut graph = Vec::new();
        // Sample indices are relative to the selected clip, as in Python's waveform splicer.
        graph.push(format!("[0:a]aresample=48000,atrim=start_sample={start}{end},asetpts=PTS-STARTPTS,asplit={count}{labels}",
            start = (clip_start * CUT_SAMPLE_RATE).round() as u64,
            end = clip_end.map(|end| format!(":end_sample={}", (end * CUT_SAMPLE_RATE).round() as u64)).unwrap_or_default(),
            count = ranges.len(),
            labels = (0..ranges.len()).map(|i| format!("[s{i}]")).collect::<String>(),
        ));
        let mut lengths = Vec::new();
        for (index, (first, last)) in ranges.iter().enumerate() {
            graph.push(format!(
                "[s{index}]atrim=start_sample={first}{end},asetpts=PTS-STARTPTS[a{index}]",
                end = last
                    .map(|last| format!(":end_sample={last}"))
                    .unwrap_or_default()
            ));
            lengths.push(last.map(|last| last.saturating_sub(*first)));
        }
        let mut accumulated = lengths[0].unwrap_or(CUT_CROSSFADE_SAMPLES);
        let mut label = "a0".to_string();
        if ranges.len() == 1 {
            graph.push("[a0]anull[outa]".to_string());
        }
        for (index, length) in lengths.iter().enumerate().skip(1) {
            let overlap = cut_overlap(accumulated, length.unwrap_or(CUT_CROSSFADE_SAMPLES));
            let output = if index + 1 == ranges.len() {
                "outa".to_string()
            } else {
                format!("joined{index}")
            };
            graph.push(format!(
                "[{label}][a{index}]acrossfade=ns={overlap}:c1=tri:c2=tri[{output}]"
            ));
            accumulated = accumulated + length.unwrap_or(CUT_CROSSFADE_SAMPLES) - overlap;
            label = output;
        }
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
        self.process_with_additional_filter_with_callbacks(
            request,
            additional_filter,
            || false,
            |_| {},
        )
    }

    pub fn process_with_callbacks<C, P>(
        &self,
        request: AudioProcessingRequest,
        is_cancelled: C,
        on_progress_line: P,
    ) -> Result<AudioProcessingResult, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
        self.process_with_additional_filter_with_callbacks(
            request,
            None,
            is_cancelled,
            on_progress_line,
        )
    }

    pub fn process_with_additional_filter_with_callbacks<C, P>(
        &self,
        request: AudioProcessingRequest,
        additional_filter: Option<&str>,
        is_cancelled: C,
        on_progress_line: P,
    ) -> Result<AudioProcessingResult, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
        self.process_with_format_with_callbacks(
            request,
            additional_filter,
            false,
            is_cancelled,
            on_progress_line,
        )
    }

    pub fn process_studio_with_additional_filter_with_callbacks<C, P>(
        &self,
        request: AudioProcessingRequest,
        additional_filter: Option<&str>,
        is_cancelled: C,
        on_progress_line: P,
    ) -> Result<AudioProcessingResult, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
        self.process_with_format_with_callbacks(
            request,
            additional_filter,
            true,
            is_cancelled,
            on_progress_line,
        )
    }

    fn process_with_format_with_callbacks<C, P>(
        &self,
        mut request: AudioProcessingRequest,
        additional_filter: Option<&str>,
        studio: bool,
        mut is_cancelled: C,
        mut on_progress_line: P,
    ) -> Result<AudioProcessingResult, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
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

        if !request.remove_intervals.is_empty() && request.clip_end_seconds.is_none() {
            // Bound the crossfade against a short final chunk and omit an empty tail.
            request.clip_end_seconds = Some(self.probe_duration_with_callbacks(
                &request.input_path,
                &mut is_cancelled,
                &mut on_progress_line,
            )?);
        }
        let args = if studio {
            Self::studio_ffmpeg_arguments(&request, additional_filter)?
        } else {
            Self::ffmpeg_arguments_with_additional_filter(&request, additional_filter)?
        };
        run_process(
            &self.ffmpeg_command,
            &args,
            &mut is_cancelled,
            &mut on_progress_line,
        )
        .map_err(|error| map_process_error(&self.ffmpeg_command, error))?;

        let duration_seconds = self.probe_duration_with_callbacks(
            &request.output_path,
            &mut is_cancelled,
            &mut on_progress_line,
        )?;
        Ok(AudioProcessingResult {
            output_path: request.output_path,
            duration_seconds,
            output_format: request.output_format,
        })
    }

    pub fn probe_duration(&self, path: &Path) -> Result<f64, AudioProcessingError> {
        self.probe_duration_with_callbacks(path, || false, |_| {})
    }

    pub fn probe_duration_with_callbacks<C, P>(
        &self,
        path: &Path,
        mut is_cancelled: C,
        mut on_progress_line: P,
    ) -> Result<f64, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
        let args = [
            OsString::from("-v"),
            OsString::from("error"),
            OsString::from("-show_entries"),
            OsString::from("format=duration"),
            OsString::from("-of"),
            OsString::from("default=noprint_wrappers=1:nokey=1"),
            path.to_path_buf().into_os_string(),
        ];
        let result = run_process(
            &self.ffprobe_command,
            &args,
            &mut is_cancelled,
            &mut on_progress_line,
        )
        .map_err(|error| map_process_error(&self.ffprobe_command, error))?;

        let raw = String::from_utf8_lossy(&result.stdout).trim().to_string();
        let duration_seconds = raw
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| AudioProcessingError::InvalidDuration {
                command: self.ffprobe_command.display().to_string(),
            })?;
        // MP3 Xing/VBR headers are sometimes stale or truncated. Prefer the actual
        // packet timeline for MP3 so trim controls do not cut valid trailing audio.
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("mp3"))
        {
            let args = [
                OsString::from("-v"),
                OsString::from("error"),
                OsString::from("-select_streams"),
                OsString::from("a:0"),
                OsString::from("-show_packets"),
                OsString::from("-show_entries"),
                OsString::from("packet=pts_time,duration_time"),
                OsString::from("-of"),
                OsString::from("csv=p=0"),
                path.to_path_buf().into_os_string(),
            ];
            match run_process(
                &self.ffprobe_command,
                &args,
                &mut is_cancelled,
                &mut on_progress_line,
            ) {
                Ok(packets) => {
                    if let Some(packet_duration) = packet_timeline_duration(&packets.stdout) {
                        return Ok(packet_duration);
                    }
                }
                Err(error @ (ProcessError::Cancelled { .. } | ProcessError::Terminate { .. })) => {
                    return Err(map_process_error(&self.ffprobe_command, error));
                }
                // Packet probing is best-effort when format duration is available.
                Err(_) => {}
            }
        }
        Ok(duration_seconds)
    }

    /// Find low-energy intervals with FFmpeg's silencedetect filter. Detection does not
    /// alter or encode the source; returned times use the source's timeline.
    pub fn detect_silences(
        &self,
        path: &Path,
        threshold_db: f64,
        minimum_seconds: f64,
    ) -> Result<Vec<DetectedSilence>, AudioProcessingError> {
        self.detect_silences_with_callbacks(path, threshold_db, minimum_seconds, || false, |_| {})
    }

    pub fn detect_silences_with_callbacks<C, P>(
        &self,
        path: &Path,
        threshold_db: f64,
        minimum_seconds: f64,
        mut is_cancelled: C,
        mut on_progress_line: P,
    ) -> Result<Vec<DetectedSilence>, AudioProcessingError>
    where
        C: FnMut() -> bool,
        P: FnMut(&str),
    {
        if !threshold_db.is_finite()
            || threshold_db > 0.0
            || !minimum_seconds.is_finite()
            || minimum_seconds <= 0.0
        {
            return Err(AudioProcessingError::InvalidDuration {
                command: "silencedetect settings".to_string(),
            });
        }
        let args = [
            OsString::from("-hide_banner"),
            OsString::from("-i"),
            path.as_os_str().to_owned(),
            OsString::from("-af"),
            OsString::from(format!(
                "silencedetect=noise={threshold_db:.2}dB:d={minimum_seconds:.3}"
            )),
            OsString::from("-f"),
            OsString::from("null"),
            OsString::from("-"),
        ];
        let result = run_process(
            &self.ffmpeg_command,
            &args,
            &mut is_cancelled,
            &mut on_progress_line,
        )
        .map_err(|error| map_process_error(&self.ffmpeg_command, error))?;
        let log = String::from_utf8_lossy(&result.stderr);
        let mut open_start = None;
        let mut silences = Vec::new();
        for line in log.lines() {
            if let Some(value) = line
                .split("silence_start:")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
            {
                open_start = value.parse::<f64>().ok();
            }
            if let Some(value) = line
                .split("silence_end:")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                && let (Some(start_seconds), Ok(end_seconds)) =
                    (open_start.take(), value.parse::<f64>())
                && end_seconds > start_seconds
            {
                silences.push(DetectedSilence {
                    start_seconds,
                    end_seconds,
                });
            }
        }
        if let Some(start_seconds) = open_start {
            let end_seconds =
                self.probe_duration_with_callbacks(path, &mut is_cancelled, &mut on_progress_line)?;
            if end_seconds > start_seconds {
                silences.push(DetectedSilence {
                    start_seconds,
                    end_seconds,
                });
            }
        }
        Ok(silences)
    }
}

fn packet_timeline_duration(output: &[u8]) -> Option<f64> {
    let mut first_pts = None;
    let mut last_end = None;
    for line in String::from_utf8_lossy(output).lines() {
        let mut fields = line.trim().split(',');
        let pts = fields.next()?.parse::<f64>().ok()?;
        let packet_duration = fields.next()?.parse::<f64>().ok()?;
        if !pts.is_finite() || !packet_duration.is_finite() || packet_duration < 0.0 {
            continue;
        }
        first_pts.get_or_insert(pts);
        last_end = Some(pts + packet_duration);
    }
    let duration = last_end? - first_pts?;
    (duration.is_finite() && duration > 0.0).then_some(duration)
}

fn map_process_error(command: &Path, error: ProcessError) -> AudioProcessingError {
    let command = command.display().to_string();
    match error {
        ProcessError::Start { source, .. } => {
            AudioProcessingError::StartCommand { command, source }
        }
        ProcessError::Cancelled { .. } => AudioProcessingError::Cancelled { command },
        ProcessError::Failed { message, .. } => {
            AudioProcessingError::CommandFailed { command, message }
        }
        other => AudioProcessingError::CommandFailed {
            command,
            message: other.to_string(),
        },
    }
}

pub(crate) fn resolve_tool(environment_variable: &str, command: &str) -> PathBuf {
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

#[cfg(test)]
mod packet_duration_tests {
    use super::packet_timeline_duration;

    #[test]
    fn packet_timeline_uses_last_audio_packet_end() {
        let packets = b"0.000000,0.026122\n0.026122,0.026122\n1290.318367,0.026122\n";
        assert!((packet_timeline_duration(packets).unwrap() - 1290.344489).abs() < 0.000001);
    }
}
