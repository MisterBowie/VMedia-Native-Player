use std::{cell::Cell, rc::Rc};

use gtk::prelude::*;

use crate::core::state::AppState;

use super::SeekBar;

fn format_time(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Returns the appropriate volume icon for a given volume level.
fn volume_icon_name(volume: f64, is_muted: bool) -> &'static str {
    if is_muted || volume <= 0.0 {
        "audio-volume-muted-symbolic"
    } else if volume <= 25.0 {
        "audio-volume-low-symbolic"
    } else if volume <= 50.0 {
        "audio-volume-medium-symbolic"
    } else if volume <= 75.0 {
        "audio-volume-high-symbolic"
    } else {
        "audio-volume-overamplified-symbolic"
    }
}

#[derive(Clone)]
pub struct PlayerControls {
    root: gtk::Box,
    pub open_button: gtk::Button,
    pub backward_button: gtk::Button,
    pub play_pause_button: gtk::Button,
    pub forward_button: gtk::Button,
    pub fullscreen_button: gtk::Button,
    pub volume_scale: gtk::Scale,
    pub speed_button: gtk::Button,
    pub mute_button: gtk::Button,
    pub playlist_button: gtk::Button,
    volume_syncing: Rc<Cell<bool>>,
    duration_seconds: Rc<Cell<f64>>,
    seek_bar: SeekBar,
    time_label: gtk::Label,
    remaining_label: gtk::Label,
    speed_label: gtk::Label,
    status_label: gtk::Label,
    pp_icon: gtk::Image,
    fs_icon: gtk::Image,
    mute_icon: gtk::Image,
}

impl PlayerControls {
    pub fn new() -> Self {
        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(6)
            .css_classes(["control-bar"])
            .build();

        // ── Row 1: time | seek bar | remaining ──
        let time_label = gtk::Label::builder()
            .label("0:00")
            .xalign(1.0)
            .css_classes(["time-label"])
            .build();

        let remaining_label = gtk::Label::builder()
            .label("-0:00")
            .xalign(0.0)
            .css_classes(["time-label"])
            .build();

        let seek_bar = SeekBar::new();

        let seek_row = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["seek-row"])
            .build();
        seek_row.append(&time_label);
        seek_bar.widget().set_hexpand(true);
        seek_row.append(seek_bar.widget());
        seek_row.append(&remaining_label);

        // ── Row 2: left | transport | right controls ──
        // Transport: ⏮ ▶ ⏭ — pure icons
        let backward_icon = gtk::Image::from_icon_name("media-seek-backward-symbolic");
        backward_icon.set_pixel_size(27);
        let backward_button = gtk::Button::builder()
            .child(&backward_icon)
            .css_classes(["flat", "transport-btn"])
            .tooltip_text("快退 5 秒")
            .build();

        let pp_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        pp_icon.set_pixel_size(27);
        let play_pause_button = gtk::Button::builder()
            .child(&pp_icon)
            .css_classes(["play-btn"])
            .tooltip_text("播放 / 暂停")
            .build();
        play_pause_button.set_size_request(46, 46);

        let forward_icon = gtk::Image::from_icon_name("media-seek-forward-symbolic");
        forward_icon.set_pixel_size(27);
        let forward_button = gtk::Button::builder()
            .child(&forward_icon)
            .css_classes(["flat", "transport-btn"])
            .tooltip_text("快进 5 秒")
            .build();

        // ── Volume: icon button + vertical popover slider ──
        let mute_icon = gtk::Image::from_icon_name("audio-volume-high-symbolic");
        mute_icon.set_pixel_size(18);

        // IINA-style inline horizontal volume control.
        let volume_scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
        volume_scale.set_draw_value(false);
        volume_scale.set_size_request(112, -1);
        volume_scale.set_value(50.0);
        volume_scale.add_css_class("volume-slider");

        let mute_button = gtk::Button::builder()
            .child(&mute_icon)
            .css_classes(["flat", "ctrl-icon"])
            .tooltip_text("静音")
            .build();

        let speed_label = gtk::Label::builder()
            .label("1.0x")
            .css_classes(["speed-label"])
            .build();
        let speed_button = gtk::Button::builder()
            .child(&speed_label)
            .css_classes(["flat", "ctrl-icon"])
            .tooltip_text("播放速度")
            .build();

        let fs_icon = gtk::Image::from_icon_name("view-fullscreen-symbolic");
        fs_icon.set_pixel_size(17);
        let fullscreen_button = gtk::Button::builder()
            .child(&fs_icon)
            .css_classes(["flat", "ctrl-icon"])
            .tooltip_text("全屏")
            .build();

        // Open file button — in left group
        let open_button = gtk::Button::builder()
            .icon_name("document-open-symbolic")
            .css_classes(["flat", "ctrl-icon"])
            .tooltip_text("打开媒体文件")
            .build();

        // Playlist button
        let playlist_button = gtk::Button::builder()
            .icon_name("view-list-symbolic")
            .tooltip_text("播放列表")
            .css_classes(["flat", "ctrl-icon"])
            .build();

        let volume_syncing = Rc::new(Cell::new(false));
        let status_label = gtk::Label::builder()
            .xalign(0.5)
            .wrap(true)
            .css_classes(["status-label"])
            .visible(false)
            .build();

        // ── Layout Row 2 with CenterBox ──
        let left = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .css_classes(["control-group", "left-controls"])
            .build();
        left.append(&mute_button);
        left.append(&volume_scale);

        let center = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(2)
            .halign(gtk::Align::Center)
            .css_classes(["control-group", "transport-controls"])
            .build();
        center.append(&backward_button);
        center.append(&play_pause_button);
        center.append(&forward_button);

        let right = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(6)
            .css_classes(["control-group", "right-controls"])
            .build();
        right.append(&open_button);
        right.append(&speed_button);
        right.append(&playlist_button);
        right.append(&fullscreen_button);

        let controls_row = gtk::CenterBox::new();
        controls_row.set_start_widget(Some(&left));
        controls_row.set_center_widget(Some(&center));
        controls_row.set_end_widget(Some(&right));
        controls_row.add_css_class("controls-row");

        root.append(&controls_row);
        root.append(&seek_row);

        Self {
            root,
            open_button,
            backward_button,
            play_pause_button,
            forward_button,
            fullscreen_button,
            volume_scale,
            speed_button,
            mute_button,
            playlist_button,
            volume_syncing,
            duration_seconds: Rc::new(Cell::new(0.0)),
            seek_bar,
            time_label,
            remaining_label,
            speed_label,
            status_label,
            pp_icon,
            fs_icon,
            mute_icon,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }

    pub fn seek_bar(&self) -> &SeekBar {
        &self.seek_bar
    }

    pub fn bind_volume<F>(&self, on_change: F)
    where
        F: Fn(f64) + 'static,
    {
        let volume_syncing = self.volume_syncing.clone();
        self.volume_scale.connect_value_changed(move |scale| {
            if volume_syncing.get() {
                return;
            }
            on_change(scale.value());
        });
    }

    pub fn preview_seek_position(&self, position_seconds: f64) {
        let duration_seconds = self.duration_seconds.get();
        self.time_label.set_text(&format_time(position_seconds));
        let remaining = duration_seconds - position_seconds;
        self.remaining_label
            .set_text(&format!("-{}", format_time(remaining)));
    }

    pub fn render(&self, state: &AppState) {
        let has_media = state.playback.current_media.is_some();

        // Play/pause icon
        if has_media && !state.playback.is_paused {
            self.pp_icon
                .set_icon_name(Some("media-playback-pause-symbolic"));
        } else {
            self.pp_icon
                .set_icon_name(Some("media-playback-start-symbolic"));
        }

        // Seek bar
        self.seek_bar.set_position(
            state.playback.position_seconds,
            state.playback.duration_seconds,
        );

        self.duration_seconds
            .set(state.playback.duration_seconds.max(0.0));
        // While dragging, the pointer position owns the time preview so
        // backend refreshes cannot overwrite it.
        if !self.seek_bar.is_dragging() {
            self.preview_seek_position(state.playback.position_seconds);
        }

        // Speed
        self.speed_label
            .set_text(&format!("{:.1}x", state.playback.speed));

        // Volume icon (5 states)
        let icon = volume_icon_name(state.playback.volume, state.playback.is_muted);
        self.mute_icon.set_icon_name(Some(icon));

        // Volume slider sync
        if (self.volume_scale.value() - state.playback.volume).abs() > f64::EPSILON {
            self.volume_syncing.set(true);
            self.volume_scale.set_value(state.playback.volume);
            self.volume_syncing.set(false);
        }

        // Fullscreen
        if state.playback.is_fullscreen {
            self.fs_icon.set_icon_name(Some("view-restore-symbolic"));
        } else {
            self.fs_icon.set_icon_name(Some("view-fullscreen-symbolic"));
        }
    }
}
