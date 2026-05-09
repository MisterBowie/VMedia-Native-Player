use std::path::PathBuf;

use crate::core::{
    event::AppEvent,
    models::{MediaSource, Track},
};

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Loaded(MediaSource),
    Unloaded,
    Ended,
    Error(String),
    PauseChanged(bool),
    PositionChanged {
        position_seconds: f64,
        duration_seconds: f64,
    },
    VolumeChanged(f64),
    MuteChanged(bool),
    SpeedChanged(f64),
    FullscreenChanged(bool),
    TracksChanged {
        audio: Vec<Track>,
        subtitles: Vec<Track>,
    },
    MediaInfoChanged {
        media_title: Option<String>,
        video_codec: Option<String>,
        audio_codec: Option<String>,
        video_width: Option<i64>,
        video_height: Option<i64>,
    },
    ScreenshotTaken(PathBuf),
    SubtitleDelayChanged(f64),
    AudioDelayChanged(f64),
    ABLoopChanged {
        a: Option<f64>,
        b: Option<f64>,
    },
    Status(String),
}

impl From<BackendEvent> for AppEvent {
    fn from(value: BackendEvent) -> Self {
        match value {
            BackendEvent::Loaded(media) => AppEvent::PlaybackLoaded { media },
            BackendEvent::Unloaded => AppEvent::PlaybackUnloaded,
            BackendEvent::Ended => AppEvent::PlaybackEnded,
            BackendEvent::Error(message) => AppEvent::Error(message),
            BackendEvent::PauseChanged(is_paused) => AppEvent::PlaybackPaused(is_paused),
            BackendEvent::PositionChanged {
                position_seconds,
                duration_seconds,
            } => AppEvent::PositionUpdated {
                position_seconds,
                duration_seconds,
            },
            BackendEvent::VolumeChanged(volume) => AppEvent::VolumeUpdated(volume),
            BackendEvent::MuteChanged(is_muted) => AppEvent::MuteChanged(is_muted),
            BackendEvent::SpeedChanged(speed) => AppEvent::SpeedChanged(speed),
            BackendEvent::FullscreenChanged(is_fullscreen) => {
                AppEvent::FullscreenChanged(is_fullscreen)
            }
            BackendEvent::TracksChanged { audio, subtitles } => {
                AppEvent::TracksUpdated { audio, subtitles }
            }
            BackendEvent::MediaInfoChanged {
                media_title,
                video_codec,
                audio_codec,
                video_width,
                video_height,
            } => AppEvent::MediaInfoAvailable {
                media_title,
                video_codec,
                audio_codec,
                video_width,
                video_height,
            },
            BackendEvent::ScreenshotTaken(path) => AppEvent::ScreenshotTaken(path),
            BackendEvent::SubtitleDelayChanged(delay) => AppEvent::SubtitleDelayChanged(delay),
            BackendEvent::AudioDelayChanged(delay) => AppEvent::AudioDelayChanged(delay),
            BackendEvent::ABLoopChanged { a, b } => AppEvent::ABLoopChanged { a, b },
            BackendEvent::Status(message) => AppEvent::StatusMessage(message),
        }
    }
}
