use std::path::PathBuf;

use crate::core::models::{MediaSource, Track};

#[derive(Debug, Clone)]
pub enum AppEvent {
    // === 播放状态 ===
    PlaybackLoaded {
        media: MediaSource,
    },
    PlaybackUnloaded,
    PlaybackPaused(bool),
    PlaybackEnded,
    PositionUpdated {
        position_seconds: f64,
        duration_seconds: f64,
    },
    VolumeUpdated(f64),
    MuteChanged(bool),
    FullscreenChanged(bool),
    SpeedChanged(f64),
    TracksUpdated {
        audio: Vec<Track>,
        subtitles: Vec<Track>,
    },

    // === 媒体信息 ===
    MediaInfoAvailable {
        media_title: Option<String>,
        video_codec: Option<String>,
        audio_codec: Option<String>,
        video_width: Option<i64>,
        video_height: Option<i64>,
    },

    // === 增强播控反馈 ===
    ScreenshotTaken(PathBuf),
    SubtitleDelayChanged(f64),
    AudioDelayChanged(f64),
    ABLoopChanged {
        a: Option<f64>,
        b: Option<f64>,
    },

    // === 其他 ===
    StatusMessage(String),
    Error(String),
}
