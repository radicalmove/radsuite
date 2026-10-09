//! Versioned embedded helper; no changes to historical/external RADcast helpers.
use crate::{
    EnhancementProcessingError, EnhancementProcessingRequest, EnhancementProcessingResult,
};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn python_command() -> PathBuf {
    if let Some(value) = std::env::var_os("RADSUITE_STUDIO_TREBLE_PYTHON") {
        return PathBuf::from(value);
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .unwrap_or_default();
    for env in ["venv311", "venv"] {
        let p = home.join(".radcast").join(env).join(if cfg!(windows) {
            "Scripts/python.exe"
        } else {
            "bin/python"
        });
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("python3")
}

pub(crate) fn is_available(python: &Path) -> bool {
    run_helper(python, &[OsString::from("--check-runtime")]).is_ok()
        && [
            crate::audio::resolve_tool("RADSUITE_FFMPEG", "ffmpeg"),
            crate::audio::resolve_tool("RADSUITE_FFPROBE", "ffprobe"),
        ]
        .iter()
        .all(|tool| {
            Command::new(tool)
                .arg("-version")
                .output()
                .is_ok_and(|o| o.status.success())
        })
}

fn run_helper(python: &Path, args: &[OsString]) -> Result<(), EnhancementProcessingError> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let folder = std::env::temp_dir().join(format!(
        "radcast-studio-treble-{}-{stamp}",
        std::process::id()
    ));
    fs::create_dir_all(&folder).map_err(EnhancementProcessingError::PrepareWorkspace)?;
    let result = (|| {
        fs::write(
            folder.join("analysis.py"),
            include_str!("../../../tools/radcast/analysis.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::write(
            folder.join("speech_cleanup.py"),
            include_str!("../../../tools/radcast/speech_cleanup.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::write(
            folder.join("studio.py"),
            include_str!("../../../tools/radcast/studio.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::write(
            folder.join("treble_model.py"),
            include_str!("../../../tools/radcast/treble_model.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::write(
            folder.join("treble_preservation.py"),
            include_str!("../../../tools/radcast/treble_preservation.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        fs::write(
            folder.join("studio_treble.py"),
            include_str!("../../../tools/radcast/studio_treble.py"),
        )
        .map_err(EnhancementProcessingError::PrepareWorkspace)?;
        let output = Command::new(python)
            .arg(folder.join("studio_treble.py"))
            .args(args)
            .env("OMP_NUM_THREADS", "4")
            .env("OPENBLAS_NUM_THREADS", "4")
            .env("MKL_NUM_THREADS", "4")
            .env(
                "RADSUITE_STUDIO_FFMPEG",
                crate::audio::resolve_tool("RADSUITE_FFMPEG", "ffmpeg"),
            )
            .env(
                "RADSUITE_STUDIO_FFPROBE",
                crate::audio::resolve_tool("RADSUITE_FFPROBE", "ffprobe"),
            )
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .output()
            .map_err(|source| EnhancementProcessingError::StartCommand {
                command: python.display().to_string(),
                source,
            })?;
        if !output.status.success() {
            return Err(EnhancementProcessingError::CommandFailed {
                message: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        Ok(())
    })();
    let _ = fs::remove_dir_all(folder);
    result
}

pub(crate) fn process(
    python: &Path,
    request: EnhancementProcessingRequest,
) -> Result<EnhancementProcessingResult, EnhancementProcessingError> {
    if !request.input_path.is_file() {
        return Err(EnhancementProcessingError::MissingInput {
            path: request.input_path,
        });
    }
    let parent = request.output_path.parent().ok_or_else(|| {
        EnhancementProcessingError::MissingOutputParent {
            path: request.output_path.clone(),
        }
    })?;
    fs::create_dir_all(parent).map_err(EnhancementProcessingError::PrepareWorkspace)?;
    let result = run_helper(
        python,
        &[
            request.input_path.as_os_str().to_owned(),
            request.output_path.as_os_str().to_owned(),
        ],
    );
    if let Err(error) = result {
        let _ = fs::remove_file(&request.output_path);
        let _ = fs::remove_file(request.output_path.with_extension("qa.json"));
        return Err(error);
    }
    if !request.output_path.is_file() || !request.output_path.with_extension("qa.json").is_file() {
        let _ = fs::remove_file(&request.output_path);
        return Err(EnhancementProcessingError::MissingOutput {
            path: request.output_path,
        });
    }
    Ok(EnhancementProcessingResult {
        output_path: request.output_path,
    })
}

pub(crate) fn verify_export(
    python: &Path,
    source: &Path,
    export: &Path,
    qa: &Path,
    origin: &Path,
    edited: bool,
    removed_seconds: f64,
) -> Result<(), EnhancementProcessingError> {
    let mut args = vec![
        OsString::from("--verify-export"),
        source.as_os_str().to_owned(),
        export.as_os_str().to_owned(),
        qa.as_os_str().to_owned(),
        OsString::from("--origin"),
        origin.as_os_str().to_owned(),
    ];
    if edited {
        args.extend([
            OsString::from("--edited"),
            OsString::from("--removed-seconds"),
            OsString::from(removed_seconds.to_string()),
        ]);
    }
    run_helper(python, &args)
}
