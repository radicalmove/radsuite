use radsuite_engines::{
    AudioOutputFormat, AudioProcessingRequest, AudioProcessor, EnhancementModel,
};
use std::path::PathBuf;

#[test]
fn treble_is_a_separate_guarded_preset_with_legacy_compatibility() {
    let model: EnhancementModel = serde_json::from_str("\"studio_treble\"").unwrap();
    assert_eq!(model.label(), "Studio — Treble (recommended)");
    assert!(model.is_guarded_studio());
    assert!(EnhancementModel::all().contains(&model));
    assert!(EnhancementModel::StudioV1.is_guarded_studio());
    assert!(!EnhancementModel::StudioV18.is_guarded_studio());
    assert_eq!(
        serde_json::to_string(&EnhancementModel::StudioV1).unwrap(),
        "\"studio_v1\""
    );
}

#[test]
fn studio_registration_preserves_legacy_ids() {
    let new: EnhancementModel = serde_json::from_str("\"studio_v1\"").expect("new preset");
    assert_eq!(new.label(), "Studio — Classic");
    assert!(EnhancementModel::all().contains(&new));
    for id in ["studio", "studio_v18", "studio_v18_natural_double_plus"] {
        let old: EnhancementModel = serde_json::from_str(&format!("\"{id}\"")).unwrap();
        assert_eq!(serde_json::to_string(&old).unwrap(), format!("\"{id}\""));
    }
}

#[test]
fn studio_working_arguments_are_float_pcm_48k_and_no_mastering() {
    let request = AudioProcessingRequest {
        input_path: PathBuf::from("input.mp3"),
        output_path: PathBuf::from("working.wav"),
        output_format: AudioOutputFormat::Wav,
        clip_start_seconds: None,
        clip_end_seconds: None,
        max_silence_seconds: None,
        remove_intervals: vec![],
        cleanup_enabled: false,
    };
    let args = AudioProcessor::studio_ffmpeg_arguments(&request, None).unwrap();
    let args: Vec<_> = args
        .iter()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert!(args.windows(2).any(|a| a == ["-codec:a", "pcm_f32le"]));
    assert!(args.windows(2).any(|a| a == ["-ar", "48000"]));
    assert!(
        !args
            .iter()
            .any(|a| a.contains("loudnorm") || a.contains("libmp3lame"))
    );
}

#[test]
fn studio_final_mp3_encoding_only_at_export() {
    let request = AudioProcessingRequest {
        input_path: PathBuf::from("mastered.wav"),
        output_path: PathBuf::from("final.mp3"),
        output_format: AudioOutputFormat::Mp3,
        clip_start_seconds: None,
        clip_end_seconds: None,
        max_silence_seconds: None,
        remove_intervals: vec![],
        cleanup_enabled: false,
    };
    let args = AudioProcessor::studio_ffmpeg_arguments(&request, None).unwrap();
    let args: Vec<_> = args
        .iter()
        .map(|a| a.to_string_lossy().to_string())
        .collect();
    assert!(args.windows(2).any(|a| a == ["-codec:a", "libmp3lame"]));
    assert!(
        !args
            .iter()
            .any(|a| a.contains("loudnorm") || a.contains("lowpass"))
    );
}
