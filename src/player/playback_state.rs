use std::time::Instant;

use crate::core::models::{MediaSource, Track};

#[derive(Debug, Clone)]
pub struct BackendPlaybackState {
    pub current_media: Option<MediaSource>,
    pub is_loading_media: bool,
    pub loading_started_at: Option<Instant>,
    pub is_playing: bool,
    pub is_paused: bool,
    pub position_seconds: f64,
    pub duration_seconds: f64,
    pub volume: f64,
    pub is_muted: bool,
    pub speed: f64,
    pub is_fullscreen: bool,
    pub audio_tracks: Vec<Track>,
    pub subtitle_tracks: Vec<Track>,
    pub active_audio_track: Option<i64>,
    pub active_subtitle_track: Option<i64>,
    pub subtitle_delay: f64,
    pub audio_delay: f64,
    pub ab_loop_a: Option<f64>,
    pub ab_loop_b: Option<f64>,
}

impl Default for BackendPlaybackState {
    fn default() -> Self {
        Self {
            current_media: None,
            is_loading_media: false,
            loading_started_at: None,
            is_playing: false,
            is_paused: false,
            position_seconds: 0.0,
            duration_seconds: 0.0,
            volume: 50.0,
            is_muted: false,
            speed: 1.0,
            is_fullscreen: false,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            active_audio_track: None,
            active_subtitle_track: None,
            subtitle_delay: 0.0,
            audio_delay: 0.0,
            ab_loop_a: None,
            ab_loop_b: None,
        }
    }
}
