use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{
    AudioOutputFormat, AudioProcessingError, AudioProcessingRequest, AudioProcessor,
};

#[test]
fn audio_processing_rejects_a_clip_that_ends_before_it_starts() {
    let request = request(AudioOutputFormat::Mp3);

    let error = AudioProcessor::validate_request(&AudioProcessingRequest {
        clip_start_seconds: Some(8.0),
        clip_end_seconds: Some(3.0),
        ..request
    })
    .expect_err("invalid clip range");

    assert!(matches!(
        error,
        AudioProcessingError::InvalidClipRange { .. }
    ));
}

#[test]
fn audio_processing_builds_trimmed_cleanup_commands_for_mp3_and_wav() {
    let mp3 = AudioProcessor::ffmpeg_arguments(&AudioProcessingRequest {
        output_format: AudioOutputFormat::Mp3,
        clip_start_seconds: Some(2.5),
        clip_end_seconds: Some(12.0),
        cleanup_enabled: true,
        ..request(AudioOutputFormat::Mp3)
    })
    .expect("build MP3 arguments");
    let mp3 = display_args(&mp3);

    assert_eq!(
        mp3[0..5],
        ["-y", "-hide_banner", "-loglevel", "error", "-ss"]
    );
    assert!(mp3.contains(&"2.500".to_string()));
    assert!(mp3.contains(&"-t".to_string()));
    assert!(mp3.iter().any(|arg| arg.contains("afftdn")));
    assert!(mp3.contains(&"libmp3lame".to_string()));

    let wav = AudioProcessor::ffmpeg_arguments(&AudioProcessingRequest {
        output_format: AudioOutputFormat::Wav,
        cleanup_enabled: false,
        ..request(AudioOutputFormat::Wav)
    })
    .expect("build WAV arguments");
    let wav = display_args(&wav);
    assert!(wav.contains(&"pcm_s16le".to_string()));
    assert!(!wav.iter().any(|arg| arg.contains("afftdn")));
}

#[test]
fn audio_processor_runs_with_deterministic_tool_commands() {
    let dir = test_dir("process");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nmkdir -p \"$(dirname \"$output\")\"\nprintf 'fake audio' > \"$output\"\n",
    );
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nprintf '12.5\\n'");
    let input = dir.join("source.wav");
    let output = dir.join("outputs").join("clean.mp3");
    fs::write(&input, b"source audio").expect("write source");

    let result = AudioProcessor::from_commands(ffmpeg, ffprobe)
        .process(AudioProcessingRequest {
            input_path: input,
            output_path: output.clone(),
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: true,
        })
        .expect("process audio");

    assert_eq!(result.output_path, output);
    assert_eq!(result.duration_seconds, 12.5);
    assert_eq!(result.output_format, AudioOutputFormat::Mp3);
    assert_eq!(
        fs::read(result.output_path).expect("read output"),
        b"fake audio"
    );

    remove_dir(dir);
}

fn request(output_format: AudioOutputFormat) -> AudioProcessingRequest {
    AudioProcessingRequest {
        input_path: PathBuf::from("source.wav"),
        output_path: PathBuf::from(format!("output.{}", output_format.extension())),
        output_format,
        clip_start_seconds: None,
        clip_end_seconds: None,
        cleanup_enabled: false,
    }
}

fn display_args(args: &[std::ffi::OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn write_executable(dir: &Path, filename: &str, contents: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, contents).expect("write fake tool");
    let mut permissions = fs::metadata(&path)
        .expect("read fake tool metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make fake tool executable");
    path
}

fn test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("radsuite-radcast-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
