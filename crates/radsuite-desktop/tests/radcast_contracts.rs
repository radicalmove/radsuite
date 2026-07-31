use std::{
    collections::HashMap,
    env, fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use radsuite_db::migrate;
use radsuite_desktop::radcast::{
    RadcastProcessingPhase, RadcastProjectSettings, RadcastTrimRange,
    process_audio_with_processors_and_enhancement_with_progress,
    process_audio_with_processors_and_enhancement_with_progress_and_cancellation,
};
use radsuite_desktop::{
    CreateRadciteProjectRequest, DeleteRadcastAudioRequest, DesktopState,
    ImportRadcastAudioRequest, ListRadcastAudioRequest, ProcessRadcastAudioRequest,
    RadcastAudioError, RadcastJobStatus, RadcastStorageError, SaveRadcastSettingsRequest,
    cancel_radcast_audio, create_radcite_project, delete_radcast_audio,
    get_radcast_capabilities_with_processor, get_radcast_capabilities_with_processors,
    import_radcast_audio_with_processor, list_radcast_audio, list_radcite_projects,
    process_radcast_audio_with_processor, process_radcast_audio_with_processors,
    process_radcast_audio_with_processors_and_enhancement, save_radcast_settings,
};
use radsuite_engines::{
    AudioOutputFormat, AudioProcessor, CaptionFormat, CaptionProcessor, CaptionQualityMode,
    EnhancementModel, EnhancementProcessor, EnhancementQuality, FillerRemovalMode,
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
            enhancement_quality: EnhancementQuality::Standard,
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
    assert_eq!(
        default_audio
            .settings
            .trim_ranges_by_source_id
            .get(&source.id),
        Some(&RadcastTrimRange {
            clip_start_seconds: 2.0,
            clip_end_seconds: 8.0,
        })
    );

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

    let source_path = source.path.clone();
    delete_radcast_audio(
        &state,
        DeleteRadcastAudioRequest {
            project_id: Some(default_project),
            source_id: source.id.clone(),
        },
    )
    .await
    .expect("remove saved source");

    let after_delete = list_radcast_audio(
        &state,
        ListRadcastAudioRequest {
            project_id: Some(default_project),
        },
    )
    .await
    .expect("list audio after source removal");
    assert!(after_delete.sources.is_empty());
    assert_eq!(after_delete.outputs.len(), 1);
    assert!(
        !after_delete
            .settings
            .trim_ranges_by_source_id
            .contains_key(&source.id)
    );
    assert!(!Path::new(&source_path).exists());
    assert!(Path::new(&output.path).is_file());

    remove_dir(dir);
}

#[tokio::test]
async fn radcast_import_reports_when_the_selected_source_cannot_be_read() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let dir = test_dir("unreadable-source");
    let source_path = dir.join("protected.wav");
    fs::write(&source_path, b"source audio").expect("write source");
    let mut permissions = fs::metadata(&source_path)
        .expect("read source metadata")
        .permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&source_path, permissions).expect("protect source");

    let error = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: None,
        },
        AudioProcessor::default(),
    )
    .await
    .expect_err("unreadable source");

    assert!(matches!(
        error,
        RadcastAudioError::Storage(RadcastStorageError::SourceCopy { source_path: path, .. })
            if path == source_path
    ));

    let mut permissions = fs::metadata(&source_path)
        .expect("read protected source metadata")
        .permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&source_path, permissions).expect("restore source permissions");
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
            enhancement_quality: EnhancementQuality::Standard,
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
async fn radcast_processing_honours_local_cancellation_before_work_begins() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let result = process_audio_with_processors_and_enhancement_with_progress_and_cancellation(
        &state.paths.data_dir,
        projects[0].id,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: "not-needed-after-cancellation".to_string(),
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: true,
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::None,
            enhancement_quality: EnhancementQuality::Standard,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
        },
        AudioProcessor::default(),
        CaptionProcessor::default(),
        EnhancementProcessor::default(),
        |_| {},
        || true,
    );

    assert!(matches!(result, Err(RadcastStorageError::Cancelled)));
}

#[tokio::test]
async fn radcast_cancel_command_marks_a_running_local_job_for_cancellation() {
    let state = DesktopState::for_tests();
    let job_id = "cancel-me".to_string();
    state
        .radcast_jobs
        .lock()
        .expect("lock jobs")
        .insert(job_id.clone(), RadcastJobStatus::running(job_id.clone()));

    let status = cancel_radcast_audio(&state, job_id.clone()).expect("request cancellation");

    assert_eq!(status.id, job_id);
    assert!(
        state
            .radcast_cancel_requests
            .lock()
            .expect("lock cancellation requests")
            .contains("cancel-me")
    );
}

#[tokio::test]
async fn radcast_real_audio_fixture_can_process_when_available() {
    let Ok(audio_path) = env::var("RADSUITE_REAL_RADCAST_AUDIO") else {
        eprintln!("skipping real RADcast smoke test: RADSUITE_REAL_RADCAST_AUDIO is not set");
        return;
    };
    let audio_path = PathBuf::from(audio_path);
    assert!(audio_path.is_file(), "missing RADcast audio fixture");

    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: audio_path.to_string_lossy().into_owned(),
            original_filename: None,
        },
        AudioProcessor::default(),
    )
    .await
    .expect("import real audio");

    assert!(source.duration_seconds > 0.0);
    let output = process_radcast_audio_with_processor(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: source.id,
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: Some(0.0),
            clip_end_seconds: Some(10.0),
            cleanup_enabled: true,
            max_silence_seconds: Some(1.0),
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::None,
            enhancement_quality: EnhancementQuality::Standard,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
        },
        AudioProcessor::default(),
    )
    .await
    .expect("process real audio");

    assert!(output.duration_seconds > 0.0);
    assert!(output.duration_seconds <= 10.5);
    assert_eq!(output.output_format, AudioOutputFormat::Mp3);
    assert!(output.cleanup_enabled);
    assert_eq!(output.max_silence_seconds, Some(1.0));
    assert!(Path::new(&output.path).is_file());
}

#[tokio::test]
async fn radcast_real_audio_fixture_can_process_with_the_optimized_profile_when_available() {
    let Ok(audio_path) = env::var("RADSUITE_REAL_RADCAST_OPTIMIZED_AUDIO") else {
        eprintln!(
            "skipping real RADcast optimized smoke test: RADSUITE_REAL_RADCAST_OPTIMIZED_AUDIO is not set"
        );
        return;
    };
    let audio_path = PathBuf::from(audio_path);
    assert!(
        audio_path.is_file(),
        "missing RADcast optimized audio fixture"
    );

    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: audio_path.to_string_lossy().into_owned(),
            original_filename: Some("optimized-lecture.wav".to_string()),
        },
        AudioProcessor::default(),
    )
    .await
    .expect("import optimized audio");

    let output = process_radcast_audio_with_processors_and_enhancement(
        &state,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: source.id,
            output_format: AudioOutputFormat::Wav,
            clip_start_seconds: Some(0.0),
            clip_end_seconds: Some(8.0),
            cleanup_enabled: true,
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::StudioV18,
            enhancement_quality: EnhancementQuality::Fast,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Normal,
        },
        AudioProcessor::default(),
        CaptionProcessor::default(),
        EnhancementProcessor::default(),
    )
    .await
    .expect("process optimized audio");

    assert_eq!(output.enhancement_model, EnhancementModel::StudioV18);
    assert_eq!(output.enhancement_quality, EnhancementQuality::Fast);
    assert!(output.duration_seconds > 0.0);
    assert!(output.duration_seconds <= 8.5);
    assert!(Path::new(&output.path).is_file());
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
            enhancement_quality: EnhancementQuality::Standard,
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
    assert!(!output.caption_review_required);
    assert_eq!(output.caption_average_probability, Some(0.82));
    assert_eq!(output.caption_low_confidence_segments, 0);
    assert_eq!(output.caption_total_segments, 1);
    assert!(output.caption_review_path.is_none());
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
            enhancement_quality: EnhancementQuality::Fast,
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
    assert_eq!(output.enhancement_quality, EnhancementQuality::Fast);
    assert!(Path::new(&output.path).is_file());
    remove_dir(dir);
}

#[tokio::test]
async fn radcast_real_audio_fixture_can_process_with_each_legacy_profile_when_available() {
    let Ok(raw_path) = env::var("RADSUITE_REAL_RADCAST_LEGACY_AUDIO") else {
        eprintln!(
            "skipping real RADcast legacy profile smoke test: RADSUITE_REAL_RADCAST_LEGACY_AUDIO is not set"
        );
        return;
    };
    let source_path = PathBuf::from(raw_path);
    assert!(
        source_path.is_file(),
        "missing RADcast legacy profile fixture"
    );

    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let dir = test_dir("legacy-profiles");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: Some("legacy-profile-fixture.wav".to_string()),
        },
        AudioProcessor::default(),
    )
    .await
    .expect("import legacy profile fixture");

    for model in [
        EnhancementModel::Resemble,
        EnhancementModel::DeepFilterNet,
        EnhancementModel::Studio,
    ] {
        let output = process_radcast_audio_with_processors_and_enhancement(
            &state,
            ProcessRadcastAudioRequest {
                project_id: Some(projects[0].id),
                source_id: source.id.clone(),
                output_format: AudioOutputFormat::Wav,
                clip_start_seconds: None,
                clip_end_seconds: None,
                cleanup_enabled: false,
                max_silence_seconds: None,
                caption_format: None,
                caption_language: "en".to_string(),
                caption_quality_mode: CaptionQualityMode::Reviewed,
                caption_glossary: None,
                enhancement_model: model,
                enhancement_quality: EnhancementQuality::Fast,
                remove_filler_words: false,
                filler_removal_mode: FillerRemovalMode::Aggressive,
            },
            AudioProcessor::default(),
            CaptionProcessor::default(),
            EnhancementProcessor::default(),
        )
        .await
        .unwrap_or_else(|error| panic!("{model:?} profile failed: {error}"));

        assert_eq!(output.enhancement_model, model);
        assert!(Path::new(&output.path).is_file());
    }

    remove_dir(dir);
}

#[tokio::test]
async fn radcast_processing_reports_ordered_local_progress_phases() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let dir = test_dir("progress");
    let source_path = dir.join("lecture.wav");
    fs::write(&source_path, b"source audio").expect("write source");
    let source = import_radcast_audio_with_processor(
        &state,
        ImportRadcastAudioRequest {
            project_id: Some(projects[0].id),
            path: source_path.to_string_lossy().into_owned(),
            original_filename: Some("progress-lecture.wav".to_string()),
        },
        fake_processor(&dir),
    )
    .await
    .expect("import source");

    let mut phases = Vec::new();
    process_audio_with_processors_and_enhancement_with_progress(
        &state.paths.data_dir,
        projects[0].id,
        ProcessRadcastAudioRequest {
            project_id: Some(projects[0].id),
            source_id: source.id,
            output_format: AudioOutputFormat::Mp3,
            clip_start_seconds: None,
            clip_end_seconds: None,
            cleanup_enabled: false,
            max_silence_seconds: None,
            caption_format: None,
            caption_language: "en".to_string(),
            caption_quality_mode: CaptionQualityMode::Reviewed,
            caption_glossary: None,
            enhancement_model: EnhancementModel::None,
            enhancement_quality: EnhancementQuality::Standard,
            remove_filler_words: false,
            filler_removal_mode: FillerRemovalMode::Aggressive,
        },
        fake_processor(&dir),
        CaptionProcessor::default(),
        EnhancementProcessor::default(),
        |progress| phases.push(progress.phase),
    )
    .expect("process source");

    assert_eq!(
        phases,
        vec![
            RadcastProcessingPhase::Preparing,
            RadcastProcessingPhase::RenderingAudio,
            RadcastProcessingPhase::SavingOutput,
        ]
    );
    remove_dir(dir);
}

#[tokio::test]
async fn radcast_project_settings_are_persisted_in_local_project_storage() {
    let state = desktop_state_with_migrated_pool().await;
    let projects = list_radcite_projects(&state).await.expect("list projects");
    let settings = RadcastProjectSettings {
        output_format: AudioOutputFormat::Wav,
        caption_format: Some(CaptionFormat::Vtt),
        caption_language: "mi".to_string(),
        caption_quality_mode: CaptionQualityMode::Accurate,
        caption_glossary: Some("Te Tiriti".to_string()),
        enhancement_model: EnhancementModel::None,
        enhancement_quality: EnhancementQuality::Fast,
        cleanup_enabled: false,
        max_silence_seconds: Some(1.5),
        remove_filler_words: true,
        filler_removal_mode: FillerRemovalMode::Normal,
        trim_ranges_by_source_id: HashMap::from([(
            "source-lecture".to_string(),
            RadcastTrimRange {
                clip_start_seconds: 12.5,
                clip_end_seconds: 98.25,
            },
        )]),
    };

    save_radcast_settings(
        &state,
        SaveRadcastSettingsRequest {
            project_id: Some(projects[0].id),
            settings: settings.clone(),
        },
    )
    .await
    .expect("save RADcast settings");

    let listing = list_radcast_audio(
        &state,
        ListRadcastAudioRequest {
            project_id: Some(projects[0].id),
        },
    )
    .await
    .expect("list RADcast settings");
    assert_eq!(listing.settings, settings);
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
    assert_eq!(both_ready.enhancement_models.len(), 5);
    assert!(
        both_ready
            .enhancement_models
            .iter()
            .any(|model| model.id == EnhancementModel::StudioV18 && model.available)
    );

    let unavailable = get_radcast_capabilities_with_processor(CaptionProcessor::from_commands(
        whisper,
        dir.join("missing.bin"),
    ));
    assert!(!unavailable.caption_available);
    assert!(unavailable.caption_detail.contains("model"));
    remove_dir(dir);
}

#[test]
fn radcast_capabilities_explain_each_local_enhancement_backend() {
    let dir = test_dir("backend-capabilities");
    let resemble = write_executable(&dir, "resemble.sh", "#!/bin/sh\nexit 0\n");
    let deepfilternet = write_executable(&dir, "deepfilter.sh", "#!/bin/sh\nexit 0\n");
    let studio = write_executable(&dir, "studio.sh", "#!/bin/sh\nexit 0\n");
    let optimized = write_executable(&dir, "optimized.sh", "#!/bin/sh\nexit 0\n");
    let processor = EnhancementProcessor::from_commands(resemble, deepfilternet, studio, optimized);

    let capabilities = get_radcast_capabilities_with_processors(
        CaptionProcessor::from_commands(dir.join("missing-whisper"), dir.join("missing-model")),
        processor,
    );

    assert!(
        capabilities
            .enhancement_models
            .iter()
            .all(|model| model.available || model.id == EnhancementModel::None)
    );
    assert!(
        capabilities
            .enhancement_models
            .iter()
            .all(|model| !model.detail.trim().is_empty())
    );
    remove_dir(dir);
}

#[test]
fn radcast_defaults_to_the_original_optimized_quality_profile() {
    assert_eq!(
        RadcastProjectSettings::default().enhancement_model,
        EnhancementModel::StudioV18
    );
    assert_eq!(
        RadcastProjectSettings::default().enhancement_quality,
        EnhancementQuality::High
    );
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
        "#!/bin/sh\noutput=''\njson=0\nprevious=''\nfor arg in \"$@\"; do\n  if [ \"$previous\" = \"-of\" ]; then output=\"$arg\"; fi\n  if [ \"$arg\" = \"-oj\" ]; then json=1; fi\n  previous=\"$arg\"\ndone\nprintf '1\\n00:00:00,000 --> 00:00:01,000\\nHello\\n' > \"$output.srt\"\nif [ \"$json\" = \"1\" ]; then printf '{\"transcription\":[{\"tokens\":[{\"text\":\" um\",\"offsets\":{\"from\":250,\"to\":450},\"p\":0.82}]}]}' > \"$output.json\"; fi\n",
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
