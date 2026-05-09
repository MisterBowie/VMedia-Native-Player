use crate::core::{
    event::AppEvent,
    models::{MediaInfo, MediaSource, Track},
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub playback: PlaybackState,
    pub status_line: String,
    pub last_error: Option<String>,
}

impl AppState {
    pub fn apply_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::PlaybackLoaded { media } => {
                self.playback.current_media = Some(media.clone());
                self.playback.is_playing = true;
                self.playback.is_paused = false;
                self.playback.media_info = MediaInfo::default();
                self.status_line = format!("已载入 {}", media.display_name);
                self.last_error = None;
            }
            AppEvent::PlaybackUnloaded => {
                self.playback = PlaybackState {
                    volume: self.playback.volume,
                    is_muted: self.playback.is_muted,
                    speed: 1.0,
                    ..PlaybackState::default()
                };
                self.status_line = "当前没有正在播放的媒体。".to_string();
                self.last_error = None;
            }
            AppEvent::PlaybackPaused(is_paused) => {
                self.playback.is_paused = *is_paused;
                self.playback.is_playing = self.playback.current_media.is_some() && !is_paused;
            }
            AppEvent::PlaybackEnded => {
                self.playback.is_playing = false;
                self.playback.is_paused = false;
                self.status_line = "播放完毕。".to_string();
            }
            AppEvent::PositionUpdated {
                position_seconds,
                duration_seconds,
            } => {
                self.playback.position_seconds = *position_seconds;
                self.playback.duration_seconds = *duration_seconds;
            }
            AppEvent::VolumeUpdated(volume) => {
                self.playback.volume = *volume;
            }
            AppEvent::MuteChanged(is_muted) => {
                self.playback.is_muted = *is_muted;
            }
            AppEvent::FullscreenChanged(is_fullscreen) => {
                self.playback.is_fullscreen = *is_fullscreen;
            }
            AppEvent::SpeedChanged(speed) => {
                self.playback.speed = *speed;
            }
            AppEvent::TracksUpdated { audio, subtitles } => {
                self.playback.audio_tracks = audio.clone();
                self.playback.subtitle_tracks = subtitles.clone();
                self.playback.active_audio_track = audio
                    .iter()
                    .find(|track| track.selected)
                    .map(|track| track.id);
                self.playback.active_subtitle_track = subtitles
                    .iter()
                    .find(|track| track.selected)
                    .map(|track| track.id);
            }
            AppEvent::MediaInfoAvailable {
                media_title,
                video_codec,
                audio_codec,
                video_width,
                video_height,
            } => {
                self.playback.media_info = MediaInfo {
                    media_title: media_title.clone(),
                    video_codec: video_codec.clone(),
                    audio_codec: audio_codec.clone(),
                    video_width: *video_width,
                    video_height: *video_height,
                };
            }
            AppEvent::ScreenshotTaken(path) => {
                self.status_line = format!("截图已保存：{}", path.display());
            }
            AppEvent::SubtitleDelayChanged(delay) => {
                self.playback.subtitle_delay = *delay;
            }
            AppEvent::AudioDelayChanged(delay) => {
                self.playback.audio_delay = *delay;
            }
            AppEvent::ABLoopChanged { a, b } => {
                self.playback.ab_loop_a = *a;
                self.playback.ab_loop_b = *b;
            }
            AppEvent::StatusMessage(message) => {
                self.status_line = message.clone();
                self.last_error = None;
            }
            AppEvent::Error(message) => {
                self.status_line = message.clone();
                self.last_error = Some(message.clone());
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub current_media: Option<MediaSource>,
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
    pub media_info: MediaInfo,
    pub subtitle_delay: f64,
    pub audio_delay: f64,
    pub ab_loop_a: Option<f64>,
    pub ab_loop_b: Option<f64>,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            current_media: None,
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
            media_info: MediaInfo::default(),
            subtitle_delay: 0.0,
            audio_delay: 0.0,
            ab_loop_a: None,
            ab_loop_b: None,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            playback: PlaybackState::default(),
            status_line: "打开一个本地媒体文件开始播放。".to_string(),
            last_error: None,
        }
    }
}
