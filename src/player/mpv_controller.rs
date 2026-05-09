use std::time::{Duration, Instant};

use crate::core::{command::AppCommand, event::AppEvent, models::MediaSource};

use super::{libmpv::LibMpv, mpv_events::BackendEvent, playback_state::BackendPlaybackState};

const MEDIA_LOAD_TIMEOUT: Duration = Duration::from_secs(10);

pub struct MpvController {
    backend: Option<LibMpv>,
    backend_error: Option<String>,
    state: BackendPlaybackState,
}

impl MpvController {
    pub fn new() -> Self {
        match LibMpv::new() {
            Ok(backend) => Self {
                backend: Some(backend),
                backend_error: None,
                state: BackendPlaybackState::default(),
            },
            Err(error) => Self {
                backend: None,
                backend_error: Some(error),
                state: BackendPlaybackState::default(),
            },
        }
    }

    pub fn render_backend(&self) -> Option<LibMpv> {
        self.backend.clone()
    }

    pub fn handle_pending_backend_updates(&mut self) -> Vec<AppEvent> {
        if !self.state.is_loading_media && !self.backend_has_pending_update() {
            return Vec::new();
        }

        match self.drain_backend_updates() {
            Ok(events) => events.into_iter().map(AppEvent::from).collect(),
            Err(error) => vec![AppEvent::Error(error)],
        }
    }

    pub fn handle_command(&mut self, command: AppCommand) -> Vec<AppEvent> {
        let is_background_refresh = matches!(&command, AppCommand::RefreshState);
        let result = match command {
            AppCommand::OpenFile(path) => self.load_media(path),
            AppCommand::TogglePause => self.toggle_pause(),
            AppCommand::SeekRelative(seconds) => self.seek_relative(seconds),
            AppCommand::SeekAbsolute(seconds) => self.seek_absolute(seconds),
            AppCommand::SetVolume(volume) => self.set_volume(volume),
            AppCommand::SelectSubtitle(track_id) => self.select_subtitle(track_id),
            AppCommand::SelectAudio(track_id) => self.select_audio(track_id),
            AppCommand::SetFullscreen(is_fullscreen) => self.set_fullscreen(is_fullscreen),
            AppCommand::LoadExternalSubtitle(path) => self.load_external_subtitle(path),
            AppCommand::Stop => self.stop(),
            AppCommand::RefreshState => self.refresh_state(),
            AppCommand::SetSpeed(speed) => self.set_speed(speed),
            AppCommand::SetSubtitleDelay(delay) => self.set_subtitle_delay(delay),
            AppCommand::SetAudioDelay(delay) => self.set_audio_delay(delay),
            AppCommand::Screenshot => self.take_screenshot(),
            AppCommand::SetABLoop { a, b } => self.set_ab_loop(a, b),
            AppCommand::ToggleMute => self.toggle_mute(),
        };

        match result {
            Ok(events) => events.into_iter().map(AppEvent::from).collect(),
            Err(_) if is_background_refresh => Vec::new(),
            Err(error) => vec![AppEvent::Error(error)],
        }
    }

    fn load_media(&mut self, path: std::path::PathBuf) -> Result<Vec<BackendEvent>, String> {
        let media = MediaSource::from_path(path.clone());
        self.backend()?.load_file(&path)?;

        self.state.current_media = Some(media.clone());
        self.state.is_loading_media = true;
        self.state.loading_started_at = Some(Instant::now());
        self.state.is_playing = false;
        self.state.is_paused = false;
        self.state.position_seconds = 0.0;
        self.state.duration_seconds = 0.0;
        self.state.audio_tracks.clear();
        self.state.subtitle_tracks.clear();
        self.state.active_audio_track = None;
        self.state.active_subtitle_track = None;

        let mut events = vec![
            BackendEvent::Unloaded,
            BackendEvent::PauseChanged(false),
            BackendEvent::PositionChanged {
                position_seconds: 0.0,
                duration_seconds: 0.0,
            },
            BackendEvent::TracksChanged {
                audio: Vec::new(),
                subtitles: Vec::new(),
            },
        ];
        events.push(BackendEvent::Status(format!(
            "正在加载 {}",
            media.display_name
        )));
        Ok(events)
    }

    fn toggle_pause(&mut self) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.toggle_pause()?;

        let paused = self.backend()?.paused()?;
        let mut events = self.refresh_state_events()?;
        let status = if paused {
            "已暂停（libmpv）"
        } else {
            "已继续（libmpv）"
        };
        events.push(BackendEvent::Status(status.to_string()));
        Ok(events)
    }

    fn seek_relative(&mut self, seconds: f64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.seek_relative(seconds)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!(
            "跳转 {:.0} 秒（libmpv）。",
            seconds
        )));
        Ok(events)
    }

    fn seek_absolute(&mut self, seconds: f64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.seek_absolute(seconds)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!(
            "跳转到 {:.0} 秒。",
            seconds
        )));
        Ok(events)
    }

    fn set_volume(&mut self, volume: f64) -> Result<Vec<BackendEvent>, String> {
        self.backend()?.set_volume(volume)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!(
            "音量 {:.0}%",
            self.state.volume
        )));
        Ok(events)
    }

    fn select_subtitle(&mut self, track_id: i64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.set_subtitle_track(track_id)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!("已切换字幕轨 {}", track_id)));
        Ok(events)
    }

    fn select_audio(&mut self, track_id: i64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.set_audio_track(track_id)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!("已切换音轨 {}", track_id)));
        Ok(events)
    }

    fn set_fullscreen(&mut self, is_fullscreen: bool) -> Result<Vec<BackendEvent>, String> {
        self.state.is_fullscreen = is_fullscreen;

        let mut events = self.refresh_state_events()?;
        let status = if is_fullscreen {
            "已进入全屏"
        } else {
            "已退出全屏"
        };
        events.push(BackendEvent::Status(status.to_string()));
        Ok(events)
    }

    fn load_external_subtitle(
        &mut self,
        path: std::path::PathBuf,
    ) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.load_subtitle(&path)?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status(format!(
            "已加载外挂字幕：{}",
            path.display()
        )));
        Ok(events)
    }

    fn stop(&mut self) -> Result<Vec<BackendEvent>, String> {
        self.backend()?.stop()?;

        self.state.current_media = None;
        self.state.is_loading_media = false;
        self.state.loading_started_at = None;
        self.state.is_playing = false;
        self.state.is_paused = false;
        self.state.position_seconds = 0.0;
        self.state.duration_seconds = 0.0;
        self.state.audio_tracks.clear();
        self.state.subtitle_tracks.clear();
        self.state.active_audio_track = None;
        self.state.active_subtitle_track = None;

        Ok(vec![
            BackendEvent::Unloaded,
            BackendEvent::Status("播放已停止。".to_string()),
        ])
    }

    fn set_speed(&mut self, speed: f64) -> Result<Vec<BackendEvent>, String> {
        self.backend()?.set_speed(speed)?;
        self.state.speed = speed;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::SpeedChanged(speed));
        events.push(BackendEvent::Status(format!("{:.1}x 倍速", speed)));
        Ok(events)
    }

    fn set_subtitle_delay(&mut self, delay: f64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.set_subtitle_delay(delay)?;
        self.state.subtitle_delay = delay;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::SubtitleDelayChanged(delay));
        events.push(BackendEvent::Status(format!("字幕延迟 {:.1}s", delay)));
        Ok(events)
    }

    fn set_audio_delay(&mut self, delay: f64) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.set_audio_delay(delay)?;
        self.state.audio_delay = delay;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::AudioDelayChanged(delay));
        events.push(BackendEvent::Status(format!("音频延迟 {:.1}s", delay)));
        Ok(events)
    }

    fn take_screenshot(&mut self) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.screenshot()?;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::Status("截图已保存。".to_string()));
        Ok(events)
    }

    fn set_ab_loop(
        &mut self,
        a: Option<f64>,
        b: Option<f64>,
    ) -> Result<Vec<BackendEvent>, String> {
        self.require_media()?;
        self.backend()?.set_ab_loop_a(a)?;
        self.backend()?.set_ab_loop_b(b)?;
        self.state.ab_loop_a = a;
        self.state.ab_loop_b = b;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::ABLoopChanged { a, b });
        let status = match (a, b) {
            (Some(a), Some(b)) => format!("A-B 循环: {:.1}s - {:.1}s", a, b),
            (Some(a), None) => format!("A-B 循环: A={:.1}s (等待 B 点)", a),
            _ => "A-B 循环已清除。".to_string(),
        };
        events.push(BackendEvent::Status(status));
        Ok(events)
    }

    fn toggle_mute(&mut self) -> Result<Vec<BackendEvent>, String> {
        let currently_muted = self.backend()?.muted().unwrap_or(false);
        let new_mute = !currently_muted;
        self.backend()?.set_mute(new_mute)?;
        self.state.is_muted = new_mute;

        let mut events = self.refresh_state_events()?;
        events.push(BackendEvent::MuteChanged(new_mute));
        let status = if new_mute { "已静音" } else { "已取消静音" };
        events.push(BackendEvent::Status(status.to_string()));
        Ok(events)
    }

    fn backend_has_pending_update(&self) -> bool {
        self.backend
            .as_ref()
            .is_some_and(LibMpv::take_wakeup_pending)
    }

    fn drain_backend_updates(&mut self) -> Result<Vec<BackendEvent>, String> {
        let Some(backend) = &self.backend else {
            return Ok(Vec::new());
        };

        let drained_pending_events = backend.drain_pending_events()?;
        if !drained_pending_events && !self.state.is_loading_media {
            return Ok(Vec::new());
        }

        self.refresh_state()
    }

    fn refresh_state(&mut self) -> Result<Vec<BackendEvent>, String> {
        if self.state.current_media.is_none()
            && !self.state.is_loading_media
            && self.backend()?.path()?.is_none()
        {
            return Ok(Vec::new());
        }

        self.refresh_state_events()
    }

    fn refresh_state_events(&mut self) -> Result<Vec<BackendEvent>, String> {
        let backend = self.backend()?;

        let media_path = backend.path()?;
        let requested_media_path = self
            .state
            .current_media
            .as_ref()
            .map(|media| media.path.clone());
        let current_media_path = self
            .state
            .current_media
            .as_ref()
            .map(|media| media.path.clone());
        let mut emitted_events = Vec::new();
        let mut paused = backend.paused().unwrap_or(self.state.is_paused);
        let mut position_seconds = backend
            .position_seconds()
            .unwrap_or(self.state.position_seconds);
        let mut duration_seconds = backend
            .duration_seconds()
            .unwrap_or(self.state.duration_seconds);
        let volume = backend.volume().unwrap_or(self.state.volume);
        let speed = backend.speed().unwrap_or(self.state.speed);
        let is_muted = backend.muted().unwrap_or(self.state.is_muted);
        let is_fullscreen = self.state.is_fullscreen;
        let (mut audio_tracks, mut subtitle_tracks) = backend.tracks().unwrap_or_else(|_| {
            (
                self.state.audio_tracks.clone(),
                self.state.subtitle_tracks.clone(),
            )
        });
        let mut active_audio_track = backend
            .current_audio_track()
            .unwrap_or(self.state.active_audio_track);
        let mut active_subtitle_track = backend
            .current_subtitle_track()
            .unwrap_or(self.state.active_subtitle_track);

        match media_path {
            Some(path) => {
                let path = std::path::PathBuf::from(path);
                let still_loading_new_media = self.state.is_loading_media
                    && requested_media_path
                        .as_ref()
                        .is_some_and(|requested_path| requested_path != &path);

                if !still_loading_new_media {
                    let should_emit_loaded =
                        self.state.is_loading_media || current_media_path.as_ref() != Some(&path);
                    let confirmed_media = MediaSource::from_path(path);
                    if should_emit_loaded {
                        emitted_events.push(BackendEvent::Loaded(confirmed_media.clone()));

                        // Emit media info when media is first loaded
                        let media_title = backend.media_title().unwrap_or(None);
                        let video_codec = backend.video_codec().unwrap_or(None);
                        let audio_codec = backend.audio_codec().unwrap_or(None);
                        let video_width = backend.video_width().unwrap_or(None);
                        let video_height = backend.video_height().unwrap_or(None);
                        emitted_events.push(BackendEvent::MediaInfoChanged {
                            media_title,
                            video_codec,
                            audio_codec,
                            video_width,
                            video_height,
                        });
                    }
                    self.state.current_media = Some(confirmed_media);
                    self.state.is_loading_media = false;
                    self.state.loading_started_at = None;
                } else if self.media_load_timed_out() {
                    emitted_events.push(BackendEvent::Unloaded);
                    emitted_events
                        .push(BackendEvent::Error("媒体加载失败或已被卸载。".to_string()));
                    self.state.current_media = None;
                    self.state.is_loading_media = false;
                    self.state.loading_started_at = None;
                    paused = false;
                    position_seconds = 0.0;
                    duration_seconds = 0.0;
                    audio_tracks.clear();
                    subtitle_tracks.clear();
                    active_audio_track = None;
                    active_subtitle_track = None;
                } else {
                    paused = self.state.is_paused;
                    position_seconds = self.state.position_seconds;
                    duration_seconds = self.state.duration_seconds;
                    audio_tracks = self.state.audio_tracks.clone();
                    subtitle_tracks = self.state.subtitle_tracks.clone();
                    active_audio_track = self.state.active_audio_track;
                    active_subtitle_track = self.state.active_subtitle_track;
                }
            }
            None if !self.state.is_loading_media => {
                if self.state.current_media.is_some() {
                    emitted_events.push(BackendEvent::Unloaded);
                }
                self.state.current_media = None;
                self.state.loading_started_at = None;
                paused = false;
                position_seconds = 0.0;
                duration_seconds = 0.0;
                audio_tracks.clear();
                subtitle_tracks.clear();
                active_audio_track = None;
                active_subtitle_track = None;
            }
            None => {
                if self.media_load_timed_out() {
                    emitted_events.push(BackendEvent::Unloaded);
                    emitted_events
                        .push(BackendEvent::Error("媒体加载失败或已被卸载。".to_string()));
                    self.state.current_media = None;
                    self.state.is_loading_media = false;
                    self.state.loading_started_at = None;
                    paused = false;
                    position_seconds = 0.0;
                    duration_seconds = 0.0;
                    audio_tracks.clear();
                    subtitle_tracks.clear();
                    active_audio_track = None;
                    active_subtitle_track = None;
                } else {
                    paused = self.state.is_paused;
                    position_seconds = self.state.position_seconds;
                    duration_seconds = self.state.duration_seconds;
                    audio_tracks = self.state.audio_tracks.clone();
                    subtitle_tracks = self.state.subtitle_tracks.clone();
                    active_audio_track = self.state.active_audio_track;
                    active_subtitle_track = self.state.active_subtitle_track;
                }
            }
        }

        self.state.is_paused = paused;
        self.state.is_playing = self.state.current_media.is_some() && !paused;
        self.state.position_seconds = position_seconds;
        self.state.duration_seconds = duration_seconds;
        self.state.volume = volume;
        self.state.speed = speed;
        self.state.is_muted = is_muted;
        self.state.is_fullscreen = is_fullscreen;
        self.state.audio_tracks = audio_tracks.clone();
        self.state.subtitle_tracks = subtitle_tracks.clone();
        self.state.active_audio_track = active_audio_track;
        self.state.active_subtitle_track = active_subtitle_track;

        emitted_events.extend([
            BackendEvent::PauseChanged(paused),
            BackendEvent::PositionChanged {
                position_seconds,
                duration_seconds,
            },
            BackendEvent::VolumeChanged(volume),
            BackendEvent::SpeedChanged(speed),
            BackendEvent::MuteChanged(is_muted),
            BackendEvent::FullscreenChanged(is_fullscreen),
            BackendEvent::TracksChanged {
                audio: audio_tracks,
                subtitles: subtitle_tracks,
            },
        ]);

        Ok(emitted_events)
    }

    fn media_load_timed_out(&self) -> bool {
        self.state.is_loading_media
            && self
                .state
                .loading_started_at
                .is_some_and(|started_at| started_at.elapsed() >= MEDIA_LOAD_TIMEOUT)
    }

    fn backend(&self) -> Result<&LibMpv, String> {
        self.backend.as_ref().ok_or_else(|| {
            self.backend_error
                .clone()
                .unwrap_or_else(|| "libmpv backend is unavailable.".to_string())
        })
    }

    fn require_media(&self) -> Result<(), String> {
        if self.state.current_media.is_none() {
            Err("请先打开一个本地媒体文件。".to_string())
        } else if self.state.is_loading_media {
            Err("请等待媒体加载完成。".to_string())
        } else {
            Ok(())
        }
    }
}
