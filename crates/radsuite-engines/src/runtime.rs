use std::path::{Path, PathBuf};

pub(crate) fn windows_ffmpeg_path(
    local_app_data: Option<&Path>,
    command: &str,
    windows: bool,
) -> Option<PathBuf> {
    if !windows || !matches!(command, "ffmpeg" | "ffprobe") {
        return None;
    }

    let executable = format!("{command}.exe");
    Some(
        local_app_data?
            .join("RADsuite")
            .join("runtime")
            .join("ffmpeg")
            .join("bin")
            .join(executable),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::windows_ffmpeg_path;

    #[test]
    fn resolves_ffmpeg_from_the_windows_user_local_runtime() {
        let app_data = Path::new(r"C:\Users\Example\AppData\Local");
        let path = windows_ffmpeg_path(Some(app_data), "ffmpeg", true).expect("local ffmpeg path");

        assert_eq!(
            path.to_string_lossy().replace('\\', "/"),
            r"C:\Users\Example\AppData\Local/RADsuite/runtime/ffmpeg/bin/ffmpeg.exe"
                .replace('\\', "/")
        );
    }
}
