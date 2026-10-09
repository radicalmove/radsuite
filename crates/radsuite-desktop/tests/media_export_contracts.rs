use radsuite_desktop::{
    MediaOutputFormat,
    media_assets::{MediaAssetError, ProjectMediaStore},
    radcast::{ProcessRadcastAudioRequest, RadcastAudioOutput, RadcastProjectSettings},
    radt_ts::{RadtTsAudioOutput, RadtTsOutputFormat, StartRadtTsSynthesisRequest},
    radt_ts_tools::{
        RadtTsMediaArtifact, RadtTsMediaJobKind, RadtTsMediaOutput, StartRadtTsClipRequest,
    },
};
use radsuite_engines::AudioOutputFormat;
use serde_json::{Value, json};
use std::fs;
use uuid::Uuid;

fn legacy_output_format(media_format: MediaOutputFormat) -> &'static str {
    match media_format {
        MediaOutputFormat::Mp3 => "mp3",
        MediaOutputFormat::Wav | MediaOutputFormat::Mp4 => "wav",
    }
}

#[test]
fn media_format_serializes_with_snake_case_and_maps_audio_formats() {
    let cases = [
        (
            MediaOutputFormat::Mp3,
            "mp3",
            "mp3",
            true,
            AudioOutputFormat::Mp3,
        ),
        (
            MediaOutputFormat::Wav,
            "wav",
            "wav",
            true,
            AudioOutputFormat::Wav,
        ),
        (
            MediaOutputFormat::Mp4,
            "mp4",
            "mp4",
            false,
            AudioOutputFormat::Wav,
        ),
    ];

    for (format, json_name, extension, is_audio, audio_format) in cases {
        assert_eq!(
            serde_json::to_string(&format).unwrap(),
            format!("\"{json_name}\"")
        );
        assert_eq!(format.extension(), extension);
        assert_eq!(format.is_audio(), is_audio);
        assert_eq!(format.audio_format(), audio_format);
    }

    assert_eq!(
        serde_json::from_str::<MediaOutputFormat>(r#""mp3""#).unwrap(),
        MediaOutputFormat::Mp3
    );
    assert_eq!(
        serde_json::from_str::<MediaOutputFormat>(r#""wav""#).unwrap(),
        MediaOutputFormat::Wav
    );
}

#[test]
fn media_format_normalizer_prefers_media_then_legacy_then_mp3() {
    assert_eq!(
        MediaOutputFormat::from_request(Some(MediaOutputFormat::Mp4), Some(AudioOutputFormat::Mp3),),
        MediaOutputFormat::Mp4
    );
    assert_eq!(
        MediaOutputFormat::from_request(None, Some(AudioOutputFormat::Wav)),
        MediaOutputFormat::Wav
    );
    assert_eq!(
        MediaOutputFormat::from_request(None, None::<AudioOutputFormat>),
        MediaOutputFormat::Mp3
    );

    assert_eq!(
        MediaOutputFormat::from_request(None, Some(RadtTsOutputFormat::Wav)),
        MediaOutputFormat::Wav
    );
    assert_eq!(
        MediaOutputFormat::from_request(None, None::<RadtTsOutputFormat>),
        MediaOutputFormat::Mp3
    );
    assert_eq!(
        RadtTsOutputFormat::from(MediaOutputFormat::Mp4),
        RadtTsOutputFormat::Wav
    );
}

#[test]
fn all_workflows_normalize_mp3_wav_and_mp4_without_widening_audio_enums() {
    let formats = [
        MediaOutputFormat::Mp3,
        MediaOutputFormat::Wav,
        MediaOutputFormat::Mp4,
    ];

    for media_format in formats {
        let legacy = legacy_output_format(media_format);
        let radcast: ProcessRadcastAudioRequest = serde_json::from_value(json!({
            "source_id": "source-1",
            "media_format": media_format,
            "output_format": legacy,
            "clip_start_seconds": null,
            "clip_end_seconds": null,
            "cleanup_enabled": true,
        }))
        .unwrap();
        assert_eq!(radcast.normalized_media_format(), media_format);

        let voice: StartRadtTsSynthesisRequest = serde_json::from_value(json!({
            "text": "A short script.",
            "quality": "high",
            "chunk_mode": "sentence",
            "pause_min_seconds": 0.25,
            "pause_max_seconds": 0.75,
            "media_format": media_format,
            "output_format": legacy,
            "output_name": "intro",
            "acknowledge_voice_clone": true,
        }))
        .unwrap();
        assert_eq!(voice.normalized_media_format(), media_format);
        let voice_wire = serde_json::to_value(&voice).unwrap();
        assert_eq!(
            voice_wire["media_format"],
            serde_json::to_value(media_format).unwrap()
        );
        assert_eq!(voice_wire["output_format"], Value::from(legacy));

        let clip: StartRadtTsClipRequest = serde_json::from_value(json!({
            "audio_path": "/tmp/lecture.mp3",
            "segments_json_path": "/tmp/lecture.segments.json",
            "output_name": "clip",
            "start_time": 0.0,
            "end_time": 1.0,
            "verification_mode": "strict",
            "media_format": media_format,
            "output_format": legacy,
        }))
        .unwrap();
        assert_eq!(clip.normalized_media_format(), media_format);
        let clip_wire = serde_json::to_value(&clip).unwrap();
        assert_eq!(
            clip_wire["media_format"],
            serde_json::to_value(media_format).unwrap()
        );
        assert_eq!(clip_wire["output_format"], Value::from(legacy));
    }
}

#[test]
fn legacy_requests_and_settings_fall_back_to_output_format() {
    let radcast: ProcessRadcastAudioRequest = serde_json::from_value(json!({
        "source_id": "source-1",
        "output_format": "wav",
        "clip_start_seconds": null,
        "clip_end_seconds": null,
        "cleanup_enabled": true,
    }))
    .unwrap();
    assert_eq!(radcast.media_format, None);
    assert_eq!(radcast.normalized_media_format(), MediaOutputFormat::Wav);

    let settings: RadcastProjectSettings = serde_json::from_value(json!({
        "output_format": "wav"
    }))
    .unwrap();
    assert_eq!(settings.media_format, None);
    assert_eq!(settings.normalized_media_format(), MediaOutputFormat::Wav);

    let voice: StartRadtTsSynthesisRequest = serde_json::from_value(json!({
        "text": "A short script.",
        "quality": "high",
        "chunk_mode": "sentence",
        "pause_min_seconds": 0.25,
        "pause_max_seconds": 0.75,
        "output_format": "wav",
        "output_name": "intro",
        "acknowledge_voice_clone": true,
    }))
    .unwrap();
    assert_eq!(voice.normalized_media_format(), MediaOutputFormat::Wav);

    let clip: StartRadtTsClipRequest = serde_json::from_value(json!({
        "audio_path": "/tmp/lecture.mp3",
        "segments_json_path": "/tmp/lecture.segments.json",
        "output_name": "clip",
        "start_time": 0.0,
        "end_time": 1.0,
        "verification_mode": "strict",
        "output_format": "wav"
    }))
    .unwrap();
    assert_eq!(clip.media_format, None);
    assert_eq!(clip.normalized_media_format(), MediaOutputFormat::Wav);
}

#[test]
fn new_wire_values_emit_both_fields_with_wav_placeholder_for_mp4() {
    let request: ProcessRadcastAudioRequest = serde_json::from_value(json!({
        "source_id": "source-1",
        "media_format": "mp4",
        "output_format": "wav",
        "clip_start_seconds": null,
        "clip_end_seconds": null,
        "cleanup_enabled": true,
    }))
    .unwrap();
    let serialized = serde_json::to_value(&request).unwrap();
    assert_eq!(serialized["media_format"], Value::from("mp4"));
    assert_eq!(serialized["output_format"], Value::from("wav"));

    let settings = RadcastProjectSettings::from_request(&request);
    assert_eq!(settings.media_format, Some(MediaOutputFormat::Mp4));
    assert_eq!(settings.output_format, AudioOutputFormat::Wav);

    let settings = RadcastProjectSettings {
        media_format: Some(MediaOutputFormat::Mp4),
        output_format: AudioOutputFormat::Wav,
        ..RadcastProjectSettings::default()
    };
    let serialized = serde_json::to_value(&settings).unwrap();
    assert_eq!(serialized["media_format"], Value::from("mp4"));
    assert_eq!(serialized["output_format"], Value::from("wav"));
}

#[test]
fn radcast_mp4_request_and_output_preserve_presenter_image_intent() {
    let request: ProcessRadcastAudioRequest = serde_json::from_value(json!({
        "source_id": "source-1",
        "media_format": "mp4",
        "output_format": "wav",
        "presenter_image_path": "/managed/staged-image.png",
        "save_presenter_image_as_project_default": true,
        "clip_start_seconds": null,
        "clip_end_seconds": null,
        "cleanup_enabled": true
    }))
    .unwrap();
    assert_eq!(
        request.presenter_image_path.as_deref(),
        Some("/managed/staged-image.png")
    );
    assert!(request.save_presenter_image_as_project_default);

    let output: RadcastAudioOutput = serde_json::from_value(json!({
        "id": "output-1",
        "source_id": "source-1",
        "filename": "lesson.mp4",
        "path": "/managed/lesson.mp4",
        "duration_seconds": 12.5,
        "output_format": "wav",
        "media_format": "mp4",
        "image_path": "/managed/cover/cover.png",
        "cleanup_enabled": false,
        "clip_start_seconds": null,
        "clip_end_seconds": null,
        "created_at": "2026-08-17T00:00:00Z"
    }))
    .unwrap();
    assert_eq!(
        output.image_path.as_deref(),
        Some("/managed/cover/cover.png")
    );
}

#[test]
fn radtts_mp4_request_and_output_preserve_presenter_image_intent() {
    let request: StartRadtTsSynthesisRequest = serde_json::from_value(json!({
        "text": "A short script.",
        "quality": "high",
        "chunk_mode": "sentence",
        "pause_min_seconds": 0.25,
        "pause_max_seconds": 0.75,
        "media_format": "mp4",
        "output_format": "wav",
        "presenter_image_path": "/managed/staged-image.png",
        "save_presenter_image_as_project_default": true,
        "output_name": "intro",
        "acknowledge_voice_clone": true
    }))
    .unwrap();
    assert_eq!(
        request.presenter_image_path.as_deref(),
        Some("/managed/staged-image.png")
    );
    assert!(request.save_presenter_image_as_project_default);

    let output: RadtTsAudioOutput = serde_json::from_value(json!({
        "id": "voice-1",
        "filename": "voice.mp4",
        "path": "/managed/voice.mp4",
        "output_format": "wav",
        "media_format": "mp4",
        "image_path": "/managed/cover/cover.png",
        "caption_paths": [],
        "duration_seconds": 4.0,
        "created_at": null
    }))
    .unwrap();
    assert_eq!(
        output.image_path.as_deref(),
        Some("/managed/cover/cover.png")
    );
}

#[test]
fn radtts_clip_mp4_contract_preserves_presenter_image_intent() {
    let request: StartRadtTsClipRequest = serde_json::from_value(json!({
        "audio_path": "/tmp/lecture.wav",
        "segments_json_path": "/tmp/lecture.segments.json",
        "output_name": "clip",
        "start_time": 0.0,
        "end_time": 4.0,
        "verification_mode": "strict",
        "output_format": "wav",
        "media_format": "mp4",
        "presenter_image_path": "/managed/presenter.png",
        "save_presenter_image_as_project_default": false
    }))
    .unwrap();
    assert_eq!(
        request.presenter_image_path.as_deref(),
        Some("/managed/presenter.png")
    );
    assert!(!request.save_presenter_image_as_project_default);
}

#[test]
fn legacy_output_records_remain_readable_and_normalize_to_audio_formats() {
    let radcast: RadcastAudioOutput = serde_json::from_value(json!({
        "id": "output-1",
        "source_id": "source-1",
        "filename": "lesson.wav",
        "path": "/tmp/lesson.wav",
        "duration_seconds": 12.5,
        "output_format": "wav",
        "cleanup_enabled": false,
        "clip_start_seconds": null,
        "clip_end_seconds": null,
        "created_at": "2026-08-17T00:00:00Z"
    }))
    .unwrap();
    assert_eq!(radcast.media_format, None);
    assert_eq!(radcast.normalized_media_format(), MediaOutputFormat::Wav);

    let voice: RadtTsAudioOutput = serde_json::from_value(json!({
        "id": "job-1",
        "filename": "voice.mp3",
        "path": "/tmp/voice.mp3",
        "output_format": "mp3",
        "caption_paths": [],
        "duration_seconds": 4.0,
        "created_at": null
    }))
    .unwrap();
    assert_eq!(voice.normalized_media_format(), MediaOutputFormat::Mp3);

    let clip: RadtTsMediaOutput = serde_json::from_value(json!({
        "id": "clip-1",
        "kind": "clip",
        "name": "opening",
        "primary_path": "/tmp/opening.wav",
        "artifacts": [{"label": "Boundary report", "path": "/tmp/opening.json"}],
        "output_format": "wav",
        "warnings": []
    }))
    .unwrap();
    assert_eq!(clip.normalized_media_format(), MediaOutputFormat::Wav);
    assert_eq!(clip.kind, RadtTsMediaJobKind::Clip);
    assert_eq!(clip.artifacts.len(), 1);

    let new_clip = RadtTsMediaOutput {
        id: "clip-2".to_string(),
        kind: RadtTsMediaJobKind::Clip,
        name: "video-clip".to_string(),
        primary_path: "/tmp/video-clip.mp4".to_string(),
        artifacts: vec![RadtTsMediaArtifact {
            label: "Boundary report".to_string(),
            path: "/tmp/video-clip.json".to_string(),
        }],
        output_format: Some(RadtTsOutputFormat::Wav),
        media_format: Some(MediaOutputFormat::Mp4),
        image_path: None,
        warnings: Vec::new(),
    };
    let serialized = serde_json::to_value(new_clip).unwrap();
    assert_eq!(serialized["media_format"], Value::from("mp4"));
    assert_eq!(serialized["output_format"], Value::from("wav"));
}

#[test]
fn all_workflow_output_records_preserve_requested_media_format() {
    for media_format in [
        MediaOutputFormat::Mp3,
        MediaOutputFormat::Wav,
        MediaOutputFormat::Mp4,
    ] {
        let legacy = legacy_output_format(media_format);
        let radcast: RadcastAudioOutput = serde_json::from_value(json!({
            "id": "radcast-output",
            "source_id": "source-1",
            "filename": "lesson.wav",
            "path": "/tmp/lesson.wav",
            "duration_seconds": 12.5,
            "output_format": legacy,
            "media_format": media_format,
            "cleanup_enabled": false,
            "clip_start_seconds": null,
            "clip_end_seconds": null,
            "created_at": "2026-08-17T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(radcast.normalized_media_format(), media_format);

        let voice = RadtTsAudioOutput {
            id: "voice-output".to_string(),
            filename: "voice.wav".to_string(),
            path: "/tmp/voice.wav".to_string(),
            output_format: match media_format {
                MediaOutputFormat::Mp3 => RadtTsOutputFormat::Mp3,
                MediaOutputFormat::Wav | MediaOutputFormat::Mp4 => RadtTsOutputFormat::Wav,
            },
            media_format: Some(media_format),
            image_path: None,
            caption_paths: Vec::new(),
            duration_seconds: Some(4.0),
            created_at: None,
        };
        assert_eq!(voice.normalized_media_format(), media_format);

        let clip = RadtTsMediaOutput {
            id: "clip-output".to_string(),
            kind: RadtTsMediaJobKind::Clip,
            name: "opening".to_string(),
            primary_path: "/tmp/opening.wav".to_string(),
            artifacts: Vec::new(),
            output_format: Some(voice.output_format),
            media_format: Some(media_format),
            image_path: None,
            warnings: Vec::new(),
        };
        assert_eq!(clip.normalized_media_format(), media_format);

        for serialized in [
            serde_json::to_value(&radcast).unwrap(),
            serde_json::to_value(&voice).unwrap(),
            serde_json::to_value(&clip).unwrap(),
        ] {
            assert_eq!(
                serialized["media_format"],
                serde_json::to_value(media_format).unwrap()
            );
            assert_eq!(serialized["output_format"], Value::from(legacy));
        }
    }
}

#[test]
fn media_assets_stage_supported_presenter_images_inside_project_storage() {
    let root = test_root("stage");
    let source = root.join("presenter.png");
    fs::create_dir_all(&root).unwrap();
    fs::write(&source, b"\x89PNG\r\n\x1a\n").unwrap();
    let store = ProjectMediaStore::new(root.join("data"));

    let staged = store.stage_image("course-1", &source).unwrap();

    assert!(staged.path().is_file());
    assert!(
        staged
            .path()
            .starts_with(root.join("data/media/projects/course-1"))
    );
    assert_eq!(staged.extension(), "png");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_assets_reject_unsupported_and_animated_images() {
    let root = test_root("reject");
    fs::create_dir_all(&root).unwrap();
    let store = ProjectMediaStore::new(root.join("data"));
    let gif = root.join("presenter.gif");
    fs::write(&gif, b"GIF89a").unwrap();
    assert!(matches!(
        store.stage_image("course-1", &gif),
        Err(MediaAssetError::UnsupportedImage { .. })
    ));

    let webp = root.join("animated.webp");
    fs::write(&webp, b"RIFF\x10\0\0\0WEBPVP8XANIM").unwrap();
    assert!(matches!(
        store.stage_image("course-1", &webp),
        Err(MediaAssetError::AnimatedWebp { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_assets_roll_back_and_finalize_project_cover_replacement() {
    let root = test_root("cover-transaction");
    fs::create_dir_all(&root).unwrap();
    let png = root.join("first.png");
    let jpeg = root.join("replacement.jpg");
    fs::write(&png, b"\x89PNG\r\n\x1a\nfirst").unwrap();
    fs::write(&jpeg, b"\xff\xd8\xffreplacement").unwrap();
    let store = ProjectMediaStore::new(root.join("data"));

    let first = store.stage_image("course-1", &png).unwrap();
    let first_pending = store.prepare_commit(first, Uuid::new_v4(), true).unwrap();
    let first_saved = store.finalize_commit(first_pending).unwrap();
    assert_eq!(first_saved.extension(), "png");

    let replacement = store.stage_image("course-1", &jpeg).unwrap();
    let pending = store
        .prepare_commit(replacement, Uuid::new_v4(), true)
        .unwrap();
    assert_eq!(
        store.saved_cover("course-1").unwrap().unwrap().extension(),
        "jpg"
    );
    store.rollback_commit(pending).unwrap();
    assert_eq!(
        store.saved_cover("course-1").unwrap().unwrap().extension(),
        "png"
    );

    let replacement = store.stage_image("course-1", &jpeg).unwrap();
    let pending = store
        .prepare_commit(replacement, Uuid::new_v4(), true)
        .unwrap();
    let saved = store.finalize_commit(pending).unwrap();
    assert_eq!(saved.extension(), "jpg");
    assert!(!saved.path().with_file_name("cover.png").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_assets_persist_and_recover_orphan_cleanup_paths() {
    let root = test_root("orphan-recovery");
    let data = root.join("data");
    let orphan = data.join("media/projects/course-1/exports/output-1.partial.mp4");
    fs::create_dir_all(orphan.parent().unwrap()).unwrap();
    fs::write(&orphan, b"partial").unwrap();
    let store = ProjectMediaStore::new(&data);

    store
        .record_orphans("course-1", std::slice::from_ref(&orphan))
        .unwrap();
    let report = store.recover_project("course-1").unwrap();

    assert_eq!(report.removed, vec![orphan.clone()]);
    assert!(report.unresolved.is_empty());
    assert!(!orphan.exists());
    assert!(
        !data
            .join("media/projects/course-1/orphan-cleanup.json")
            .exists()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_assets_reject_orphan_paths_outside_the_project_root() {
    let root = test_root("orphan-containment");
    let store = ProjectMediaStore::new(root.join("data"));
    let outside = root.join("outside.mp4");
    fs::create_dir_all(&root).unwrap();
    fs::write(&outside, b"do not remove").unwrap();

    assert!(matches!(
        store.record_orphans("course-1", std::slice::from_ref(&outside)),
        Err(MediaAssetError::PathOutsideProject { .. })
    ));
    assert!(outside.exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_assets_startup_recovery_scans_projects_and_partial_files() {
    let root = test_root("startup-recovery");
    let data = root.join("data");
    let project = data.join("media/projects/course-1");
    let partial = project.join("exports/lesson.partial-123.mp4");
    fs::create_dir_all(partial.parent().unwrap()).unwrap();
    fs::write(&partial, b"partial").unwrap();
    let store = ProjectMediaStore::new(&data);

    let reports = store.recover_all().unwrap();

    assert_eq!(reports.len(), 1);
    assert!(reports[0].removed.contains(&partial));
    assert!(!partial.exists());
    fs::remove_dir_all(root).unwrap();
}

fn test_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("radsuite-media-assets-{label}-{}", Uuid::new_v4()))
}
