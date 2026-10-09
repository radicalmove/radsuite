use radsuite_engines::AudioOutputFormat;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaOutputFormat {
    #[default]
    Mp3,
    Wav,
    Mp4,
}

impl MediaOutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Mp4 => "mp4",
        }
    }

    pub const fn is_audio(self) -> bool {
        matches!(self, Self::Mp3 | Self::Wav)
    }

    pub const fn audio_format(self) -> AudioOutputFormat {
        match self {
            Self::Mp3 => AudioOutputFormat::Mp3,
            Self::Wav | Self::Mp4 => AudioOutputFormat::Wav,
        }
    }

    pub fn from_request<L>(media_format: Option<Self>, output_format: Option<L>) -> Self
    where
        L: Into<Self>,
    {
        media_format
            .or_else(|| output_format.map(Into::into))
            .unwrap_or_default()
    }
}

impl From<AudioOutputFormat> for MediaOutputFormat {
    fn from(value: AudioOutputFormat) -> Self {
        match value {
            AudioOutputFormat::Mp3 => Self::Mp3,
            AudioOutputFormat::Wav => Self::Wav,
        }
    }
}

impl From<MediaOutputFormat> for AudioOutputFormat {
    fn from(value: MediaOutputFormat) -> Self {
        value.audio_format()
    }
}
