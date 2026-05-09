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
    placeholder_label: gtk::Label,
    controls: PlayerControls,
    pub controls_wrapper: gtk::Box,
    info_label: gtk::Label,
    pub back_button: gtk::Button,
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
        let placeholder_icon = gtk::Image::from_icon_name("video-x-generic-symbolic");
        placeholder_icon.set_pixel_size(80);
        placeholder_icon.add_css_class("placeholder-icon");

        let placeholder_title = gtk::Label::builder()
            .label("未打开媒体文件")
            .css_classes(["placeholder-title"])
            .build();

        let placeholder_label = gtk::Label::builder()
            .label("空格=暂停  ←→=快进  ↑↓=音量  f=全屏\ns=截图  m=静音  []=倍速  q=停止")
            .justify(gtk::Justification::Center)
            .wrap(true)
            .css_classes(["placeholder-hint"])
            .build();

        let placeholder_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(16)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Center)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["placeholder-container"])
            .build();
        placeholder_box.append(&placeholder_icon);
        placeholder_box.append(&placeholder_title);
        placeholder_box.append(&placeholder_label);

        // ── Back button (top-left, like ← in player_view mockup) ──
        let back_button = gtk::Button::builder()
            .icon_name("go-previous-symbolic")
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .margin_start(12)
            .margin_top(12)
            .css_classes(["back-button"])
            .visible(false)
            .build();

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
            .margin_bottom(6)
            .build();
        controls_wrapper.append(controls.widget());

        // ── Playlist panel (right side overlay) ──
        let playlist_panel = PlaylistPanel::new();

        // ── Overlay assembly ──
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&gl_area));
        overlay.add_overlay(&placeholder_box);
        overlay.add_overlay(&back_button);
        overlay.add_overlay(&playlist_panel.root);
        overlay.add_overlay(&controls_wrapper);
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
            placeholder_label,
            controls,
            controls_wrapper,
            info_label,
            back_button,
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
        self.back_button.set_visible(has_media);
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
        }

        let render_error = active_render_error(
            self.fatal_render_error.borrow().as_deref(),
            self.media_render_error.borrow().as_deref(),
        );

        if let Some(err) = render_error {
            self.placeholder_label.set_text(&err);
            self.placeholder_box.set_visible(true);
        } else if has_media {
            if self.has_rendered_frame.get() {
                self.placeholder_box.set_visible(false);
            } else {
                self.placeholder_label.set_text("正在准备视频画面…");
                self.placeholder_box.set_visible(true);
            }
        } else {
            self.has_rendered_frame.set(false);
            self.placeholder_label
                .set_text("空格=暂停  ←→=快进  ↑↓=音量  f=全屏\ns=截图  m=静音  []=倍速  q=停止");
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
    gl_area.connect_realize(move |gl_area| {
        match be.initialize_render_context(gl_area) {
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
