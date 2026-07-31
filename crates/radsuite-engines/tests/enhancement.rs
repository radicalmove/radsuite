use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_engines::{EnhancementProcessingRequest, EnhancementProcessor};

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
