use radsuite_desktop::{
    MediaOutputFormat,
    radcast::{ProcessRadcastAudioRequest, RadcastAudioOutput, RadcastProjectSettings},
    radt_ts::{RadtTsAudioOutput, RadtTsOutputFormat, StartRadtTsSynthesisRequest},
    radt_ts_tools::{
        RadtTsMediaArtifact, RadtTsMediaJobKind, RadtTsMediaOutput, StartRadtTsClipRequest,
    },
};
use radsuite_engines::AudioOutputFormat;
use serde_json::{Value, json};

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

    let mut settings = RadcastProjectSettings::default();
    settings.media_format = Some(MediaOutputFormat::Mp4);
    settings.output_format = AudioOutputFormat::Wav;
    let serialized = serde_json::to_value(&settings).unwrap();
    assert_eq!(serialized["media_format"], Value::from("mp4"));
    assert_eq!(serialized["output_format"], Value::from("wav"));
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
        warnings: Vec::new(),
    };
    let serialized = serde_json::to_value(new_clip).unwrap();
    assert_eq!(serialized["media_format"], Value::from("mp4"));
    assert_eq!(serialized["output_format"], Value::from("wav"));
}
