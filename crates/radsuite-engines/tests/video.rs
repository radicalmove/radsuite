use std::path::Path;

#[cfg(unix)]
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(windows)]
use std::{
    ffi::OsString,
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::time::{Duration, Instant};

use radsuite_engines::{VideoExportRequest, VideoExporter, VideoProbeError};

#[cfg(any(unix, windows))]
use radsuite_engines::{ProcessError, run_process};

#[test]
fn video_arguments_use_the_approved_fixed_mp4_contract() {
    let args = VideoExporter::ffmpeg_arguments(
        Path::new("cover.png"),
        Path::new("final.wav"),
        Path::new("output.mp4"),
    );
    let args = display_args(&args);

    assert_eq!(
        args[0..13],
        [
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-loop",
            "1",
            "-framerate",
            "30",
            "-i",
            "cover.png",
            "-i",
            "final.wav",
            "-filter_complex",
        ]
    );
    let graph = args[13].clone();
    assert_eq!(
        graph,
        "[0:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720:(iw-1280)/2:(ih-720)/2[background];[background]drawbox=x=0:y=500:w=1280:h=220:color=black@0.70:t=fill[band];[1:a]showwaves=s=1280x220:mode=cline:rate=30:colors=white,format=rgba,colorkey=black:0.01:0.0[waveform];[band][waveform]overlay=0:500,fps=30,format=yuv420p[video]"
    );
    assert_eq!(
        &args[14..],
        [
            "-map",
            "[video]",
            "-map",
            "1:a:0",
            "-c:v",
            "libx264",
            "-crf",
            "23",
            "-pix_fmt",
            "yuv420p",
            "-r",
            "30",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-shortest",
            "-movflags",
            "+faststart",
            "-progress",
            "pipe:1",
            "-nostats",
            "output.mp4",
        ]
    );
}

#[test]
fn video_probe_accepts_the_exact_required_streams_and_duration() {
    let fixture = valid_probe_json();
    let result =
        VideoExporter::validate_probe_json(&fixture, 12.5).expect("valid fixture should pass");

    assert_eq!(result.duration_seconds, 12.5);
}

#[test]
fn video_probe_rejects_duplicate_or_missing_streams() {
    for json in [
        valid_probe_json().replace(
            "{\"codec_type\": \"audio\", \"codec_name\": \"aac\"}",
            "{\"codec_type\": \"audio\", \"codec_name\": \"aac\"},{\"codec_type\": \"audio\", \"codec_name\": \"aac\"}",
        ),
        valid_probe_json().replace(
            ",\n            {\"codec_type\": \"audio\", \"codec_name\": \"aac\"}",
            "",
        ),
    ] {
        assert!(VideoExporter::validate_probe_json(&json, 12.5).is_err());
    }
}

#[test]
fn video_probe_rejects_wrong_video_contract_values() {
    for replacement in [
        ("\"codec_name\": \"h264\"", "\"codec_name\": \"hevc\""),
        ("\"width\": 1280", "\"width\": 1920"),
        ("\"height\": 720", "\"height\": 1080"),
        ("\"pix_fmt\": \"yuv420p\"", "\"pix_fmt\": \"yuv444p\""),
        ("\"r_frame_rate\": \"30/1\"", "\"r_frame_rate\": \"25/1\""),
        (
            "\"avg_frame_rate\": \"30/1\"",
            "\"avg_frame_rate\": \"25/1\"",
        ),
        ("\"codec_name\": \"aac\"", "\"codec_name\": \"mp3\""),
    ] {
        let json = valid_probe_json().replacen(replacement.0, replacement.1, 1);
        assert!(
            VideoExporter::validate_probe_json(&json, 12.5).is_err(),
            "replacement {:?} should be rejected",
            replacement
        );
    }
}

#[test]
fn video_probe_rejects_invalid_or_mismatched_duration() {
    for duration in ["0", "null", "\"NaN\"", "\"inf\""] {
        let json =
            valid_probe_json().replace("\"duration\": 12.5", &format!("\"duration\":{duration}"));
        assert!(VideoExporter::validate_probe_json(&json, 12.5).is_err());
    }

    let json = valid_probe_json().replace("\"duration\": 12.5", "\"duration\": 12.61");
    let error = VideoExporter::validate_probe_json(&json, 12.5).expect_err("duration mismatch");
    assert!(matches!(error, VideoProbeError::DurationMismatch { .. }));
}

#[test]
fn video_export_request_keeps_the_known_audio_duration() {
    let request = VideoExportRequest::new("cover.png", "final.wav", "output.mp4", 12.5);
    assert_eq!(request.audio_duration_seconds, 12.5);
}

#[cfg(unix)]
#[test]
fn process_runner_streams_progress_lines_from_a_temporary_executable() {
    let dir = test_dir("progress");
    let script = write_executable(
        &dir,
        "progress.sh",
        "#!/bin/sh\nprintf 'out_time_us=1000000\\n'\nprintf 'progress=continue\\n'\nprintf 'out_time_us=2000000\\n'\nprintf 'progress=end\\n'\n",
    );
    let mut lines = Vec::new();

    let output = run_process(&script, &[], || false, |line| lines.push(line.to_string()))
        .expect("temporary progress script should finish");

    assert!(output.status.success());
    assert!(lines.iter().any(|line| line == "out_time_us=1000000"));
    assert!(lines.iter().any(|line| line == "out_time_us=2000000"));
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn process_runner_cancels_a_temporary_executable_without_waiting_for_completion() {
    let dir = test_dir("cancel");
    let script = write_executable(&dir, "long.sh", "#!/bin/sh\nsleep 30\n");
    let started = Instant::now();
    let mut polls = 0;

    let result = run_process(
        &script,
        &[],
        || {
            polls += 1;
            polls > 3
        },
        |_| {},
    );

    assert!(matches!(result, Err(ProcessError::Cancelled { .. })));
    assert!(started.elapsed() < Duration::from_secs(5));
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_parses_ffmpeg_progress_relative_to_audio_duration() {
    let dir = test_dir("export-progress");
    let ffmpeg_output_path = dir.join("ffmpeg-output-path");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        &format!(
            "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf '%s' \"$output\" > '{}'\nprintf 'out_time_us=6250000\\nprogress=continue\\n'\nprintf 'fake mp4' > \"$output\"\n",
            ffmpeg_output_path.display()
        ),
    );
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        "#!/bin/sh\nprintf '%s\\n' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":1280,\"height\":720,\"pix_fmt\":\"yuv420p\",\"r_frame_rate\":\"30/1\",\"avg_frame_rate\":\"30/1\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}],\"format\":{\"duration\":12.5}}'\n",
    );
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    let output = dir.join("output.mp4");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");
    let mut progress = Vec::new();

    let result = VideoExporter::from_commands(ffmpeg, ffprobe)
        .export_with_callbacks(
            VideoExportRequest::new(&image, &audio, &output, 12.5),
            || false,
            |value| progress.push(value),
        )
        .expect("fake video export");

    assert_eq!(result.output_path, output);
    assert!(
        progress
            .iter()
            .any(|value| (*value - 0.5).abs() < f64::EPSILON)
    );
    let rendered_path = fs::read_to_string(&ffmpeg_output_path).expect("read partial path");
    assert!(rendered_path.contains(".partial-"));
    assert!(output.is_file());
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn process_runner_reports_non_zero_exit_and_stderr() {
    let dir = test_dir("failure");
    let script = write_executable(
        &dir,
        "failure.sh",
        "#!/bin/sh\nprintf 'diagnostic from stderr\\n' >&2\nexit 7\n",
    );

    let result = run_process(&script, &[], || false, |_| {});

    match result {
        Err(ProcessError::Failed { message, .. }) => {
            assert!(message.contains("diagnostic from stderr"));
        }
        other => panic!("expected failed process, got {other:?}"),
    }
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn process_runner_emits_progress_after_a_partial_line_is_completed() {
    let dir = test_dir("partial-progress");
    let script = write_executable(
        &dir,
        "progress.sh",
        "#!/bin/sh\nprintf 'out_time_us=1000000'\nsleep 0.05\nprintf '\\n'\n",
    );
    let mut lines = Vec::new();

    run_process(&script, &[], || false, |line| lines.push(line.to_string()))
        .expect("partial progress script should finish");

    assert!(lines.iter().any(|line| line == "out_time_us=1000000"));
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_rejects_existing_output_without_spawning_or_overwriting_it() {
    let dir = test_dir("existing-output");
    let marker = dir.join("spawned");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    );
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nexit 0\n");
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    let output = dir.join("output.mp4");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");
    fs::write(&output, b"caller-owned output").expect("write existing output");

    let result = VideoExporter::from_commands(ffmpeg, ffprobe)
        .export(VideoExportRequest::new(&image, &audio, &output, 12.5));

    assert!(matches!(
        result,
        Err(radsuite_engines::VideoExportError::OutputExists { .. })
    ));
    assert_eq!(
        fs::read(&output).expect("read existing output"),
        b"caller-owned output"
    );
    assert!(!marker.exists());
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_does_not_replace_output_created_during_probe() {
    let dir = test_dir("output-race");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf 'fake mp4' > \"$output\"\n",
    );
    let output = dir.join("output.mp4");
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        &format!(
            "#!/bin/sh\nprintf 'caller-owned output' > '{}'\nprintf '%s\\n' '{{\"streams\":[{{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":1280,\"height\":720,\"pix_fmt\":\"yuv420p\",\"r_frame_rate\":\"30/1\",\"avg_frame_rate\":\"30/1\"}},{{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}}],\"format\":{{\"duration\":12.5}}}}'\n",
            output.display()
        ),
    );
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");

    let result = VideoExporter::from_commands(ffmpeg, ffprobe)
        .export(VideoExportRequest::new(&image, &audio, &output, 12.5));

    assert!(matches!(
        result,
        Err(radsuite_engines::VideoExportError::OutputExists { .. })
    ));
    assert_eq!(
        fs::read(&output).expect("read caller-owned output"),
        b"caller-owned output"
    );
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_rechecks_cancellation_before_promoting_a_valid_probe() {
    let dir = test_dir("post-probe-cancel");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf 'fake mp4' > \"$output\"\n",
    );
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        "#!/bin/sh\nprintf '%s\\n' '{\"streams\":[{\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":1280,\"height\":720,\"pix_fmt\":\"yuv420p\",\"r_frame_rate\":\"30/1\",\"avg_frame_rate\":\"30/1\"},{\"codec_type\":\"audio\",\"codec_name\":\"aac\"}],\"format\":{\"duration\":12.5}}'\n",
    );
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    let output = dir.join("output.mp4");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");
    let mut cancellation_calls = 0;

    let result = VideoExporter::from_commands(ffmpeg, ffprobe).export_with_callbacks(
        VideoExportRequest::new(&image, &audio, &output, 12.5),
        || {
            cancellation_calls += 1;
            cancellation_calls >= 5
        },
        |_| {},
    );

    assert!(matches!(
        result,
        Err(radsuite_engines::VideoExportError::Process(
            ProcessError::Cancelled { .. }
        ))
    ));
    assert!(!output.exists());
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_reports_partial_cleanup_failures() {
    let dir = test_dir("cleanup-failure");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nmkdir \"$output\"\nprintf 'mux failed\\n' >&2\nexit 7\n",
    );
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nexit 0\n");
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    let output = dir.join("output.mp4");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");

    let result = VideoExporter::from_commands(ffmpeg, ffprobe)
        .export(VideoExportRequest::new(&image, &audio, &output, 12.5));

    match result {
        Err(radsuite_engines::VideoExportError::Cleanup {
            cleanup_failures, ..
        }) => {
            assert_eq!(cleanup_failures.len(), 1);
            assert!(
                cleanup_failures[0]
                    .path
                    .to_string_lossy()
                    .contains(".partial-")
            );
        }
        other => panic!("expected cleanup failure, got {other:?}"),
    }
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn video_export_cancellation_during_ffprobe_stops_without_succeeding() {
    let dir = test_dir("probe-cancel");
    let ffprobe_started = dir.join("ffprobe-started");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf 'fake mp4' > \"$output\"\nprintf 'out_time_us=12500000\\n'\n",
    );
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        &format!(
            "#!/bin/sh\ntouch '{}'\nsleep 1\nprintf '%s\\n' '{{\"streams\":[] ,\"format\":{{\"duration\":12.5}}}}'\n",
            ffprobe_started.display()
        ),
    );
    let image = dir.join("cover.png");
    let audio = dir.join("final.wav");
    let output = dir.join("output.mp4");
    fs::write(&image, b"image").expect("write image fixture");
    fs::write(&audio, b"audio").expect("write audio fixture");
    let result = VideoExporter::from_commands(ffmpeg, ffprobe).export_with_callbacks(
        VideoExportRequest::new(&image, &audio, &output, 12.5),
        || ffprobe_started.is_file(),
        |_| {},
    );

    assert!(matches!(
        result,
        Err(radsuite_engines::VideoExportError::Process(
            ProcessError::Cancelled { .. }
        ))
    ));
    assert!(ffprobe_started.is_file());
    assert!(!output.exists());
    remove_dir(dir);
}

#[cfg(windows)]
#[test]
fn process_runner_cancels_a_cmd_descendant_tree() {
    let dir = test_dir("windows-tree");
    let script = dir.join("tree.cmd");
    fs::write(
        &script,
        "@echo off\r\nstart \"\" /b cmd /c \"ping 127.0.0.1 -n 30 >NUL\"\r\nping 127.0.0.1 -n 30 >NUL\r\n",
    )
    .expect("write Windows process tree script");
    let started = std::time::Instant::now();
    let mut polls = 0;

    let result = run_process(
        Path::new("cmd.exe"),
        &[
            OsString::from("/D"),
            OsString::from("/C"),
            script.as_os_str().to_owned(),
        ],
        || {
            polls += 1;
            polls > 3
        },
        |_| {},
    );

    assert!(matches!(result, Err(ProcessError::Cancelled { .. })));
    assert!(started.elapsed() < Duration::from_secs(5));
    remove_dir(dir);
}

fn valid_probe_json() -> String {
    r#"{
        "streams": [
            {
                "codec_type": "video",
                "codec_name": "h264",
                "width": 1280,
                "height": 720,
                "pix_fmt": "yuv420p",
                "r_frame_rate": "30/1",
                "avg_frame_rate": "30/1"
            },
            {"codec_type": "audio", "codec_name": "aac"}
        ],
        "format": {"duration": 12.5}
    }"#
    .to_string()
}

fn display_args(args: &[std::ffi::OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[cfg(unix)]
fn write_executable(dir: &Path, filename: &str, contents: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let path = dir.join(filename);
    fs::write(&path, contents).expect("write test executable");
    let mut permissions = fs::metadata(&path)
        .expect("read test executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make test executable executable");
    path
}

#[cfg(unix)]
fn test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("radsuite-video-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

#[cfg(unix)]
fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}

#[cfg(windows)]
fn test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("radsuite-video-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

#[cfg(windows)]
fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
