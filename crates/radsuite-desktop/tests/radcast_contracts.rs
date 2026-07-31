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
    get_radcast_capabilities_with_processor, get_radcast_capabilities_with_processors,
    import_radcast_audio_with_processor, list_radcast_audio, list_radcite_projects,
    process_radcast_audio_with_processor, process_radcast_audio_with_processors,
    process_radcast_audio_with_processors_and_enhancement,
};
use radsuite_engines::{
    AudioOutputFormat, AudioProcessor, CaptionFormat, CaptionProcessor, CaptionQualityMode,
    EnhancementModel, EnhancementProcessor, FillerRemovalMode,
};
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
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::None,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
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
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::None,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
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

#[tokio::test]
async fn radcast_processing_keeps_generated_captions_with_the_audio_output() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let dir = test_dir("captions");
    let source_path = dir.join("lecture.wav");
    fs::write(&source_path, b"source audio").expect("write source");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: Some("captioned-lecture.wav".to_string()),
        },
        fake_processor(&dir),
    )
    .await
    .expect("import source");

    let output = process_radcast_audio_with_processors(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: source.id,
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: false,
            max_silence_seconds: None,
            caption_format: Some(CaptionFormat::Srt),
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: Some("Te Tiriti o Waitangi".to_string()),
            enhancement_model: EnhancementModel::None,
            remove_filler_words: true,
            filler_removal_mode: FillerRemovalMode::Aggressive,
        },
        fake_processor(&dir),
        fake_caption_processor(&dir),
    )
    .await
    .expect("process with captions");

    assert_eq!(output.caption_format, Some(CaptionFormat::Srt));
    assert_eq!(output.caption_quality_mode, CaptionQualityMode::Reviewed);
    assert_eq!(
        output.caption_glossary.as_deref(),
        Some("Te Tiriti o Waitangi")
    );
    assert_eq!(output.caption_segment_count, 1);
    assert_eq!(output.removed_filler_count, 1);
    let caption_path = output.caption_path.as_deref().expect("caption path");
    assert!(caption_path.ends_with(".srt"));
    assert!(Path::new(caption_path).is_file());

    let listing = list_radcast_audio(
        &state,
        ListRadcastAudioRequest {
            project_id: Some(projects[0].id),
        },
    )
    .await
    .expect("list captioned audio");
    assert_eq!(listing.outputs[0].caption_path, output.caption_path);
    remove_dir(dir);
}

#[tokio::test]
async fn radcast_processing_can_apply_the_optimized_local_enhancement_profile() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let dir = test_dir("enhancement");
    let source_path = dir.join("lecture.wav");
    fs::write(&source_path, b"source audio").expect("write source");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: Some("enhanced-lecture.wav".to_string()),
        },
        fake_processor(&dir),
    )
    .await
    .expect("import source");

    let helper = write_executable(
        &dir,
        "studio.sh",
        "#!/bin/sh\nmkdir -p \"$2\"\nprintf '%s' enhanced > \"$2/input.wav\"\n",
    );
    let output = process_radcast_audio_with_processors_and_enhancement(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: source.id,
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: Some(1.0),
            clip_end_seconds: Some(8.0),
            cleanup_enabled: true,
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::StudioV18,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
        },
        fake_processor(&dir),
        CaptionProcessor::default(),
        EnhancementProcessor::from_command(helper),
    )
    .await
    .expect("process with enhancement");

    assert_eq!(output.enhancement_model, EnhancementModel::StudioV18);
    assert!(Path::new(&output.path).is_file());
    remove_dir(dir);
}

#[test]
fn radcast_capabilities_report_caption_model_readiness() {
    let dir = test_dir("capabilities");
    let whisper = write_executable(&dir, "whisper.sh", "#!/bin/sh\nexit 0\n");
    let model = dir.join("caption-model.bin");
    fs::write(&model, b"model").expect("write caption model");
    let studio = write_executable(&dir, "studio.sh", "#!/bin/sh\nexit 0\n");

    let ready = get_radcast_capabilities_with_processor(CaptionProcessor::from_commands(
        whisper.clone(),
        model.clone(),
    ));
    assert!(ready.caption_available);
    assert!(ready.caption_detail.contains("whisper.cpp"));

    let both_ready = get_radcast_capabilities_with_processors(
        CaptionProcessor::from_commands(whisper.clone(), model.clone()),
        EnhancementProcessor::from_command(studio),
    );
    assert!(both_ready.optimized_available);
    assert!(both_ready.optimized_detail.contains("local"));
    assert!(both_ready.optimized_detail.contains("server"));

    let unavailable = get_radcast_capabilities_with_processor(CaptionProcessor::from_commands(
        whisper,
        dir.join("missing.bin"),
    ));
    assert!(!unavailable.caption_available);
    assert!(unavailable.caption_detail.contains("model"));
    remove_dir(dir);
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

fn fake_caption_processor(dir: &Path) -> CaptionProcessor {
    let whisper = write_executable(
        dir,
        "whisper.sh",
        "#!/bin/sh\noutput=''\njson=0\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  if [ \"$arg\" = \"-oj\" ]; then json=1; fi\n  previous=\"$arg\"\ndone\nif [ \"$json\" = \"1\" ]; then printf '{\"transcription\":[{\"tokens\":[{\"text\":\" um\",\"offsets\":{\"from\":250,\"to\":450},\"p\":0.82}]}]}' > \"$output.json\"; else printf '1\\n00:00:00,000 --> 00:00:01,000\\nHello\\n' > \"$output.srt\"; fi\n",
    );
    let model = dir.join("caption-model.bin");
    fs::write(&model, b"model").expect("write caption model");
    CaptionProcessor::from_commands(whisper, model)
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
