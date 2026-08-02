use std::{
    cell::{Cell, RefCell},
    path::{Path, PathBuf},
    rc::Rc,
};

use gtk::prelude::*;
use tracing::error;

use crate::{core::state::AppState, player::libmpv::LibMpv};

use super::widgets::{PlayerControls, PlaylistPanel};

/// The main player view: video fills everything, controls overlay on bottom.
#[derive(Clone)]
pub struct PlayerView {
    overlay: gtk::Overlay,
    pub gl_area: gtk::GLArea,
    placeholder_box: gtk::Box,
    placeholder_card: gtk::Box,
    placeholder_icon: gtk::Image,
    placeholder_spinner: gtk::Spinner,
    placeholder_title: gtk::Label,
    placeholder_label: gtk::Label,
    placeholder_shortcuts: gtk::Box,
    controls: PlayerControls,
    pub controls_wrapper: gtk::Box,
    info_label: gtk::Label,
    pub empty_open_button: gtk::Button,
    pub playlist_panel: PlaylistPanel,

    has_render_backend: bool,
    fatal_render_error: Rc<RefCell<Option<String>>>,
    media_render_error: Rc<RefCell<Option<String>>>,
    has_rendered_frame: Rc<Cell<bool>>,
    last_media_path: Rc<RefCell<Option<PathBuf>>>,
}

impl PlayerView {
    pub fn new(render_backend: Option<LibMpv>) -> Self {
        // ── GL Area (video) fills whole space ──
        let gl_area = gtk::GLArea::new();
        gl_area.set_hexpand(true);
        gl_area.set_vexpand(true);
        gl_area.set_auto_render(false);
        gl_area.set_has_depth_buffer(false);
        gl_area.set_has_stencil_buffer(false);
        gl_area.set_required_version(3, 2);
        gl_area.set_use_es(false);
        gl_area.add_css_class("video-area");

        // ── Placeholder (empty state) ──
        let placeholder_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
        placeholder_icon.set_pixel_size(34);
        placeholder_icon.add_css_class("placeholder-icon");

        let placeholder_spinner = gtk::Spinner::builder()
            .width_request(34)
            .height_request(34)
            .visible(false)
            .css_classes(["placeholder-spinner"])
            .build();

        let placeholder_icon_shell = gtk::Box::builder()
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["placeholder-icon-shell"])
            .build();
        placeholder_icon_shell.append(&placeholder_icon);
        placeholder_icon_shell.append(&placeholder_spinner);

        let placeholder_title = gtk::Label::builder()
            .label("打开媒体，开始播放")
            .css_classes(["placeholder-title"])
            .build();

        let placeholder_label = gtk::Label::builder()
            .label("选择一个本地视频或音频文件，享受清爽、专注的播放体验。")
            .justify(gtk::Justification::Center)
            .wrap(true)
            .max_width_chars(46)
            .css_classes(["placeholder-hint"])
            .build();

        let open_button_content = adw::ButtonContent::builder()
            .icon_name("document-open-symbolic")
            .label("打开媒体文件")
            .build();
        let empty_open_button = gtk::Button::builder()
            .child(&open_button_content)
            .halign(gtk::Align::Center)
            .css_classes(["suggested-action", "empty-open-button"])
            .tooltip_text("选择本地视频或音频文件")
            .build();

        let shortcuts = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .halign(gtk::Align::Center)
            .css_classes(["shortcut-row"])
            .build();
        for shortcut in ["Space  播放 / 暂停", "← →  快退 / 快进", "F  全屏"] {
            shortcuts.append(
                &gtk::Label::builder()
                    .label(shortcut)
                    .css_classes(["shortcut-chip"])
                    .build(),
            );
        }

        let placeholder_card = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(12)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .css_classes(["placeholder-card"])
            .build();
        placeholder_card.append(&placeholder_icon_shell);
        placeholder_card.append(&placeholder_title);
        placeholder_card.append(&placeholder_label);
        placeholder_card.append(&empty_open_button);
        placeholder_card.append(&shortcuts);

        let placeholder_center = gtk::CenterBox::new();
        placeholder_center.set_hexpand(true);
        placeholder_center.set_vexpand(true);
        placeholder_center.set_center_widget(Some(&placeholder_card));

        let placeholder_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["placeholder-container"])
            .build();
        placeholder_box.append(&placeholder_center);

        // ── Bottom gradient overlay + controls ──
        let controls = PlayerControls::new();
        let info_label = gtk::Label::builder()
            .xalign(0.5)
            .css_classes(["info-label"])
            .build();

        let controls_wrapper = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .css_classes(["controls-bottom"])
            .valign(gtk::Align::End)
            .halign(gtk::Align::Center)
            .hexpand(false)
            .margin_start(0)
            .margin_end(0)
            .margin_bottom(32)
            .build();
        controls_wrapper.append(controls.widget());

        // ── Playlist panel (right side overlay) ──
        let playlist_panel = PlaylistPanel::new();

        // ── Overlay assembly ──
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&gl_area));
        overlay.add_overlay(&placeholder_box);
        overlay.add_overlay(&controls_wrapper);
        // The playlist is intentionally last so it overlays the video and
        // the right side of the controls, matching IINA's drawer behavior.
        overlay.add_overlay(&playlist_panel.root);
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);

        let fatal_render_error = Rc::new(RefCell::new(None));
        let media_render_error = Rc::new(RefCell::new(None));
        let has_rendered_frame = Rc::new(Cell::new(false));
        let last_media_path = Rc::new(RefCell::new(None));

        connect_gl_area(
            &gl_area,
            render_backend.clone(),
            placeholder_box.clone(),
            placeholder_label.clone(),
            fatal_render_error.clone(),
            media_render_error.clone(),
            has_rendered_frame.clone(),
        );

        Self {
            overlay,
            gl_area,
            placeholder_box,
            placeholder_card,
            placeholder_icon,
            placeholder_spinner,
            placeholder_title,
            placeholder_label,
            placeholder_shortcuts: shortcuts,
            controls,
            controls_wrapper,
            info_label,
            empty_open_button,
            playlist_panel,
            has_render_backend: render_backend.is_some(),
            fatal_render_error,
            media_render_error,
            has_rendered_frame,
            last_media_path,
        }
    }

    pub fn widget(&self) -> &gtk::Overlay {
        &self.overlay
    }

    pub fn controls(&self) -> &PlayerControls {
        &self.controls
    }

    pub fn render(&self, state: &AppState) {
        let current_media_path = state
            .playback
            .current_media
            .as_ref()
            .map(|m| m.path.as_path());
        let media_changed = {
            let mut last = self.last_media_path.borrow_mut();
            sync_media_session(&mut last, current_media_path)
        };
        if media_changed {
            self.has_rendered_frame.set(false);
            self.media_render_error.borrow_mut().take();
        }

        let has_media = state.playback.current_media.is_some();
        // Tag for autohide: only auto-hide when media is loaded
        if has_media {
            self.controls_wrapper.add_css_class("has-media");
        } else {
            self.controls_wrapper.remove_css_class("has-media");
            // Ensure controls are visible when no media
            self.controls_wrapper.set_opacity(1.0);
            self.controls_wrapper.set_can_target(true);
        }

        // Sync playlist with current media
        if let Some(media) = &state.playback.current_media {
            self.playlist_panel.update_for_media(&media.path);
            self.playlist_panel
                .set_media_duration(&media.path, state.playback.duration_seconds);
        }

        let render_error = active_render_error(
            self.fatal_render_error.borrow().as_deref(),
            self.media_render_error.borrow().as_deref(),
        );

        let is_preparing = render_error.is_none() && has_media && !self.has_rendered_frame.get();
        if is_preparing {
            self.placeholder_card.add_css_class("loading-state");
            self.placeholder_icon.set_visible(false);
            self.placeholder_spinner.set_visible(true);
            self.placeholder_spinner.start();
        } else {
            self.placeholder_card.remove_css_class("loading-state");
            self.placeholder_spinner.stop();
            self.placeholder_spinner.set_visible(false);
            self.placeholder_icon.set_visible(true);
        }

        if let Some(err) = render_error {
            self.placeholder_title.set_text("播放遇到问题");
            self.placeholder_label.set_text(&err);
            self.empty_open_button.set_visible(true);
            self.placeholder_shortcuts.set_visible(false);
            self.placeholder_box.set_visible(true);
        } else if has_media {
            self.empty_open_button.set_visible(false);
            self.placeholder_shortcuts.set_visible(false);
            if self.has_rendered_frame.get() {
                self.placeholder_box.set_visible(false);
            } else {
                self.placeholder_title.set_text("正在准备画面");
                self.placeholder_label.set_text("视频即将开始播放…");
                self.placeholder_box.set_visible(true);
            }
        } else {
            self.has_rendered_frame.set(false);
            self.playlist_panel.root.set_visible(false);
            self.placeholder_title.set_text("打开媒体，开始播放");
            self.placeholder_label
                .set_text("选择一个本地视频或音频文件，享受清爽、专注的播放体验。");
            self.empty_open_button.set_visible(true);
            self.placeholder_shortcuts.set_visible(true);
            self.placeholder_box.set_visible(true);
        }

        // Update controls
        self.controls.render(state);
    }
}

fn connect_gl_area(
    gl_area: &gtk::GLArea,
    render_backend: Option<LibMpv>,
    placeholder_box: gtk::Box,
    placeholder_label: gtk::Label,
    fatal_render_error: Rc<RefCell<Option<String>>>,
    media_render_error: Rc<RefCell<Option<String>>>,
    has_rendered_frame: Rc<Cell<bool>>,
) {
    let Some(render_backend) = render_backend else {
        return;
    };

    let be = render_backend.clone();
    let pb = placeholder_box.clone();
    let pl = placeholder_label.clone();
    let fe = fatal_render_error.clone();
    let me = media_render_error.clone();
    let hf = has_rendered_frame.clone();
    gl_area.connect_realize(move |gl_area| match be.initialize_render_context(gl_area) {
        Ok(()) => {
            hf.set(false);
            fe.borrow_mut().take();
            me.borrow_mut().take();
        }
        Err(err) => {
            hf.set(false);
            error!(%err, "failed to initialize mpv render context");
            let msg = format!("视频渲染初始化失败：{err}");
            pl.set_text(&msg);
            pb.set_visible(true);
            *fe.borrow_mut() = Some(msg);
        }
    });

    gl_area.connect_resize(|gl_area, _, _| {
        gl_area.queue_render();
    });

    let be2 = render_backend.clone();
    let pb2 = placeholder_box.clone();
    let pl2 = placeholder_label.clone();
    let me2 = media_render_error.clone();
    let hf2 = has_rendered_frame.clone();
    gl_area.connect_render(move |gl_area, _| match be2.render_to_gl_area(gl_area) {
        Ok(true) => {
            hf2.set(true);
            me2.borrow_mut().take();
            pb2.set_visible(false);
            gtk::glib::Propagation::Stop
        }
        Ok(false) => gtk::glib::Propagation::Proceed,
        Err(err) => {
            hf2.set(false);
            error!(%err, "failed to render mpv frame");
            let msg = format!("视频渲染失败：{err}");
            pl2.set_text(&msg);
            pb2.set_visible(true);
            *me2.borrow_mut() = Some(msg);
            gtk::glib::Propagation::Proceed
        }
    });

    gl_area.connect_unrealize(move |_| {
        render_backend.destroy_render_context();
    });
}

fn sync_media_session(last: &mut Option<PathBuf>, current: Option<&Path>) -> bool {
    if last.as_deref() == current {
        return false;
    }
    *last = current.map(Path::to_path_buf);
    true
}

fn active_render_error(fatal: Option<&str>, media: Option<&str>) -> Option<String> {
    fatal.or(media).map(ToOwned::to_owned)
}
