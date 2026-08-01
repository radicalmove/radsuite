use std::{
    fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{
    AudioTimeInterval, CaptionFormat, CaptionProcessingError, CaptionProcessingRequest,
    CaptionProcessor, CaptionQualityMode, CaptionTranscriptionRequest, CaptionWord,
    FillerRemovalMode, SpeechCleanupPlanningError, detect_filler_intervals,
    plan_speech_cleanup,
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
    assert!(args.contains(&"-ovtt".to_string()));
    assert!(args.contains(&"-oj".to_string()));
    assert!(args.contains(&"-ojf".to_string()));
    assert!(args.contains(&"2500".to_string()));
    assert!(args.contains(&"9500".to_string()));
}

#[test]
fn caption_quality_controls_whisper_search_and_glossary_prompt() {
    let dir = test_dir("quality-options");
    let small_model = dir.join("ggml-small.bin");
    let medium_model = dir.join("ggml-medium.bin");
    fs::write(&small_model, b"small model").expect("write small model");
    fs::write(&medium_model, b"medium model").expect("write medium model");
    let processor = CaptionProcessor::from_commands("whisper", small_model.clone());
    let request = CaptionProcessingRequest {
        input_path: PathBuf::from("lecture.wav"),
        output_path: PathBuf::from("captions.srt"),
        caption_format: CaptionFormat::Srt,
        language: "en".to_string(),
        clip_start_seconds: None,
        clip_end_seconds: None,
    };

    let fast = display_args(
        &processor
            .whisper_arguments_with_options(&request, CaptionQualityMode::Fast, None)
            .expect("build fast arguments"),
    );
    assert!(fast.windows(2).any(|pair| pair == ["-bs", "1"]));
    assert!(
        fast.windows(2)
            .any(|pair| { pair == ["-m", small_model.to_string_lossy().as_ref()] })
    );

    let reviewed = display_args(
        &processor
            .whisper_arguments_with_options(
                &request,
                CaptionQualityMode::Reviewed,
                Some("Te Tiriti o Waitangi, kaiwhakahaere"),
            )
            .expect("build reviewed arguments"),
    );
    assert!(
        reviewed
            .windows(2)
            .any(|pair| pair == ["-m", medium_model.to_string_lossy().as_ref()])
    );
    assert!(reviewed.windows(2).any(|pair| pair == ["-bs", "5"]));
    assert!(reviewed.windows(2).any(|pair| {
        pair == [
            "--prompt",
            "Use these exact names and terms when they occur: Te Tiriti o Waitangi, kaiwhakahaere",
        ]
    }));
    remove_dir(dir);
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
fn reviewed_captions_write_a_confidence_review_file() {
    let dir = test_dir("caption-review");
    let whisper = write_executable(
        &dir,
        "whisper-review.sh",
        r#"#!/bin/sh
output=''
previous=''
for arg in "$@"; do
  if [ "$previous" = "-of" ]; then output="$arg"; fi
  previous="$arg"
done
printf '1\n00:00:00,000 --> 00:00:01,000\nLow confidence line\n\n2\n00:00:01,000 --> 00:00:02,000\nStable line\n' > "$output.srt"
printf '%s' '{"transcription":[{"offsets":{"from":0,"to":1000},"tokens":[{"text":" Low","offsets":{"from":0,"to":400},"p":0.35},{"text":" confidence","offsets":{"from":400,"to":800},"p":0.35},{"text":" line","offsets":{"from":800,"to":1000},"p":0.35}]},{"offsets":{"from":1000,"to":2000},"tokens":[{"text":" Stable","offsets":{"from":1000,"to":1400},"p":0.92},{"text":" line","offsets":{"from":1400,"to":2000},"p":0.92}]}]}' > "$output.json"
"#,
    );
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");
    let output = dir.join("outputs").join("captions.srt");

    let result = CaptionProcessor::from_commands(whisper, model)
        .process_with_options(
            CaptionProcessingRequest {
                input_path: input,
                output_path: output.clone(),
                caption_format: CaptionFormat::Srt,
                language: "en".to_string(),
                clip_start_seconds: None,
                clip_end_seconds: None,
            },
            CaptionQualityMode::Reviewed,
            None,
        )
        .expect("process reviewed captions");

    assert_eq!(result.segment_count, 2);
    assert_eq!(result.quality.total_segment_count, 2);
    assert_eq!(result.quality.low_confidence_segment_count, 1);
    assert!(result.quality.review_recommended);
    assert_eq!(result.quality.average_probability, Some(0.635));
    let review_path = result.quality.review_path.expect("review file path");
    assert_eq!(
        review_path,
        output.with_file_name("captions.srt.review.txt")
    );
    let review = fs::read_to_string(review_path).expect("read caption review");
    assert!(review.contains("Low-confidence caption lines: 1"));
    assert!(review.contains("Low confidence line"));

    remove_dir(dir);
}

#[test]
fn reviewed_captions_flag_text_without_confidence_data() {
    let dir = test_dir("caption-review-without-confidence");
    let whisper = write_executable(
        &dir,
        "whisper-review.sh",
        r#"#!/bin/sh
output=''
previous=''
for arg in "$@"; do
  if [ "$previous" = "-of" ]; then output="$arg"; fi
  previous="$arg"
done
printf '1\n00:00:00,000 --> 00:00:01,000\nNeeds review\n' > "$output.srt"
printf '%s' '{"transcription":[{"offsets":{"from":0,"to":1000},"text":"Needs review"}]}' > "$output.json"
"#,
    );
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");
    let output = dir.join("outputs").join("captions.srt");

    let result = CaptionProcessor::from_commands(whisper, model)
        .process_with_options(
            CaptionProcessingRequest {
                input_path: input,
                output_path: output,
                caption_format: CaptionFormat::Srt,
                language: "en".to_string(),
                clip_start_seconds: None,
                clip_end_seconds: None,
            },
            CaptionQualityMode::Reviewed,
            None,
        )
        .expect("process reviewed captions");

    assert_eq!(result.quality.average_probability, None);
    assert_eq!(result.quality.low_confidence_segment_count, 1);
    assert!(result.quality.review_recommended);
    let review_path = result.quality.review_path.expect("review file path");
    let review = fs::read_to_string(review_path).expect("read caption review");
    assert!(review.contains("No word confidence data was available"));
    assert!(review.contains("Needs review"));

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

#[test]
fn caption_processor_builds_cleanup_plan_from_one_transcription() {
    let dir = test_dir("speech-cleanup-processor");
    let count_path = dir.join("transcription-count");
    let whisper_script = format!(
        "#!/bin/sh\ncount_file='{}'\ncount=0\nif [ -f \"$count_file\" ]; then count=$(cat \"$count_file\"); fi\ncount=$((count + 1))\nprintf '%s' \"$count\" > \"$count_file\"\noutput=''\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  previous=\"$arg\"\ndone\nprintf '{{\"transcription\":[{{\"tokens\":[{{\"text\":\" first\",\"offsets\":{{\"from\":2250,\"to\":2450}},\"p\":0.9}},{{\"text\":\" second\",\"offsets\":{{\"from\":3500,\"to\":3800}},\"p\":0.9}}]}}]}}' > \"$output.json\"\n",
        count_path.display()
    );
    let whisper = write_executable(&dir, "whisper-cleanup.sh", &whisper_script);
    let model = dir.join("model.bin");
    fs::write(&model, b"model").expect("write model");
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").expect("write source");

    let plan = CaptionProcessor::from_commands(whisper, model)
        .speech_cleanup_plan(
            &CaptionTranscriptionRequest {
                input_path: input,
                language: "en".to_string(),
                clip_start_seconds: Some(2.0),
                clip_end_seconds: Some(4.0),
            },
            2.0,
            Some(0.1),
            false,
            FillerRemovalMode::Normal,
        )
        .expect("build cleanup plan from transcription");

    assert_eq!(plan.removal_intervals.len(), 1);
    assert!((plan.removal_intervals[0].start_seconds - 0.55).abs() < 1e-9);
    assert_eq!(plan.removal_intervals[0].end_seconds, 1.5);
    assert_eq!(plan.removed_pause_count, 1);
    assert_eq!(fs::read_to_string(count_path).expect("read transcription count"), "1");
    remove_dir(dir);
}

#[test]
fn speech_cleanup_shortens_leading_inter_word_and_trailing_pauses() {
    let plan = plan_speech_cleanup(
        &[
            word("hello", 1.0, 1.2, 0.9),
            word("world", 2.0, 2.2, 0.9),
        ],
        3.0,
        Some(0.4),
        false,
        FillerRemovalMode::Normal,
    )
    .expect("build speech cleanup plan");

    assert_eq!(
        plan.removal_intervals,
        vec![interval(0.4, 1.0), interval(1.6, 2.0), interval(2.6, 3.0)]
    );
    assert_eq!(plan.removed_pause_count, 3);
    assert_eq!(plan.removed_filler_count, 0);
}

#[test]
fn speech_cleanup_keeps_gaps_at_or_below_the_original_threshold() {
    let plan = plan_speech_cleanup(
        &[
            word("first", 0.0, 0.2, 0.9),
            word("second", 0.55, 0.75, 0.9),
        ],
        0.75,
        Some(0.0),
        false,
        FillerRemovalMode::Normal,
    )
    .expect("build speech cleanup plan");

    assert!(plan.removal_intervals.is_empty());
    assert_eq!(plan.removed_pause_count, 0);
}

#[test]
fn speech_cleanup_excludes_fillers_from_pause_timeline_and_merges_overlaps() {
    let plan = plan_speech_cleanup(
        &[
            word("hello", 0.0, 0.2, 0.9),
            word("um", 0.3, 0.5, 0.9),
            word("world", 0.6, 0.8, 0.9),
        ],
        0.8,
        Some(0.0),
        true,
        FillerRemovalMode::Normal,
    )
    .expect("build speech cleanup plan");

    assert_eq!(plan.removal_intervals, vec![interval(0.2, 0.6)]);
    assert_eq!(plan.removed_pause_count, 1);
    assert_eq!(plan.removed_filler_count, 1);

    let filler_only = plan_speech_cleanup(
        &[
            word("hello", 0.0, 0.2, 0.9),
            word("um", 0.3, 0.5, 0.9),
            word("world", 0.6, 0.8, 0.9),
        ],
        0.8,
        None,
        true,
        FillerRemovalMode::Normal,
    )
    .expect("build filler-only cleanup plan");

    assert_eq!(filler_only.removal_intervals, vec![interval(0.28, 0.52)]);
    assert_eq!(filler_only.removed_pause_count, 0);
    assert_eq!(filler_only.removed_filler_count, 1);
}

#[test]
fn speech_cleanup_rejects_invalid_timing_inputs() {
    let error = plan_speech_cleanup(
        &[word("broken", -0.1, 0.2, 0.9)],
        1.0,
        Some(0.5),
        false,
        FillerRemovalMode::Normal,
    )
    .expect_err("negative word timing must fail");
    assert!(matches!(
        error,
        SpeechCleanupPlanningError::InvalidWordTiming { index: 0, .. }
    ));

    let error = plan_speech_cleanup(
        &[word("broken", 0.1, 1.1, 0.9)],
        1.0,
        Some(0.5),
        false,
        FillerRemovalMode::Normal,
    )
    .expect_err("out-of-range word timing must fail");
    assert!(matches!(
        error,
        SpeechCleanupPlanningError::InvalidWordTiming { index: 0, .. }
    ));

    let error = plan_speech_cleanup(
        &[],
        1.0,
        Some(f64::NAN),
        false,
        FillerRemovalMode::Normal,
    )
    .expect_err("non-finite pause duration must fail");
    assert!(matches!(
        error,
        SpeechCleanupPlanningError::InvalidMaxSilence { .. }
    ));
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
    let temporary_path = dir.join(format!(".{filename}.tmp"));
    let mut file = fs::File::create(&temporary_path).expect("create fake tool");
    file.write_all(contents.as_bytes())
        .expect("write fake tool");
    file.sync_all().expect("sync fake tool");
    drop(file);
    let mut permissions = fs::metadata(&temporary_path)
        .expect("read fake tool metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&temporary_path, permissions).expect("make fake tool executable");
    fs::rename(temporary_path, &path).expect("publish fake tool");
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
