use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{
    EnhancementModel, EnhancementProcessingRequest, EnhancementProcessor, EnhancementQuality,
};

#[test]
fn enhancement_processor_builds_directory_helper_arguments() {
    let processor = EnhancementProcessor::from_command("radcast-studio-enhance");
    let args = processor
        .helper_arguments(&EnhancementProcessingRequest {
            input_path: PathBuf::from("/tmp/source.wav"),
            output_path: PathBuf::from("/tmp/output/enhanced.wav"),
        })
        .expect("build helper arguments");
    let args = display_args(&args);

    assert_eq!(args[0], "/tmp");
    assert_eq!(args[1], "/tmp/output");
    assert!(args.windows(2).any(|pair| pair == ["--suffix", ".wav"]));
    assert!(args.windows(2).any(|pair| pair == ["--device", "cpu"]));
    assert!(args.windows(2).any(|pair| pair == ["--nfe", "32"]));
    assert!(args.windows(2).any(|pair| pair == ["--lambd", "0.62"]));
    assert!(args.windows(2).any(|pair| pair == ["--tau", "0.45"]));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--dereverb-method", "nara"])
    );
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--nara-chunk-seconds", "8"])
    );
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--nara-overlap-seconds", "1"])
    );
    assert!(args.windows(2).any(|pair| pair == ["--nara-taps", "6"]));
    assert!(args.windows(2).any(|pair| pair == ["--nara-delay", "2"]));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--nara-iterations", "1"])
    );
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--nara-psd-context", "1"])
    );
}

#[test]
fn enhancement_processor_maps_quality_profiles_to_model_steps() {
    let processor = EnhancementProcessor::from_command("radcast-studio-enhance");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    for (quality, expected_nfe) in [
        (EnhancementQuality::Fast, "8"),
        (EnhancementQuality::Standard, "16"),
        (EnhancementQuality::High, "32"),
    ] {
        let args = processor
            .helper_arguments_with_quality(&request, quality)
            .expect("build quality arguments");
        let args = display_args(&args);
        assert!(args.windows(2).any(|pair| pair == ["--nfe", expected_nfe]));
    }
}

#[test]
fn enhancement_processor_uses_the_optimized_model_for_the_natural_variant() {
    let processor = EnhancementProcessor::from_command("radcast-studio-enhance");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    let args = processor
        .helper_arguments_for_model(
            &request,
            EnhancementModel::StudioV18Natural,
            EnhancementQuality::High,
        )
        .expect("build RADcast Natural helper arguments");
    let args = display_args(&args);

    assert!(
        args.windows(2)
            .any(|pair| pair == ["--dereverb-method", "nara"])
    );
    assert!(args.windows(2).any(|pair| pair == ["--nfe", "32"]));
}

#[test]
fn enhancement_processor_uses_the_optimized_model_for_the_natural_plus_variant() {
    let processor = EnhancementProcessor::from_command("radcast-studio-enhance");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    let args = processor
        .helper_arguments_for_model(
            &request,
            EnhancementModel::StudioV18NaturalPlus,
            EnhancementQuality::High,
        )
        .expect("build RADcast Natural+ helper arguments");
    let args = display_args(&args);

    assert!(
        args.windows(2)
            .any(|pair| pair == ["--dereverb-method", "nara"])
    );
    assert!(args.windows(2).any(|pair| pair == ["--nfe", "32"]));
}

#[test]
fn enhancement_processor_uses_speech_preserving_dereverb_for_the_natural_double_plus_variant() {
    let processor = EnhancementProcessor::from_command("radcast-studio-enhance");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    let args = processor
        .helper_arguments_for_model(
            &request,
            EnhancementModel::StudioV18NaturalDoublePlus,
            EnhancementQuality::High,
        )
        .expect("build RADcast Natural++ helper arguments");
    let args = display_args(&args);

    assert!(
        args.windows(2)
            .any(|pair| pair == ["--dereverb-method", "spectral"])
    );
    assert!(args.windows(2).any(|pair| pair == ["--reduction", "0.90"]));
    assert!(args.windows(2).any(|pair| pair == ["--gain-floor", "0.16"]));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--time-smoothing", "0.64"])
    );
    assert!(args.iter().any(|arg| arg == "--skip-enhance"));
    assert!(!args.iter().any(|arg| arg == "--nfe"));
}

#[test]
fn enhancement_processor_builds_resemble_arguments_for_the_selected_backend() {
    let processor = EnhancementProcessor::from_command("radcast-enhance");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    let args = processor
        .helper_arguments_for_model(
            &request,
            EnhancementModel::Resemble,
            EnhancementQuality::Fast,
        )
        .expect("build Resemble arguments");
    let args = display_args(&args);

    assert!(args.windows(2).any(|pair| pair == ["--nfe", "8"]));
    assert!(args.windows(2).any(|pair| pair == ["--lambd", "0.7"]));
    assert!(args.windows(2).any(|pair| pair == ["--tau", "0.5"]));
    assert!(!args.iter().any(|arg| arg == "--dereverb-method"));
}

#[test]
fn enhancement_processor_builds_deepfilternet_arguments_for_the_selected_backend() {
    let processor = EnhancementProcessor::from_command("deepFilter");
    let request = EnhancementProcessingRequest {
        input_path: PathBuf::from("/tmp/source.wav"),
        output_path: PathBuf::from("/tmp/output/enhanced.wav"),
    };

    let args = processor
        .helper_arguments_for_model(
            &request,
            EnhancementModel::DeepFilterNet,
            EnhancementQuality::Standard,
        )
        .expect("build DeepFilterNet arguments");
    let args = display_args(&args);

    assert!(
        args.windows(2)
            .any(|pair| pair == ["--output-dir", "/tmp/output"])
    );
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--model-base-dir", "DeepFilterNet3"])
    );
    assert!(args.iter().any(|arg| arg == "/tmp/source.wav"));
    assert!(args.iter().any(|arg| arg == "--no-suffix"));
}

#[test]
fn enhancement_processor_copies_the_helper_output_to_the_requested_path() {
    let dir = test_dir("process");
    let helper = write_executable(
        &dir,
        "studio.sh",
        "#!/bin/sh\nin_dir=\"$1\"\nout_dir=\"$2\"\nmkdir -p \"$out_dir\"\nprintf '%s' enhanced > \"$out_dir/input.wav\"\n",
    );
    let input = dir.join("source.wav");
    let output = dir.join("outputs").join("enhanced.wav");
    fs::write(&input, b"source").expect("write source");

    let result = EnhancementProcessor::from_command(helper)
        .process(EnhancementProcessingRequest {
            input_path: input,
            output_path: output.clone(),
        })
        .expect("process enhancement");

    assert_eq!(result.output_path, output);
    assert_eq!(
        fs::read(result.output_path).expect("read output"),
        b"enhanced"
    );
    remove_dir(dir);
}

#[test]
fn enhancement_processor_forwards_streamed_chunk_progress() {
    let dir = test_dir("progress");
    let helper = write_executable(
        &dir,
        "studio.sh",
        "#!/bin/sh\nin_dir=\"$1\"\nout_dir=\"$2\"\nmkdir -p \"$out_dir\"\nprintf '%s\\n' 'RADCAST_ENHANCE_PROGRESS 0/3'\nsleep 0.02\nprintf '%s\\n' 'RADCAST_ENHANCE_PROGRESS 1/3'\nsleep 0.02\nprintf '%s\\n' 'RADCAST_ENHANCE_PROGRESS 2/3'\nsleep 0.02\nprintf '%s\\n' 'RADCAST_ENHANCE_PROGRESS 3/3'\nprintf '%s' enhanced > \"$out_dir/input.wav\"\n",
    );
    let input = dir.join("source.wav");
    let output = dir.join("outputs").join("enhanced.wav");
    fs::write(&input, b"source").expect("write source");
    let mut progress = Vec::new();

    EnhancementProcessor::from_command(helper)
        .process_model_with_quality_and_progress(
            EnhancementProcessingRequest {
                input_path: input,
                output_path: output,
            },
            EnhancementModel::StudioV18,
            EnhancementQuality::Standard,
            |completed, total| progress.push((completed, total)),
        )
        .expect("process enhancement with progress");

    assert_eq!(progress, vec![(0, 3), (1, 3), (2, 3), (3, 3)]);
    remove_dir(dir);
}

fn display_args(args: &[std::ffi::OsString]) -> Vec<String> {
    args.iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

fn write_executable(dir: &Path, filename: &str, contents: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, contents).expect("write helper");
    let mut permissions = fs::metadata(&path)
        .expect("read helper metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).expect("make helper executable");
    path
}

fn test_dir(label: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("radsuite-enhancement-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
