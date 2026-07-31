use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{
    AudioTimeInterval, CaptionFormat, CaptionProcessingError, CaptionProcessingRequest,
    CaptionProcessor, CaptionTranscriptionRequest, CaptionWord, FillerRemovalMode,
    detect_filler_intervals,
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

#[test]
fn filler_detection_respects_normal_and_aggressive_modes() {
    let words = vec![
        word("The", 0.0, 0.2, 0.98),
        word("um", 0.25, 0.45, 0.82),
        word("lecture", 0.6, 1.0, 0.96),
        word("uh", 1.2, 1.4, 0.08),
        word("continues", 1.55, 1.9, 0.95),
    ];

    let normal = detect_filler_intervals(&words, FillerRemovalMode::Normal);
    assert_eq!(normal, vec![interval(0.23, 0.47)]);

    let aggressive = detect_filler_intervals(&words, FillerRemovalMode::Aggressive);
    assert_eq!(aggressive, vec![interval(0.23, 0.47), interval(1.18, 1.42)]);
}

#[test]
fn caption_processor_extracts_word_timestamps_from_whisper_json() {
    let dir = test_dir("word-timestamps");
    let whisper = write_executable(
        &dir,
        "whisper-json.sh",
        "#!/bin/sh\noutput=''\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  previous=\"$arg\"\ndone\nprintf '{\"transcription\":[{\"tokens\":[{\"text\":\" um\",\"offsets\":{\"from\":250,\"to\":450},\"p\":0.82}]}]}' > \"$output.json\"\n",
    );
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");

    let words = CaptionProcessor::from_commands(whisper, model)
        .transcribe_words(&CaptionTranscriptionRequest {
            input_path: input,
            language: "en".to_string(),
            clip_start_seconds: None,
            clip_end_seconds: None,
        })
        .expect("transcribe word timestamps");

    assert_eq!(words, vec![word("um", 0.25, 0.45, 0.82)]);
    remove_dir(dir);
}

#[test]
fn filler_intervals_are_relative_to_the_selected_clip() {
    let dir = test_dir("clip-filler");
    let whisper = write_executable(
        &dir,
        "whisper-json.sh",
        "#!/bin/sh\noutput=''\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  previous=\"$arg\"\ndone\nprintf '{\"transcription\":[{\"tokens\":[{\"text\":\" um\",\"offsets\":{\"from\":2250,\"to\":2450},\"p\":0.82}]}]}' > \"$output.json\"\n",
    );
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");

    let intervals = CaptionProcessor::from_commands(whisper, model)
        .filler_intervals(
            &CaptionTranscriptionRequest {
                input_path: input,
                language: "en".to_string(),
                clip_start_seconds: Some(2.0),
                clip_end_seconds: Some(5.0),
            },
            FillerRemovalMode::Normal,
        )
        .expect("detect clip filler interval");

    assert_eq!(intervals, vec![interval(0.23, 0.47)]);
    remove_dir(dir);
}

fn word(text: &str, start_seconds: f64, end_seconds: f64, probability: f64) -> CaptionWord {
    CaptionWord {
        text: text.to_string(),
        start_seconds,
        end_seconds,
        probability,
    }
}

fn interval(start_seconds: f64, end_seconds: f64) -> AudioTimeInterval {
    AudioTimeInterval {
        start_seconds,
        end_seconds,
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
    let path = std::env::temp_dir().join(format!("radsuite-radcast-captions-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
