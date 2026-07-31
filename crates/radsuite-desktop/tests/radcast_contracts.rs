use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_db::migrate;
use radsuite_desktop::{
    CreateRadciteProjectRequest, DesktopState, ImportRadcastAudioRequest, ListRadcastAudioRequest,
    ProcessRadcastAudioRequest, RadcastAudioError, RadcastStorageError, create_radcite_project,
    import_radcast_audio_with_processor, list_radcast_audio, list_radcite_projects,
    process_radcast_audio_with_processor,
};
use radsuite_engines::{AudioOutputFormat, AudioProcessor};
use sqlx::sqlite::SqlitePoolOptions;

#[tokio::test]
async fn radcast_import_process_and_list_are_project_scoped() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let default_project = projects[0].id;
    let second_project = create_radcite_project(
        &state,
        CreateRadciteProjectRequest {
            code: Some("RAD201".to_string()),
            title: "Audio production".to_string(),
        },
    )
    .await
    .expect("create second project");

    let dir = test_dir("project-scope");
    let source_path = dir.join("lecture.wav");
    fs::write(&source_path, b"source audio").expect("write source");
    let processor = fake_processor(&dir);

    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(default_project),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: Some("week-one.wav".to_string()),
        },
        processor.clone(),
    )
    .await
    .expect("import source");

    assert_eq!(source.original_filename, "week-one.wav");
    assert_eq!(source.duration_seconds, 12.5);
    assert!(Path::new(&source.path).is_file());

    let output = process_radcast_audio_with_processor(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(default_project),
            source_id: source.id.clone(),
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: Some(2.0),
            clip_end_seconds: Some(8.0),
            cleanup_enabled: true,
        },
        processor,
    )
    .await
    .expect("process source");

    assert_eq!(output.source_id, source.id);
    assert_eq!(output.duration_seconds, 12.5);
    assert!(output.path.ends_with(".mp3"));
    assert!(Path::new(&output.path).is_file());

    let default_audio = list_radcast_audio(
        &state,
        ListRadcastAudioRequest {
            project_id: Some(default_project),
        },
    )
    .await
    .expect("list default project audio");
    assert_eq!(default_audio.sources.len(), 1);
    assert_eq!(default_audio.outputs.len(), 1);

    let other_audio = list_radcast_audio(
        &state,
        ListRadcastAudioRequest {
            project_id: Some(second_project.id),
        },
    )
    .await
    .expect("list second project audio");
    assert!(other_audio.sources.is_empty());
    assert!(other_audio.outputs.is_empty());

    remove_dir(dir);
}

#[tokio::test]
async fn radcast_processing_rejects_unknown_sources() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");

    let error = process_radcast_audio_with_processor(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: "missing-source".to_string(),
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: false,
        },
        AudioProcessor::default(),
    )
    .await
    .expect_err("unknown source");

    assert!(matches!(
        error,
        RadcastAudioError::Storage(RadcastStorageError::MissingSource(_))
    ));
}

async fn desktop_state_with_migrated_pool() -> DesktopState {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect test database");
    migrate(&pool).await.expect("migrate test database");
    DesktopState::for_tests_with_pool(pool)
}

fn fake_processor(dir: &Path) -> AudioProcessor {
    let ffmpeg = write_executable(
        dir,
        "ffmpeg.sh",
        "#!/bin/sh\noutput=''\nfor arg in \"$@\"; do output=\"$arg\"; done\nmkdir -p \"$(dirname \"$output\")\"\nprintf 'fake audio' > \"$output\"\n",
    );
    let ffprobe = write_executable(dir, "ffprobe.sh", "#!/bin/sh\nprintf '12.5\\n'");
    AudioProcessor::from_commands(ffmpeg, ffprobe)
}

fn write_executable(dir: &Path, filename: &str, contents: &str) -> PathBuf {
    let path = dir.join(filename);
    fs::write(&path, contents).expect("write fake tool");
    let mut permissions = fs::metadata(&path)
        .expect("read tool metadata")
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
    let path = std::env::temp_dir().join(format!("radsuite-radcast-desktop-{label}-{suffix}"));
    fs::create_dir_all(&path).expect("create test directory");
    path
}

fn remove_dir(path: PathBuf) {
    let _ = fs::remove_dir_all(path);
}
