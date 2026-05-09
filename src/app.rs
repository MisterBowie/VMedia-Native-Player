use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};

use adw::prelude::*;
use gtk::gdk;
use tracing::info;

use crate::{
    core::{command::AppCommand, event::AppEvent, state::AppState},
    infra::{config::AppConfig, db::Database, playback_history::PlaybackHistory},
    player::MpvController,
    ui::AppWindow,
};

const CUSTOM_CSS: &str = include_str!("ui/style.css");

pub fn run() -> gtk::glib::ExitCode {
    let config = AppConfig::default();
    let app = adw::Application::builder()
        .application_id(config.app_id)
        .flags(gtk::gio::ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| {
        load_custom_css();
    });

    app.connect_activate(build_ui);

    // Handle command-line file opening: vmedia /path/to/file.mp4
    app.connect_open(|app, files, _| {
        app.activate();

        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                unsafe {
                    app.set_data("vmedia-pending-open", path);
                }
            }
        }
    });

    app.run()
}

fn load_custom_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(CUSTOM_CSS);

    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &adw::Application) {
    // Force dark color scheme
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceDark);

    let _database = Database::new();
    let history = Rc::new(RefCell::new(PlaybackHistory::load()));
    let state = Rc::new(RefCell::new(AppState::default()));
    let player = Rc::new(RefCell::new(MpvController::new()));
    let window_slot = Rc::new(RefCell::new(Weak::<AppWindow>::new()));

    let render_backend = player.borrow().render_backend();
    let state_for_events = state.clone();
    let window_slot_for_events = window_slot.clone();
    let apply_events: Rc<dyn Fn(Vec<AppEvent>)> = Rc::new(move |events| {
        if events.is_empty() {
            return;
        }

        let state_snapshot = {
            let mut app_state = state_for_events.borrow_mut();

            for event in &events {
                if matches!(event, AppEvent::Error(_)) {
                    info!(?event, "command produced error event");
                }
                app_state.apply_event(event);
            }

            app_state.clone()
        };

        if let Some(window) = window_slot_for_events.borrow().upgrade() {
            window.render(&state_snapshot);
        }
    });

    let apply_events_for_commands = apply_events.clone();
    let player_for_handler = player.clone();
    let history_for_handler = history.clone();
    let state_for_handler = state.clone();
    let command_handler: Rc<dyn Fn(AppCommand)> = Rc::new(move |command| {
        info!(?command, "handling command");

        // Before opening a new file, save current playback position
        if let AppCommand::OpenFile(ref new_path) = command {
            let st = state_for_handler.borrow();
            if let Some(ref media) = st.playback.current_media {
                // Skip if double-clicking the same file
                if media.path == *new_path {
                    info!("skipping open: same file already playing");
                    return;
                }
                // Save position of current file
                if st.playback.position_seconds > 2.0 {
                    history_for_handler
                        .borrow_mut()
                        .save_position(&media.path, st.playback.position_seconds);
                    info!(
                        path = %media.path.display(),
                        pos = st.playback.position_seconds,
                        "saved playback position"
                    );
                }
            }
        }

        // Before stopping, save current position
        if matches!(command, AppCommand::Stop) {
            let st = state_for_handler.borrow();
            if let Some(ref media) = st.playback.current_media {
                if st.playback.position_seconds > 2.0 {
                    history_for_handler
                        .borrow_mut()
                        .save_position(&media.path, st.playback.position_seconds);
                }
            }
        }

        let events = player_for_handler.borrow_mut().handle_command(command);
        apply_events_for_commands(events);
    });

    // After loading a file, check history and resume
    let history_for_resume = history.clone();
    let state_for_resume = state.clone();
    let player_for_resume = player.clone();
    let apply_events_for_resume = apply_events.clone();
    let resume_check: Rc<RefCell<Option<std::path::PathBuf>>> = Rc::new(RefCell::new(None));
    let resume_check_for_timer = resume_check.clone();
    // When true, pause after resuming (used for restore-on-startup)
    let pause_after_resume: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let pause_flag_for_timer = pause_after_resume.clone();

    let apply_events_for_backend = apply_events.clone();
    let player_for_backend = player.clone();
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        let events = player_for_backend
            .borrow_mut()
            .handle_pending_backend_updates();

        // Check if we just loaded a new media — if so, try to resume
        for event in &events {
            if let AppEvent::PlaybackLoaded { media } = event {
                let path = media.path.clone();
                // Save as last played media
                history_for_resume.borrow_mut().set_last_media(&path);
                // Schedule resume on next tick (media needs to be fully loaded)
                *resume_check_for_timer.borrow_mut() = Some(path);
            }
        }

        apply_events_for_backend(events);

        // Perform resume seek if pending
        if let Some(path) = resume_check.borrow_mut().take() {
            if let Some(pos) = history_for_resume.borrow().get_position(&path) {
                info!(path = %path.display(), pos, "resuming playback position");
                let seek_events = player_for_resume
                    .borrow_mut()
                    .handle_command(AppCommand::SeekAbsolute(pos));
                apply_events_for_resume(seek_events);
            }
            // If this was a restore-on-startup, pause immediately
            if pause_flag_for_timer.get() {
                pause_flag_for_timer.set(false);
                let pause_events = player_for_resume
                    .borrow_mut()
                    .handle_command(AppCommand::TogglePause);
                apply_events_for_resume(pause_events);
            }
        }

        // Periodically save position (every tick is 50ms, so not too heavy)
        let st = state_for_resume.borrow();
        if let Some(ref media) = st.playback.current_media {
            if st.playback.position_seconds > 2.0 && !st.playback.is_paused {
                // Save every ~5 seconds (100 ticks × 50ms)
                let tick = (st.playback.position_seconds * 20.0) as u64;
                if tick % 100 == 0 {
                    history_for_resume
                        .borrow_mut()
                        .save_position(&media.path, st.playback.position_seconds);
                }
            }
        }

        gtk::glib::ControlFlow::Continue
    });

    let window = Rc::new(AppWindow::new(app, render_backend, command_handler.clone()));
    let initial_state = state.borrow().clone();
    window.render(&initial_state);
    window.present();
    *window_slot.borrow_mut() = Rc::downgrade(&window);

    // Check for pending file from command-line
    let pending_path: Option<std::path::PathBuf> = unsafe { app.steal_data("vmedia-pending-open") };
    if let Some(path) = pending_path {
        info!(?path, "opening file from command line");
        command_handler(AppCommand::OpenFile(path));
    } else {
        // No command-line file: restore last played media
        let last = history.borrow().last_media().map(|p| p.to_path_buf());
        if let Some(path) = last {
            if path.exists() {
                info!(?path, "restoring last played media");
                // Set flag to pause after resume
                pause_after_resume.set(true);
                // Populate the playlist panel from the file's directory
                window.populate_playlist(&path);
                // Open the file (it will auto-resume to saved position via the resume logic)
                command_handler(AppCommand::OpenFile(path));
            }
        }
    }

    // SAFETY: the application owns this per-window session data for the rest of the process.
    unsafe {
        app.set_data("vmedia-native-player-window", window.clone());
    }
}
