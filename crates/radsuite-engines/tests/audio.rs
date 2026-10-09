use std::path::PathBuf;

#[cfg(unix)]
use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::{os::unix::fs::PermissionsExt, path::Path};

use radsuite_engines::{
    AudioOutputFormat, AudioProcessingError, AudioProcessingRequest, AudioProcessor,
    AudioTimeInterval, RADCAST_NATURAL_DOUBLE_PLUS_POSTFILTER, RADCAST_NATURAL_PLUS_POSTFILTER,
    RADCAST_NATURAL_POSTFILTER, RADCAST_OPTIMIZED_POSTFILTER,
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
#[cfg(unix)]
fn detects_a_long_silent_audio_interval_with_ffmpeg() {
    let dir = test_dir("silencedetect");
    let input = dir.join("silence.wav");
    let generated = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=16000:cl=mono:d=4",
            "-y",
        ])
        .arg(&input)
        .status();
    let Ok(status) = generated else {
        remove_dir(dir);
        return;
    };
    if !status.success() {
        remove_dir(dir);
        return;
    }
    let processor = AudioProcessor::from_commands("ffmpeg", "ffprobe");
    let silences = processor
        .detect_silences(&input, -40.0, 2.0)
        .expect("silence detection succeeds");
    assert_eq!(silences.len(), 1);
    assert!(silences[0].end_seconds - silences[0].start_seconds >= 3.9);
    remove_dir(dir);
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
        mp3[0..7],
        [
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            "source.wav",
            "-ss"
        ]
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
fn audio_processing_can_apply_the_radcast_optimized_postfilter() {
    let args = AudioProcessor::ffmpeg_arguments_with_additional_filter(
        &request(AudioOutputFormat::Wav),
        Some(RADCAST_OPTIMIZED_POSTFILTER),
    )
    .expect("build RADcast Optimized filter arguments");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].as_str())
        .expect("audio filter");
    assert!(filter.starts_with("highpass=f=65,equalizer=f=142"));
    assert!(filter.contains("deesser=i=0.045:m=0.18:f=0.5:s=o"));
    assert!(filter.contains("loudnorm=I=-20.75:TP=-1.5:LRA=8"));
    assert!(filter.ends_with("lowpass=f=7550"));
}

#[test]
fn audio_processing_can_apply_the_radcast_natural_postfilter() {
    let args = AudioProcessor::ffmpeg_arguments_with_additional_filter(
        &request(AudioOutputFormat::Wav),
        Some(RADCAST_NATURAL_POSTFILTER),
    )
    .expect("build RADcast Natural filter arguments");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].as_str())
        .expect("audio filter");
    assert!(filter.contains("deesser=i=0.012:m=0.08:f=0.5:s=o"));
    assert!(filter.contains("equalizer=f=3000:t=q:w=1.0:g=-0.60"));
    assert!(filter.ends_with("lowpass=f=8200"));
}

#[test]
fn audio_processing_can_apply_the_radcast_natural_plus_postfilter() {
    let args = AudioProcessor::ffmpeg_arguments_with_additional_filter(
        &request(AudioOutputFormat::Wav),
        Some(RADCAST_NATURAL_PLUS_POSTFILTER),
    )
    .expect("build RADcast Natural+ filter arguments");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].as_str())
        .expect("audio filter");
    assert!(filter.contains("deesser=i=0.006:m=0.04:f=0.5:s=o"));
    assert!(filter.contains("equalizer=f=3000:t=q:w=1.0:g=-0.30"));
    assert!(filter.ends_with("lowpass=f=8800"));
}

#[test]
fn audio_processing_can_apply_the_radcast_natural_double_plus_postfilter() {
    let args = AudioProcessor::ffmpeg_arguments_with_additional_filter(
        &request(AudioOutputFormat::Wav),
        Some(RADCAST_NATURAL_DOUBLE_PLUS_POSTFILTER),
    )
    .expect("build RADcast Natural++ filter arguments");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].as_str())
        .expect("audio filter");
    assert!(filter.contains("equalizer=f=130:t=q:w=1.0:g=2.2"));
    assert!(filter.contains("equalizer=f=280:t=q:w=1.1:g=-1.2"));
    assert!(filter.contains("acompressor=threshold=0.12:ratio=1.55"));
    assert!(filter.ends_with("lowpass=f=10000"));
}

#[test]
fn audio_processing_keeps_only_the_configured_length_of_long_silences() {
    let args = AudioProcessor::ffmpeg_arguments(&AudioProcessingRequest {
        max_silence_seconds: Some(1.0),
        ..request(AudioOutputFormat::Mp3)
    })
    .expect("build pause cleanup arguments");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].clone())
        .expect("audio filter");
    assert!(filter.contains("silenceremove"));
    assert!(filter.contains("stop_duration=1.000"));
    assert!(filter.contains("stop_silence=1.000"));
}

#[test]
fn audio_processing_accepts_zero_seconds_for_pause_limit() {
    let args = AudioProcessor::ffmpeg_arguments(&AudioProcessingRequest {
        max_silence_seconds: Some(0.0),
        ..request(AudioOutputFormat::Mp3)
    })
    .expect("zero-second pause limit is a supported slider value");
    let args = display_args(&args);

    let filter = args
        .windows(2)
        .find(|pair| pair[0] == "-af")
        .map(|pair| pair[1].clone())
        .expect("audio filter");
    assert!(filter.contains("stop_duration=0.000"));
    assert!(filter.contains("stop_silence=0.000"));
}

#[test]
fn audio_processing_builds_a_crossfade_graph_for_filler_intervals() {
    let args = AudioProcessor::ffmpeg_arguments(&AudioProcessingRequest {
        clip_start_seconds: Some(2.5),
        clip_end_seconds: Some(9.0),
        remove_intervals: vec![AudioTimeInterval {
            start_seconds: 1.25,
            end_seconds: 1.75,
        }],
        cleanup_enabled: true,
        max_silence_seconds: Some(1.0),
        ..request(AudioOutputFormat::Mp3)
    })
    .expect("build filler removal arguments");
    let args = display_args(&args);

    let graph = args
        .windows(2)
        .find(|pair| pair[0] == "-filter_complex")
        .map(|pair| pair[1].clone())
        .expect("concat filter graph");
    assert!(graph.contains("atrim=start_sample=120000:end_sample=432000"));
    assert!(graph.contains("atrim=start_sample=0:end_sample=60000"));
    assert!(graph.contains("atrim=start_sample=84000:end_sample=312000"));
    assert!(graph.contains("acrossfade=ns=576:c1=tri:c2=tri[outa]"));
    assert!(graph.contains("afftdn"));
    assert!(graph.contains("silenceremove"));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["-map", "[outa_filtered]"])
    );
    assert!(!args.contains(&"-af".to_string()));
}

#[test]
#[cfg(unix)]
fn removal_crossfades_match_python_duration_and_protect_selected_clip() {
    let dir = test_dir("crossfade-real");
    let input = dir.join("source.wav");
    let output = dir.join("cut.wav");
    // Four seconds of stereo, with distinct levels outside/inside the selected clip.
    write_pcm_wav(
        &input,
        &(0..192000)
            .map(|i| {
                if i < 48000 {
                    [0.7_f32, -0.7]
                } else if i < 96000 {
                    [0.2, -0.2]
                } else if i < 144000 {
                    [-0.2, 0.2]
                } else {
                    [0.8, -0.8]
                }
            })
            .collect::<Vec<_>>(),
    );
    let result = AudioProcessor::from_commands("ffmpeg", "ffprobe")
        .process(AudioProcessingRequest {
            input_path: input,
            output_path: output.clone(),
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: Some(1.0),
            clip_end_seconds: Some(3.0),
            cleanup_enabled: false,
            max_silence_seconds: None,
            remove_intervals: vec![AudioTimeInterval {
                start_seconds: 0.8,
                end_seconds: 1.2,
            }],
        })
        .unwrap();
    assert!(
        (result.duration_seconds - 1.588).abs() < 0.0001,
        "{}",
        result.duration_seconds
    );
    let samples = read_stereo_samples(&output);
    assert!(
        samples
            .iter()
            .all(|s| s[0].abs() <= 0.201 && s[1].abs() <= 0.201)
    );
    let jump = samples
        .windows(2)
        .map(|s| (s[0][0] - s[1][0]).abs())
        .fold(0.0, f32::max);
    assert!(jump < 0.002, "discontinuous splice: {jump}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
#[cfg(unix)]
fn removal_crossfades_handle_tiny_middle_and_trailing_chunks() {
    let dir = test_dir("crossfade-tiny");
    let input = dir.join("source.wav");
    write_pcm_wav(&input, &vec![[0.2, -0.2]; 48000]);
    for (name, removals, expected) in [
        ("tiny-middle", vec![(0.4, 0.6), (0.604, 0.8)], 0.588),
        ("tiny-tail", vec![(0.4, 0.996)], 0.4),
        ("trailing-removal", vec![(0.4, 1.0)], 0.4),
        ("leading-removal", vec![(0.0, 0.6)], 0.4),
    ] {
        let result = AudioProcessor::from_commands("ffmpeg", "ffprobe")
            .process(AudioProcessingRequest {
                input_path: input.clone(),
                output_path: dir.join(format!("{name}.wav")),
                output_format: AudioOutputFormat::Wav,
                clip_start_seconds: None,
                clip_end_seconds: None,
                cleanup_enabled: false,
                max_silence_seconds: None,
                remove_intervals: removals
                    .into_iter()
                    .map(|(start_seconds, end_seconds)| AudioTimeInterval {
                        start_seconds,
                        end_seconds,
                    })
                    .collect(),
            })
            .unwrap();
        assert!(
            (result.duration_seconds - expected).abs() < 0.0001,
            "{name}: {}",
            result.duration_seconds
        );
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn removal_duration_includes_crossfades_for_guarded_export_checks() {
    let intervals = [
        AudioTimeInterval {
            start_seconds: 0.5,
            end_seconds: 0.6,
        },
        AudioTimeInterval {
            start_seconds: 1.5,
            end_seconds: 1.6,
        },
        AudioTimeInterval {
            start_seconds: 2.5,
            end_seconds: 2.6,
        },
    ];
    let removed = AudioProcessor::removal_duration_seconds(&intervals, 3.0);
    assert!((removed - 0.336).abs() < 1e-9, "{removed}");
    let removed = AudioProcessor::removal_duration_seconds(
        &[AudioTimeInterval {
            start_seconds: 0.4,
            end_seconds: 0.996,
        }],
        1.0,
    );
    assert!((removed - 0.6).abs() < 1e-9, "tiny tail: {removed}");
}

#[test]
#[cfg(unix)]
fn real_cleanup_transcription_and_render_when_fixture_is_available() {
    let Ok(path) = std::env::var("RADSUITE_REAL_CLEANUP_AUDIO") else {
        return;
    };
    let input = PathBuf::from(path);
    let original = fs::read(&input).unwrap();
    let dir = test_dir("real-cleanup");
    let processor = AudioProcessor::default();
    let duration = processor.probe_duration(&input).unwrap();
    let end = duration.min(12.0);
    assert!(end > 2.0);
    let plan = radsuite_engines::CaptionProcessor::default()
        .speech_cleanup_plan(
            &radsuite_engines::CaptionTranscriptionRequest {
                input_path: input.clone(),
                language: "en".into(),
                clip_start_seconds: Some(2.0),
                clip_end_seconds: Some(end),
            },
            end - 2.0,
            Some(0.25),
            true,
            radsuite_engines::FillerRemovalMode::Aggressive,
        )
        .unwrap();
    let expected_duration =
        end - 2.0 - AudioProcessor::removal_duration_seconds(&plan.removal_intervals, end - 2.0);
    let result = processor
        .process(AudioProcessingRequest {
            input_path: input.clone(),
            output_path: dir.join("cleanup.wav"),
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: Some(2.0),
            clip_end_seconds: Some(end),
            cleanup_enabled: false,
            max_silence_seconds: None,
            remove_intervals: plan.removal_intervals,
        })
        .unwrap();
    assert!((result.duration_seconds - expected_duration).abs() < 0.002);
    assert_eq!(fs::read(input).unwrap(), original);
    println!(
        "real cleanup: {} fillers, {} pauses; {:.3} s rendered",
        plan.removed_filler_count, plan.removed_pause_count, result.duration_seconds
    );
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
fn write_pcm_wav(path: &Path, samples: &[[f32; 2]]) {
    let mut bytes = Vec::new();
    let length = (samples.len() * 4) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + length).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&48000_u32.to_le_bytes());
    bytes.extend_from_slice(&192000_u32.to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&length.to_le_bytes());
    for sample in samples {
        for channel in sample {
            bytes.extend_from_slice(&((*channel * 32767.0).round() as i16).to_le_bytes());
        }
    }
    fs::write(path, bytes).unwrap();
}

#[cfg(unix)]
fn read_stereo_samples(path: &Path) -> Vec<[f32; 2]> {
    let decoded = std::process::Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-f", "f32le", "-ac", "2", "-"])
        .output()
        .unwrap();
    assert!(decoded.status.success());
    decoded
        .stdout
        .chunks_exact(8)
        .map(|s| {
            [
                f32::from_le_bytes(s[..4].try_into().unwrap()),
                f32::from_le_bytes(s[4..].try_into().unwrap()),
            ]
        })
        .collect()
}

#[test]
#[cfg(unix)]
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
            max_silence_seconds: None,
            remove_intervals: Vec::new(),
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

#[test]
#[cfg(unix)]
fn audio_processor_exposes_runner_progress_without_changing_the_default_path() {
    let dir = test_dir("runner-progress");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf 'out_time_us=1000000\\n'\nmkdir -p \"$(dirname \"$output\")\"\nprintf 'fake audio' > \"$output\"\n",
    );
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nprintf '12.5\\n'");
    let input = dir.join("source.wav");
    let output = dir.join("outputs").join("clean.wav");
    fs::write(&input, b"source audio").expect("write source");
    let mut progress = Vec::new();

    let result = AudioProcessor::from_commands(ffmpeg, ffprobe)
        .process_with_callbacks(
            AudioProcessingRequest {
                input_path: input,
                output_path: output,
                output_format: AudioOutputFormat::Wav,
                clip_start_seconds: None,
                clip_end_seconds: None,
                cleanup_enabled: false,
                max_silence_seconds: None,
                remove_intervals: Vec::new(),
            },
            || false,
            |line| progress.push(line.to_string()),
        )
        .expect("process audio with callbacks");

    assert_eq!(result.duration_seconds, 12.5);
    assert!(progress.iter().any(|line| line == "out_time_us=1000000"));
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn audio_processor_maps_runner_cancellation_to_audio_cancellation() {
    let dir = test_dir("runner-cancel");
    let ffmpeg = write_executable(&dir, "ffmpeg.sh", "#!/bin/sh\nsleep 30\n");
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nprintf '12.5\\n'");
    let input = dir.join("source.wav");
    let output = dir.join("outputs").join("clean.wav");
    fs::write(&input, b"source audio").expect("write source");
    let mut polls = 0;

    let result = AudioProcessor::from_commands(ffmpeg, ffprobe).process_with_callbacks(
        AudioProcessingRequest {
            input_path: input,
            output_path: output,
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: false,
            max_silence_seconds: None,
            remove_intervals: Vec::new(),
        },
        || {
            polls += 1;
            polls > 3
        },
        |_| {},
    );

    assert!(matches!(
        result,
        Err(AudioProcessingError::Cancelled { .. })
    ));
    remove_dir(dir);
}

#[test]
fn studio_arguments_preserve_float_pcm_at_48khz() {
    let args = display_args(
        &AudioProcessor::studio_ffmpeg_arguments(&request(AudioOutputFormat::Wav), None)
            .expect("studio arguments"),
    );
    assert!(
        args.windows(2)
            .any(|pair| pair == ["-codec:a", "pcm_f32le"])
    );
    assert!(args.windows(2).any(|pair| pair == ["-ar", "48000"]));
    assert!(!args.iter().any(|arg| arg == "pcm_s16le"));
}

#[cfg(unix)]
#[test]
fn studio_callbacks_preserve_render_format_and_progress() {
    let dir = test_dir("studio-callbacks");
    let ffmpeg = write_executable(
        &dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nprintf '%s\\n' \"$@\" > \"$output\"\nprintf 'out_time_us=1000000\\n'\n",
    );
    let ffprobe = write_executable(&dir, "ffprobe.sh", "#!/bin/sh\nprintf '12.5\\n'");
    let input = dir.join("source.wav");
    let output = dir.join("studio.wav");
    fs::write(&input, b"source audio").unwrap();
    let mut progress = Vec::new();
    let result = AudioProcessor::from_commands(ffmpeg, ffprobe)
        .process_studio_with_additional_filter_with_callbacks(
            AudioProcessingRequest {
                input_path: input,
                output_path: output.clone(),
                ..request(AudioOutputFormat::Wav)
            },
            Some("anull"),
            || false,
            |line| progress.push(line.to_string()),
        )
        .expect("studio rendering with callbacks");
    let arguments = fs::read_to_string(output).unwrap();
    assert!(arguments.contains("pcm_f32le\n-ar\n48000\n"));
    assert!(arguments.contains("-af\nanull\n"));
    assert!(progress.iter().any(|line| line == "out_time_us=1000000"));
    assert_eq!(result.duration_seconds, 12.5);
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn removal_bounding_probe_can_be_cancelled_before_rendering() {
    let dir = test_dir("bounding-probe-cancel");
    let ffmpeg = write_executable(&dir, "ffmpeg.sh", "#!/bin/sh\nexit 99\n");
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        "#!/bin/sh\nprintf 'probe_started\\n'\nsleep 30\n",
    );
    let input = dir.join("source.wav");
    fs::write(&input, b"source audio").unwrap();
    let cancelled = std::cell::Cell::new(false);
    let result = AudioProcessor::from_commands(ffmpeg, &ffprobe).process_with_callbacks(
        AudioProcessingRequest {
            input_path: input,
            output_path: dir.join("cut.wav"),
            remove_intervals: vec![AudioTimeInterval {
                start_seconds: 0.4,
                end_seconds: 0.996,
            }],
            ..request(AudioOutputFormat::Wav)
        },
        || cancelled.get(),
        |line| {
            if line == "probe_started" {
                cancelled.set(true);
            }
        },
    );
    assert!(
        matches!(result, Err(AudioProcessingError::Cancelled { command }) if command == ffprobe.display().to_string())
    );
    remove_dir(dir);
}

#[cfg(unix)]
#[test]
fn mp3_packet_probe_preserves_cancellation() {
    let dir = test_dir("packet-probe-cancel");
    let ffprobe = write_executable(
        &dir,
        "ffprobe.sh",
        "#!/bin/sh\nfor arg in \"$@\"; do\nif [ \"$arg\" = '-show_packets' ]; then\nprintf 'packet_probe_started\\n'\nsleep 30\nexit 0\nfi\ndone\nprintf '12.5\\n'\n",
    );
    let cancelled = std::cell::Cell::new(false);
    let result = AudioProcessor::from_commands("ffmpeg", &ffprobe).probe_duration_with_callbacks(
        &dir.join("source.mp3"),
        || cancelled.get(),
        |line| {
            if line == "packet_probe_started" {
                cancelled.set(true);
            }
        },
    );
    assert!(
        matches!(result, Err(AudioProcessingError::Cancelled { command }) if command == ffprobe.display().to_string())
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
        max_silence_seconds: None,
        remove_intervals: Vec::new(),
    }
}

fn display_args(args: &[std::ffi::OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[cfg(unix)]
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

#[cfg(unix)]
fn test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("radsuite-radcast-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

#[cfg(unix)]
fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
