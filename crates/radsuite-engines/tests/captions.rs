use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{
    CaptionFormat, CaptionProcessingError, CaptionProcessingRequest, CaptionProcessor,
};

#[test]
fn caption_processor_builds_trimmed_vtt_arguments() {
    let processor = CaptionProcessor::from_commands("whisper", "model.bin");
    let args = processor
        .whisper_arguments(&CaptionProcessingRequest {
            input_path: PathBuf::from("lecture.wav"),
            output_path: PathBuf::from("captions.vtt"),
            caption_format: CaptionFormat::Vtt,
            language: "en".to_string(),
            clip_start_seconds: Some(2.5),
            clip_end_seconds: Some(12.0),
        })
        .expect("build caption arguments");
    let args = display_args(&args);

    assert!(args.windows(2).any(|pair| pair == ["-m", "model.bin"]));
    assert!(args.windows(2).any(|pair| pair == ["-f", "lecture.wav"]));
    assert!(args.windows(2).any(|pair| pair == ["-ovtt", "-l"]));
    assert!(args.contains(&"2500".to_string()));
    assert!(args.contains(&"9500".to_string()));
}

#[test]
fn caption_processor_runs_cli_and_counts_srt_segments() {
    let dir = test_dir("process");
    let whisper = write_executable(
        &dir,
        "whisper.sh",
        "#!/bin/sh\noutput=''\nformat='srt'\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  if [ \"$arg\" = \"-ovtt\" ]; then format='vtt'; fi\n  previous=\"$arg\"\ndone\nif [ \"$format\" = \"vtt\" ]; then printf 'WEBVTT\\n\\n00:00.000 --> 00:01.000\\nHello\\n' > \"$output.vtt\"; else printf '1\\n00:00:00,000 --> 00:00:01,000\\nHello\\n\\n2\\n00:00:01,000 --> 00:00:02,000\\nWorld\\n' > \"$output.srt\"; fi\n",
    );
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");
    let output = dir.join("outputs").join("captions.srt");

    let result = CaptionProcessor::from_commands(whisper, model)
        .process(CaptionProcessingRequest {
            input_path: input,
            output_path: output.clone(),
            caption_format: CaptionFormat::Srt,
            language: "en".to_string(),
            clip_start_seconds: None,
            clip_end_seconds: None,
        })
        .expect("process captions");

    assert_eq!(result.output_path, output);
    assert_eq!(result.segment_count, 2);
    assert_eq!(result.caption_format, CaptionFormat::Srt);
    remove_dir(dir);
}

#[test]
fn caption_processor_reports_a_missing_model_before_running() {
    let dir = test_dir("missing-model");
    let whisper = write_executable(&dir, "whisper.sh", "#!/bin/sh\nexit 0\n");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");

    let error = CaptionProcessor::from_commands(whisper, dir.join("missing.bin"))
        .process(CaptionProcessingRequest {
            input_path: input,
            output_path: dir.join("captions.srt"),
            caption_format: CaptionFormat::Srt,
            language: "en".to_string(),
            clip_start_seconds: None,
            clip_end_seconds: None,
        })
        .expect_err("missing model");

    assert!(matches!(error, CaptionProcessingError::MissingModel { .. }));
    remove_dir(dir);
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
    let path = std::env::temp_dir().join(format!("radsuite-radcast-captions-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
