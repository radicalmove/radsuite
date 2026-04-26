use crate::EngineStatus;

#[derive(Debug, Default, Clone)]
pub struct EngineRegistry;

impl EngineRegistry {
    pub fn list(&self) -> Vec<EngineStatus> {
        [
            ("ffmpeg", "FFmpeg media processing"),
            ("asr", "Speech recognition"),
            ("audio_cleanup", "Audio cleanup"),
            ("tts", "Voice and text-to-speech"),
        ]
        .into_iter()
        .map(|(id, label)| EngineStatus {
            id: id.to_string(),
            label: label.to_string(),
            available: false,
            detail: "Engine detection is not implemented in Phase 1.".to_string(),
        })
        .collect()
    }
}
