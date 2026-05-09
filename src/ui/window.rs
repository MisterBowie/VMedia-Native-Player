use std::{cell::Cell, rc::Rc};

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::{
    core::{command::AppCommand, state::AppState},
    infra::config::AppConfig,
    player::libmpv::LibMpv,
};

use super::player_view::PlayerView;

type CommandHandler = Rc<dyn Fn(AppCommand)>;

#[derive(Clone)]
pub struct AppWindow {
    window: adw::ApplicationWindow,
    player_view: PlayerView,
}

impl AppWindow {
    pub fn new(
        app: &adw::Application,
        render_backend: Option<LibMpv>,
        on_command: CommandHandler,
    ) -> Self {
        let config = AppConfig::default();
        let player_view = PlayerView::new(render_backend);

        let header_bar = adw::HeaderBar::new();
        header_bar.set_title_widget(Some(
            &gtk::Label::builder()
                .label(config.window_title)
                .css_classes(["heading"])
                .build(),
        ));

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(player_view.widget()));

        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(config.window_title)
            .default_width(config.default_width)
            .default_height(config.default_height)
            .content(&toolbar_view)
            .build();

        let controls = player_view.controls();
        connect_open_button(&window, controls, on_command.clone());
        connect_transport_buttons(&window, controls, on_command.clone());
        connect_seek_bar(controls, on_command.clone());
        connect_speed_button(controls, on_command.clone());
        connect_mute_button(controls, on_command.clone());
        // Back button → Stop
        let cmd = on_command.clone();
        player_view
            .back_button
            .connect_clicked(move |_| cmd(AppCommand::Stop));
        connect_right_click_menu(&window, &player_view, on_command.clone());
        connect_video_click(&player_view, &window, on_command.clone());
        connect_keyboard_shortcuts(&window, controls, on_command.clone());

        // Playlist toggle button
        let panel = player_view.playlist_panel.clone();
        controls
            .playlist_button
            .connect_clicked(move |_| panel.toggle());

        // Playlist row click → open file
        let cmd = on_command;
        player_view
            .playlist_panel
            .connect_activate(move |path| cmd(AppCommand::OpenFile(path)));

        // ── Controls auto-hide + drag ──
        connect_controls_autohide(&player_view);
        connect_controls_drag(&player_view);
        // Set initial width to 80% of window
        connect_controls_resize(&window, &player_view);

        Self {
            window,
            player_view,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn render(&self, state: &AppState) {
        self.player_view.render(state);

        if state.playback.is_fullscreen {
            self.window.fullscreen();
        } else {
            self.window.unfullscreen();
        }
    }

    /// Populate the playlist panel from the directory of the given file.
    pub fn populate_playlist(&self, path: &std::path::Path) {
        self.player_view.playlist_panel.update_for_media(path);
    }
}

fn connect_open_button(
    window: &adw::ApplicationWindow,
    controls: &super::widgets::PlayerControls,
    on_command: CommandHandler,
) {
    let window = window.clone();
    controls.open_button.connect_clicked(move |_button| {
        let cmd = on_command.clone();
        let dialog = gtk::FileDialog::builder()
            .title("打开本地媒体文件")
            .modal(true)
            .build();

        // Add media file filters
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        let media_filter = gtk::FileFilter::new();
        media_filter.set_name(Some("媒体文件"));
        media_filter.add_mime_type("video/*");
        media_filter.add_mime_type("audio/*");
        media_filter.add_suffix("mkv");
        media_filter.add_suffix("mp4");
        media_filter.add_suffix("avi");
        media_filter.add_suffix("mov");
        media_filter.add_suffix("wmv");
        media_filter.add_suffix("flv");
        media_filter.add_suffix("webm");
        media_filter.add_suffix("mp3");
        media_filter.add_suffix("flac");
        media_filter.add_suffix("wav");
        media_filter.add_suffix("ogg");
        media_filter.add_suffix("m4a");
        filters.append(&media_filter);

        let all_filter = gtk::FileFilter::new();
        all_filter.set_name(Some("所有文件"));
        all_filter.add_pattern("*");
        filters.append(&all_filter);

        dialog.set_filters(Some(&filters));
        dialog.set_default_filter(Some(&media_filter));

        dialog.open(Some(&window), gio::Cancellable::NONE, move |result| {
            if let Ok(file) = result {
                if let Some(path) = file.path() {
                    cmd(AppCommand::OpenFile(path));
                }
            }
        });
    });
}

fn connect_transport_buttons(
    window: &adw::ApplicationWindow,
    controls: &super::widgets::PlayerControls,
    on_command: CommandHandler,
) {
    let cmd = on_command.clone();
    controls
        .play_pause_button
        .connect_clicked(move |_| cmd(AppCommand::TogglePause));

    let cmd = on_command.clone();
    controls
        .backward_button
        .connect_clicked(move |_| cmd(AppCommand::SeekRelative(-10.0)));

    let cmd = on_command.clone();
    controls
        .forward_button
        .connect_clicked(move |_| cmd(AppCommand::SeekRelative(10.0)));

    let w = window.clone();
    let cmd = on_command.clone();
    controls
        .fullscreen_button
        .connect_clicked(move |_| cmd(AppCommand::SetFullscreen(!w.is_fullscreen())));

    let cmd = on_command;
    controls.bind_volume(move |v| cmd(AppCommand::SetVolume(v)));
}

fn connect_seek_bar(controls: &super::widgets::PlayerControls, on_command: CommandHandler) {
    controls
        .seek_bar()
        .bind_seek(move |pos| on_command(AppCommand::SeekAbsolute(pos)));
}

fn connect_speed_button(
    controls: &super::widgets::PlayerControls,
    on_command: CommandHandler,
) {
    let speeds = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
    let idx = std::cell::Cell::new(2usize);
    controls.speed_button.connect_clicked(move |_| {
        let next = (idx.get() + 1) % speeds.len();
        idx.set(next);
        on_command(AppCommand::SetSpeed(speeds[next]));
    });
}

fn connect_mute_button(
    controls: &super::widgets::PlayerControls,
    on_command: CommandHandler,
) {
    controls
        .mute_button
        .connect_clicked(move |_| on_command(AppCommand::ToggleMute));
}

fn connect_right_click_menu(
    window: &adw::ApplicationWindow,
    player_view: &PlayerView,
    on_command: CommandHandler,
) {
    let group = gio::SimpleActionGroup::new();

    let cmd = on_command.clone();
    let a = gio::SimpleAction::new("toggle-pause", None);
    a.connect_activate(move |_, _| cmd(AppCommand::TogglePause));
    group.add_action(&a);

    let cmd = on_command.clone();
    let a = gio::SimpleAction::new("screenshot", None);
    a.connect_activate(move |_, _| cmd(AppCommand::Screenshot));
    group.add_action(&a);

    let cmd = on_command.clone();
    let a = gio::SimpleAction::new("stop", None);
    a.connect_activate(move |_, _| cmd(AppCommand::Stop));
    group.add_action(&a);

    for speed in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0] {
        let cmd = on_command.clone();
        let name = format!("speed-{}", (speed * 100.0) as u32);
        let a = gio::SimpleAction::new(&name, None);
        a.connect_activate(move |_, _| cmd(AppCommand::SetSpeed(speed)));
        group.add_action(&a);
    }

    window.insert_action_group("player", Some(&group));

    let menu = gio::Menu::new();
    menu.append(Some("暂停/继续"), Some("player.toggle-pause"));

    let speed_sub = gio::Menu::new();
    for s in [0.5, 0.75, 1.0, 1.25, 1.5, 2.0] {
        let name = format!("speed-{}", (s * 100.0) as u32);
        speed_sub.append(Some(&format!("{s:.2}x")), Some(&format!("player.{name}")));
    }
    menu.append_submenu(Some("倍速播放"), &speed_sub);
    menu.append(Some("截图"), Some("player.screenshot"));
    menu.append(Some("停止播放"), Some("player.stop"));

    let popover = gtk::PopoverMenu::from_model(Some(&menu));
    popover.set_parent(player_view.widget());
    popover.set_has_arrow(false);

    let gesture = gtk::GestureClick::builder().button(3).build();
    let pop = popover.clone();
    gesture.connect_pressed(move |g, _, x, y| {
        pop.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        pop.popup();
        g.set_state(gtk::EventSequenceState::Claimed);
    });
    player_view.widget().add_controller(gesture);
}

fn connect_video_click(
    player_view: &PlayerView,
    window: &adw::ApplicationWindow,
    on_command: CommandHandler,
) {
    // Double click on VIDEO area toggles fullscreen
    let dbl = gtk::GestureClick::builder().button(1).build();
    let w = window.clone();
    let cmd = on_command.clone();
    let panel = player_view.playlist_panel.clone();
    dbl.connect_released(move |g, n, _, _| {
        if n == 2 {
            cmd(AppCommand::SetFullscreen(!w.is_fullscreen()));
            g.set_state(gtk::EventSequenceState::Claimed);
        } else if n == 1 && panel.root.is_visible() {
            // Single click on video hides playlist
            panel.root.set_visible(false);
        }
    });
    player_view.gl_area.add_controller(dbl);

    // Scroll on video for volume
    let cmd = on_command;
    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    scroll.connect_scroll(move |_, _, dy| {
        let delta = if dy < 0.0 { 5.0 } else { -5.0 };
        cmd(AppCommand::SetVolume(50.0 + delta)); // TODO: track current
        glib::Propagation::Stop
    });
    player_view.gl_area.add_controller(scroll);
}

fn connect_keyboard_shortcuts(
    window: &adw::ApplicationWindow,
    controls: &super::widgets::PlayerControls,
    on_command: CommandHandler,
) {
    let w = window.clone();
    let vol = controls.volume_scale.clone();
    let ctrl = gtk::EventControllerKey::new();
    ctrl.set_propagation_phase(gtk::PropagationPhase::Capture);

    // Track current speed for [ ] shortcuts (±0.25 steps)
    let current_speed = std::rc::Rc::new(std::cell::Cell::new(1.0_f64));

    ctrl.connect_key_pressed(move |_, key, _keycode, modifier| {
        // Don't intercept when user is interacting with scale widgets
        if let Some(focus) = gtk::prelude::GtkWindowExt::focus(&w) {
            if focus.is::<gtk::Scale>() {
                return glib::Propagation::Proceed;
            }
        }

        // Only handle unmodified keys (no Ctrl/Alt combos)
        if modifier.contains(gdk::ModifierType::CONTROL_MASK)
            || modifier.contains(gdk::ModifierType::ALT_MASK)
        {
            return glib::Propagation::Proceed;
        }

        match key {
            gdk::Key::space => {
                on_command(AppCommand::TogglePause);
                glib::Propagation::Stop
            }
            gdk::Key::Left => {
                on_command(AppCommand::SeekRelative(-10.0));
                glib::Propagation::Stop
            }
            gdk::Key::Right => {
                on_command(AppCommand::SeekRelative(10.0));
                glib::Propagation::Stop
            }
            gdk::Key::Up => {
                let v = (vol.value() + 5.0).clamp(0.0, 100.0);
                vol.set_value(v);
                on_command(AppCommand::SetVolume(v));
                glib::Propagation::Stop
            }
            gdk::Key::Down => {
                let v = (vol.value() - 5.0).clamp(0.0, 100.0);
                vol.set_value(v);
                on_command(AppCommand::SetVolume(v));
                glib::Propagation::Stop
            }
            gdk::Key::f | gdk::Key::F => {
                on_command(AppCommand::SetFullscreen(!w.is_fullscreen()));
                glib::Propagation::Stop
            }
            gdk::Key::Escape => {
                if w.is_fullscreen() {
                    on_command(AppCommand::SetFullscreen(false));
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
            gdk::Key::s | gdk::Key::S => {
                on_command(AppCommand::Screenshot);
                glib::Propagation::Stop
            }
            gdk::Key::m | gdk::Key::M => {
                on_command(AppCommand::ToggleMute);
                glib::Propagation::Stop
            }
            gdk::Key::q | gdk::Key::Q => {
                on_command(AppCommand::Stop);
                glib::Propagation::Stop
            }
            // [ = speed down 0.25
            gdk::Key::bracketleft => {
                let cur = current_speed.get();
                let new_speed = (cur - 0.25_f64).max(0.25);
                current_speed.set(new_speed);
                on_command(AppCommand::SetSpeed(new_speed));
                glib::Propagation::Stop
            }
            // ] = speed up 0.25
            gdk::Key::bracketright => {
                let cur = current_speed.get();
                let new_speed = (cur + 0.25_f64).min(4.0);
                current_speed.set(new_speed);
                on_command(AppCommand::SetSpeed(new_speed));
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    window.add_controller(ctrl);
}

/// Auto-hide controls: show on any mouse move, hide 1.5s after mouse stops.
/// Only hides when media is playing. Does not hide while seek bar is dragged.
fn connect_controls_autohide(player_view: &PlayerView) {
    let wrapper = player_view.controls_wrapper.clone();
    let hide_timer: Rc<std::cell::RefCell<Option<glib::SourceId>>> =
        Rc::new(std::cell::RefCell::new(None));

    let show_controls = {
        let wrapper = wrapper.clone();
        move || {
            wrapper.set_opacity(1.0);
            wrapper.set_can_target(true);
        }
    };

    let hide_controls = {
        let wrapper = wrapper.clone();
        move || {
            wrapper.set_opacity(0.0);
            wrapper.set_can_target(false);
        }
    };

    // Cancel existing timer safely
    let cancel_timer = {
        let hide_timer = hide_timer.clone();
        move || {
            if let Some(id) = hide_timer.borrow_mut().take() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| id.remove()));
            }
        }
    };

    // Mouse motion/enter on overlay → show + reset 1.5s timer
    // Use Capture phase so events are seen before GLArea child consumes them
    let motion = gtk::EventControllerMotion::new();
    motion.set_propagation_phase(gtk::PropagationPhase::Capture);
    let show = show_controls;
    let cancel = cancel_timer;
    let hide = hide_controls;
    let timer = hide_timer;
    let seek_bar = player_view.controls().seek_bar().clone();
    let w = wrapper;
    let show_and_schedule = {
        let show = show.clone();
        let cancel = cancel.clone();
        let hide = hide.clone();
        let timer = timer.clone();
        let seek_bar = seek_bar.clone();
        let w = w.clone();
        move || {
            show();
            cancel();
            let hc = hide.clone();
            let tr = timer.clone();
            let sb = seek_bar.clone();
            let wr = w.clone();
            let id = glib::timeout_add_local_once(
                std::time::Duration::from_millis(1500),
                move || {
                    tr.borrow_mut().take();
                    if sb.is_dragging() { return; }
                    if wr.has_css_class("has-media") { hc(); }
                },
            );
            *timer.borrow_mut() = Some(id);
        }
    };
    let ss1 = show_and_schedule.clone();
    motion.connect_motion(move |_, _x, _y| { ss1(); });
    let ss2 = show_and_schedule;
    motion.connect_enter(move |_, _x, _y| { ss2(); });
    player_view.widget().add_controller(motion);
}

/// Make controls draggable: GestureDrag on the OVERLAY (stable coordinates).
fn connect_controls_drag(player_view: &PlayerView) {
    let wrapper = player_view.controls_wrapper.clone();
    let overlay = player_view.widget().clone();

    // Track whether we're dragging the controls vs the video
    let is_dragging_controls = Rc::new(Cell::new(false));
    let drag_start_pos: Rc<Cell<(f64, f64)>> = Rc::new(Cell::new((0.0, 0.0)));
    let initial_margins: Rc<Cell<(i32, i32)>> = Rc::new(Cell::new((0, 0)));

    let drag = gtk::GestureDrag::builder().button(1).build();

    // drag_begin: check if press point is within controls_wrapper
    {
        let wrapper = wrapper.clone();
        let overlay = overlay.clone();
        let is_dc = is_dragging_controls.clone();
        let start = drag_start_pos.clone();
        let margins = initial_margins.clone();
        drag.connect_drag_begin(move |_, x, y| {
            let w_alloc = wrapper.allocation();
            let pt = gtk::graphene::Point::new(x as f32, y as f32);
            if let Some(wp) = overlay.compute_point(&wrapper, &pt) {
                let ww = w_alloc.width() as f64;
                let wh = w_alloc.height() as f64;
                let wx = wp.x() as f64;
                let wy = wp.y() as f64;
                if wx >= 0.0 && wx <= ww && wy >= 0.0 && wy <= wh {
                    is_dc.set(true);
                    start.set((x, y));
                    // Compute actual position from allocation (works before switching alignment)
                    let real_x = w_alloc.x();
                    let real_y = w_alloc.y();
                    // Switch to absolute positioning NOW, using the real position
                    wrapper.set_halign(gtk::Align::Start);
                    wrapper.set_valign(gtk::Align::Start);
                    wrapper.set_margin_start(real_x);
                    wrapper.set_margin_top(real_y);
                    wrapper.set_margin_bottom(0);
                    wrapper.set_margin_end(0);
                    margins.set((real_x, real_y));
                    return;
                }
            }
            is_dc.set(false);
        });
    }

    // drag_update: move controls (coordinates are in overlay space, stable)
    {
        let wrapper = wrapper.clone();
        let overlay = overlay.clone();
        let is_dc = is_dragging_controls.clone();
        let start = drag_start_pos.clone();
        let margins = initial_margins.clone();
        drag.connect_drag_update(move |_, dx, dy| {
            if !is_dc.get() {
                return;
            }
            let (ms, mt) = margins.get();

            let ow = overlay.width();
            let oh = overlay.height();
            let cw = wrapper.width();
            let ch = wrapper.height();

            let new_x = (ms as f64 + dx).clamp(0.0, (ow - cw).max(0) as f64) as i32;
            let new_y = (mt as f64 + dy).clamp(0.0, (oh - ch).max(0) as f64) as i32;

            wrapper.set_margin_start(new_x);
            wrapper.set_margin_top(new_y);
            wrapper.set_margin_bottom(0);
            wrapper.set_margin_end(0);
        });
    }

    // drag_end: reset flag
    {
        let is_dc = is_dragging_controls;
        drag.connect_drag_end(move |_, _, _| {
            is_dc.set(false);
        });
    }

    overlay.add_controller(drag);
}

/// Keep controls width at 60% of window and reset position on resize.
fn connect_controls_resize(_window: &adw::ApplicationWindow, player_view: &PlayerView) {
    let wrapper = player_view.controls_wrapper.clone();
    let overlay = player_view.widget().clone();

    // Initial sizing
    {
        let wrapper = wrapper.clone();
        let overlay = overlay.clone();
        glib::idle_add_local_once(move || {
            let ow = overlay.width();
            if ow > 100 {
                let target = (ow as f64 * 0.60).min(600.0) as i32;
                wrapper.set_size_request(target, -1);
            }
        });
    }

    // Track size changes via a periodic check in the render loop.
    // Use notify on the native surface to detect size changes.
    let last_width: Rc<Cell<i32>> = Rc::new(Cell::new(0));
    let wrapper2 = wrapper;
    let overlay2 = overlay;
    // Poll every frame using add_tick_callback for resize
    overlay2.add_tick_callback(move |ov, _| {
        let ow = ov.width();
        if ow != last_width.get() && ow > 100 {
            last_width.set(ow);
            let target = (ow as f64 * 0.60).min(600.0) as i32;
            wrapper2.set_size_request(target, -1);
            // Reset to centered-bottom so controls are never off-screen
            wrapper2.set_halign(gtk::Align::Center);
            wrapper2.set_valign(gtk::Align::End);
            wrapper2.set_margin_start(0);
            wrapper2.set_margin_top(0);
            wrapper2.set_margin_end(0);
            wrapper2.set_margin_bottom(6);
        }
        glib::ControlFlow::Continue
    });
}
