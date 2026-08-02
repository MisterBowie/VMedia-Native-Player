use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::{
    core::{command::AppCommand, state::AppState},
    infra::{
        config::AppConfig,
        shortcut_settings::{ShortcutAction, ShortcutSettings},
    },
    player::libmpv::LibMpv,
};

use super::player_view::PlayerView;

type CommandHandler = Rc<dyn Fn(AppCommand)>;

#[derive(Clone)]
pub struct AppWindow {
    window: adw::ApplicationWindow,
    toolbar_view: adw::ToolbarView,
    title_label: gtk::Label,
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

        let title_label = gtk::Label::builder()
            .label("VMedia")
            .ellipsize(gtk::pango::EllipsizeMode::Middle)
            .max_width_chars(64)
            .css_classes(["app-title"])
            .build();

        let header_bar = adw::HeaderBar::new();
        header_bar.add_css_class("app-header");
        header_bar.set_title_widget(Some(&title_label));

        let shortcut_settings_button = gtk::Button::builder()
            .icon_name("preferences-system-symbolic")
            .tooltip_text("快捷键设置")
            .build();
        header_bar.pack_end(&shortcut_settings_button);

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
        window.set_icon_name(Some("vmedia"));
        window.add_css_class("vmedia-window");

        let shortcut_settings = Rc::new(RefCell::new(ShortcutSettings::load()));
        let controls = player_view.controls();
        connect_open_button(&window, controls, on_command.clone());
        connect_empty_open_button(&player_view, controls);
        connect_transport_buttons(&window, controls, on_command.clone());
        connect_seek_bar(controls, on_command.clone());
        connect_speed_button(controls, on_command.clone());
        connect_mute_button(controls, on_command.clone());
        connect_shortcut_settings_button(
            &window,
            &shortcut_settings_button,
            shortcut_settings.clone(),
        );
        connect_shortcut_settings_action(app, &shortcut_settings_button);
        connect_right_click_menu(
            &window,
            &player_view,
            &shortcut_settings_button,
            on_command.clone(),
        );
        connect_video_click(&player_view, &window, on_command.clone());
        connect_keyboard_shortcuts(
            &window,
            controls,
            &shortcut_settings_button,
            shortcut_settings,
            on_command.clone(),
        );

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
            toolbar_view,
            title_label,
            player_view,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn render(&self, state: &AppState) {
        self.player_view.render(state);

        let title = state
            .playback
            .current_media
            .as_ref()
            .and_then(|media| media.path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("VMedia");
        self.title_label.set_text(title);
        self.window.set_title(Some(title));
        self.toolbar_view
            .set_reveal_top_bars(!state.playback.is_fullscreen);

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

fn connect_empty_open_button(player_view: &PlayerView, controls: &super::widgets::PlayerControls) {
    let open_button = controls.open_button.clone();
    player_view
        .empty_open_button
        .connect_clicked(move |_| open_button.emit_clicked());
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
        .connect_clicked(move |_| cmd(AppCommand::SeekRelative(-5.0)));

    let cmd = on_command.clone();
    controls
        .forward_button
        .connect_clicked(move |_| cmd(AppCommand::SeekRelative(5.0)));

    let w = window.clone();
    let cmd = on_command.clone();
    controls
        .fullscreen_button
        .connect_clicked(move |_| cmd(AppCommand::SetFullscreen(!w.is_fullscreen())));

    let cmd = on_command;
    controls.bind_volume(move |v| cmd(AppCommand::SetVolume(v)));
}

fn connect_seek_bar(controls: &super::widgets::PlayerControls, on_command: CommandHandler) {
    const PREVIEW_INTERVAL: std::time::Duration = std::time::Duration::from_millis(80);

    let pending_preview = Rc::new(Cell::new(None::<f64>));
    let preview_timer: Rc<std::cell::RefCell<Option<glib::SourceId>>> =
        Rc::new(std::cell::RefCell::new(None));

    let preview_controls = controls.clone();
    let preview_command = on_command.clone();
    let pending_for_preview = pending_preview.clone();
    let timer_for_preview = preview_timer.clone();

    let commit_controls = controls.clone();
    let pending_for_commit = pending_preview;
    let timer_for_commit = preview_timer;

    controls.seek_bar().bind_seek(
        move |pos| {
            // The thumb and labels update immediately, independently of mpv.
            preview_controls.preview_seek_position(pos);
            pending_for_preview.set(Some(pos));

            // Coalesce rapid pointer updates into at most one keyframe seek
            // every 80ms, always using the newest requested position.
            if timer_for_preview.borrow().is_some() {
                return;
            }

            let pending = pending_for_preview.clone();
            let timer = timer_for_preview.clone();
            let command = preview_command.clone();
            let id = glib::timeout_add_local_once(PREVIEW_INTERVAL, move || {
                timer.borrow_mut().take();
                if let Some(latest) = pending.take() {
                    command(AppCommand::SeekPreview(latest));
                }
            });
            *timer_for_preview.borrow_mut() = Some(id);
        },
        move |pos| {
            // Release cancels any queued preview and performs one exact seek.
            if let Some(id) = timer_for_commit.borrow_mut().take() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| id.remove()));
            }
            pending_for_commit.set(None);
            commit_controls.preview_seek_position(pos);
            on_command(AppCommand::SeekAbsolute(pos));
        },
    );
}

fn connect_speed_button(controls: &super::widgets::PlayerControls, on_command: CommandHandler) {
    let speeds = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];
    let idx = std::cell::Cell::new(2usize);
    controls.speed_button.connect_clicked(move |_| {
        let next = (idx.get() + 1) % speeds.len();
        idx.set(next);
        on_command(AppCommand::SetSpeed(speeds[next]));
    });
}

fn connect_mute_button(controls: &super::widgets::PlayerControls, on_command: CommandHandler) {
    controls
        .mute_button
        .connect_clicked(move |_| on_command(AppCommand::ToggleMute));
}

fn connect_shortcut_settings_button(
    window: &adw::ApplicationWindow,
    button: &gtk::Button,
    settings: Rc<RefCell<ShortcutSettings>>,
) {
    let window = window.clone();
    button.connect_clicked(move |_| {
        show_shortcut_preferences(&window, settings.clone());
    });
}

fn connect_shortcut_settings_action(app: &adw::Application, button: &gtk::Button) {
    let button = button.clone();
    let action = gio::SimpleAction::new("shortcut-settings", None);
    action.connect_activate(move |_, _| button.emit_clicked());
    app.add_action(&action);
}

fn show_shortcut_preferences(
    parent: &adw::ApplicationWindow,
    settings: Rc<RefCell<ShortcutSettings>>,
) {
    let application = parent.application().expect("application window");
    let preferences = adw::Window::builder()
        .application(&application)
        .title("设置")
        .default_width(1040)
        .default_height(760)
        .transient_for(parent)
        .modal(true)
        .build();
    preferences.add_css_class("shortcut-settings-window");

    let header_bar = adw::HeaderBar::new();
    header_bar.add_css_class("shortcut-settings-header");
    header_bar.set_title_widget(Some(
        &gtk::Label::builder()
            .label("设置")
            .css_classes(["shortcut-window-title"])
            .build(),
    ));

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("shortcut-settings-root");

    let sidebar = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(16)
        .css_classes(["shortcut-sidebar"])
        .build();
    sidebar.set_size_request(250, -1);

    let search_entry = gtk::SearchEntry::builder()
        .placeholder_text("搜索")
        .css_classes(["shortcut-search"])
        .build();
    sidebar.append(&search_entry);

    let sidebar_item_content = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(12)
        .build();
    let sidebar_icon = gtk::Image::from_icon_name("input-keyboard-symbolic");
    sidebar_icon.set_pixel_size(19);
    let sidebar_label = gtk::Label::builder()
        .label("快捷键")
        .xalign(0.0)
        .hexpand(true)
        .build();
    sidebar_item_content.append(&sidebar_icon);
    sidebar_item_content.append(&sidebar_label);

    let sidebar_item = gtk::Button::builder()
        .child(&sidebar_item_content)
        .css_classes(["shortcut-sidebar-item", "selected"])
        .build();
    let search_for_sidebar = search_entry.clone();
    sidebar_item.connect_clicked(move |_| {
        search_for_sidebar.grab_focus();
    });
    sidebar.append(&sidebar_item);
    root.append(&sidebar);

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .css_classes(["shortcut-settings-content"])
        .build();
    content.set_margin_top(38);
    content.set_margin_bottom(38);
    content.set_margin_start(46);
    content.set_margin_end(46);

    let title = gtk::Label::builder()
        .label("快捷键")
        .xalign(0.0)
        .css_classes(["shortcut-page-title"])
        .build();
    let description = gtk::Label::builder()
        .label("点击快捷键按钮，然后按下新的按键组合。修改后立即生效。")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["shortcut-page-description"])
        .build();
    content.append(&title);
    content.append(&description);

    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .spacing(28)
        .css_classes(["shortcut-section"])
        .build();
    section.set_margin_top(24);

    let section_title = gtk::Label::builder()
        .label("播放器：")
        .xalign(0.0)
        .yalign(0.0)
        .css_classes(["shortcut-section-title"])
        .build();
    section_title.set_size_request(112, -1);
    section.append(&section_title);

    let shortcut_column = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .hexpand(true)
        .build();

    let shortcut_list = gtk::ListBox::new();
    shortcut_list.set_selection_mode(gtk::SelectionMode::None);
    shortcut_list.set_show_separators(true);
    shortcut_list.add_css_class("shortcut-settings-list");

    let mut shortcut_labels = Vec::new();
    let mut searchable_rows = Vec::new();
    for action in ShortcutAction::ALL {
        let accelerator = settings
            .borrow()
            .binding(action)
            .unwrap_or_default()
            .to_string();
        let shortcut_label = gtk::Label::builder()
            .label(shortcut_display_text(Some(&accelerator)))
            .halign(gtk::Align::Center)
            .css_classes(["shortcut-binding-label"])
            .build();

        let binding_button = gtk::Button::builder()
            .child(&shortcut_label)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Center)
            .tooltip_text("点击修改快捷键")
            .css_classes(["shortcut-binding-button"])
            .build();

        let action_label = gtk::Label::builder()
            .label(action.label())
            .xalign(0.0)
            .hexpand(true)
            .css_classes(["shortcut-action-label"])
            .build();

        let row_content = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(20)
            .css_classes(["shortcut-setting-row-content"])
            .build();
        row_content.append(&action_label);
        row_content.append(&binding_button);

        let row = gtk::ListBoxRow::builder()
            .child(&row_content)
            .activatable(false)
            .selectable(false)
            .css_classes(["shortcut-setting-row"])
            .build();

        let preferences = preferences.clone();
        let label_for_capture = shortcut_label.clone();
        let settings_for_capture = settings.clone();
        binding_button.connect_clicked(move |_| {
            show_shortcut_capture_dialog(
                &preferences,
                action,
                &label_for_capture,
                settings_for_capture.clone(),
            );
        });
        shortcut_list.append(&row);

        shortcut_labels.push((action, shortcut_label));
        searchable_rows.push((action.label().to_lowercase(), row));
    }

    let shortcut_labels = Rc::new(shortcut_labels);
    let searchable_rows = Rc::new(searchable_rows);

    let rows_for_search = searchable_rows;
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().trim().to_lowercase();
        for (label, row) in rows_for_search.iter() {
            row.set_visible(query.is_empty() || label.contains(&query));
        }
    });

    let hint_label = gtk::Label::builder()
        .label("Esc 始终用于退出全屏；在录入窗口中按 Backspace 可清除绑定。")
        .xalign(0.0)
        .wrap(true)
        .css_classes(["shortcut-settings-hint"])
        .build();

    let reset_button = gtk::Button::builder()
        .label("恢复默认")
        .halign(gtk::Align::End)
        .css_classes(["shortcut-reset-button"])
        .tooltip_text("恢复全部默认快捷键")
        .build();

    let labels_for_reset = shortcut_labels;
    let settings_for_reset = settings;
    reset_button.connect_clicked(move |_| {
        settings_for_reset.borrow_mut().reset();
        persist_shortcut_settings(&settings_for_reset);

        let current_settings = settings_for_reset.borrow();
        for (action, label) in labels_for_reset.iter() {
            label.set_text(&shortcut_display_text(current_settings.binding(*action)));
        }
    });

    shortcut_column.append(&shortcut_list);
    shortcut_column.append(&hint_label);
    shortcut_column.append(&reset_button);
    section.append(&shortcut_column);
    content.append(&section);

    let content_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&content)
        .css_classes(["shortcut-content-scroll"])
        .build();
    root.append(&content_scroll);

    toolbar_view.set_content(Some(&root));
    preferences.set_content(Some(&toolbar_view));
    preferences.present();
}

#[allow(deprecated)]
fn show_shortcut_capture_dialog(
    parent: &adw::Window,
    action: ShortcutAction,
    shortcut_label: &gtk::Label,
    settings: Rc<RefCell<ShortcutSettings>>,
) {
    let heading = format!("设置“{}”快捷键", action.label());
    let dialog = adw::MessageDialog::new(
        Some(parent),
        Some(&heading),
        Some("按下新的按键组合。按 Esc 取消，按 Backspace 清除当前快捷键。"),
    );
    dialog.add_response("cancel", "取消");
    dialog.set_close_response("cancel");

    let status_label = gtk::Label::builder()
        .label("等待输入…")
        .wrap(true)
        .css_classes(["shortcut-capture-status"])
        .build();
    dialog.set_extra_child(Some(&status_label));

    let controller = gtk::EventControllerKey::new();
    controller.set_propagation_phase(gtk::PropagationPhase::Capture);
    let dialog_for_key = dialog.clone();
    let label_for_key = shortcut_label.clone();
    controller.connect_key_pressed(move |_, key, _, modifier| {
        if key == gdk::Key::Escape {
            dialog_for_key.close();
            return glib::Propagation::Stop;
        }

        if key == gdk::Key::BackSpace {
            settings.borrow_mut().set_binding(action, None);
            persist_shortcut_settings(&settings);
            label_for_key.set_text("未设置");
            dialog_for_key.close();
            return glib::Propagation::Stop;
        }

        if is_modifier_key(key) {
            status_label.set_text("请继续按下一个非修饰键");
            status_label.remove_css_class("error");
            return glib::Propagation::Stop;
        }

        let modifiers = normalized_shortcut_modifiers(modifier);
        let accelerator = gtk::accelerator_name(key, modifiers).to_string();
        let conflict = {
            let current_settings = settings.borrow();
            ShortcutAction::ALL.into_iter().find(|candidate| {
                *candidate != action
                    && current_settings
                        .binding(*candidate)
                        .is_some_and(|binding| accelerators_equal(binding, &accelerator))
            })
        };

        if let Some(conflict) = conflict {
            status_label.set_text(&format!(
                "该快捷键已被“{}”使用，请选择其他按键。",
                conflict.label()
            ));
            status_label.add_css_class("error");
            return glib::Propagation::Stop;
        }

        settings
            .borrow_mut()
            .set_binding(action, Some(accelerator.clone()));
        persist_shortcut_settings(&settings);
        label_for_key.set_text(&shortcut_display_text(Some(&accelerator)));
        dialog_for_key.close();
        glib::Propagation::Stop
    });
    dialog.add_controller(controller);
    dialog.present();
}

fn shortcut_display_text(accelerator: Option<&str>) -> String {
    accelerator
        .and_then(gtk::accelerator_parse)
        .map(|(key, modifiers)| gtk::accelerator_get_label(key, modifiers).to_string())
        .filter(|label| !label.is_empty())
        .unwrap_or_else(|| "未设置".to_string())
}

fn persist_shortcut_settings(settings: &Rc<RefCell<ShortcutSettings>>) {
    if let Err(error) = settings.borrow().save() {
        tracing::warn!(%error, "failed to save shortcut settings");
    }
}

fn normalized_shortcut_modifiers(modifiers: gdk::ModifierType) -> gdk::ModifierType {
    modifiers & gtk::accelerator_get_default_mod_mask()
}

fn accelerators_equal(left: &str, right: &str) -> bool {
    let Some((left_key, left_modifiers)) = gtk::accelerator_parse(left) else {
        return false;
    };
    let Some((right_key, right_modifiers)) = gtk::accelerator_parse(right) else {
        return false;
    };

    left_key.to_lower() == right_key.to_lower()
        && normalized_shortcut_modifiers(left_modifiers)
            == normalized_shortcut_modifiers(right_modifiers)
}

fn accelerator_matches_event(
    accelerator: &str,
    key: gdk::Key,
    modifiers: gdk::ModifierType,
) -> bool {
    let Some((shortcut_key, shortcut_modifiers)) = gtk::accelerator_parse(accelerator) else {
        return false;
    };

    shortcut_key.to_lower() == key.to_lower()
        && normalized_shortcut_modifiers(shortcut_modifiers) == modifiers
}

fn is_modifier_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
            | gdk::Key::ISO_Level3_Shift
            | gdk::Key::Caps_Lock
            | gdk::Key::Num_Lock
    )
}

fn connect_right_click_menu(
    window: &adw::ApplicationWindow,
    player_view: &PlayerView,
    shortcut_settings_button: &gtk::Button,
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

    let settings_button = shortcut_settings_button.clone();
    let a = gio::SimpleAction::new("shortcut-settings", None);
    a.connect_activate(move |_, _| settings_button.emit_clicked());
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
    menu.append(Some("快捷键设置"), Some("player.shortcut-settings"));

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
    shortcut_settings_button: &gtk::Button,
    shortcut_settings: Rc<RefCell<ShortcutSettings>>,
    on_command: CommandHandler,
) {
    let w = window.clone();
    let vol = controls.volume_scale.clone();
    let open_button = controls.open_button.clone();
    let playlist_button = controls.playlist_button.clone();
    let settings_button = shortcut_settings_button.clone();
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

        // Escape is always available as an emergency way to leave fullscreen.
        if key == gdk::Key::Escape && w.is_fullscreen() {
            on_command(AppCommand::SetFullscreen(false));
            return glib::Propagation::Stop;
        }

        let modifiers = normalized_shortcut_modifiers(modifier);
        let action = {
            let settings = shortcut_settings.borrow();
            ShortcutAction::ALL.into_iter().find(|action| {
                settings.binding(*action).is_some_and(|accelerator| {
                    accelerator_matches_event(accelerator, key, modifiers)
                })
            })
        };

        match action {
            Some(ShortcutAction::TogglePause) => {
                on_command(AppCommand::TogglePause);
                glib::Propagation::Stop
            }
            Some(ShortcutAction::SeekBackward) => {
                on_command(AppCommand::SeekRelative(-5.0));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::SeekForward) => {
                on_command(AppCommand::SeekRelative(5.0));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::VolumeUp) => {
                let v = (vol.value() + 5.0).clamp(0.0, 100.0);
                vol.set_value(v);
                on_command(AppCommand::SetVolume(v));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::VolumeDown) => {
                let v = (vol.value() - 5.0).clamp(0.0, 100.0);
                vol.set_value(v);
                on_command(AppCommand::SetVolume(v));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::ToggleFullscreen) => {
                on_command(AppCommand::SetFullscreen(!w.is_fullscreen()));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::Screenshot) => {
                on_command(AppCommand::Screenshot);
                glib::Propagation::Stop
            }
            Some(ShortcutAction::ToggleMute) => {
                on_command(AppCommand::ToggleMute);
                glib::Propagation::Stop
            }
            Some(ShortcutAction::Stop) => {
                on_command(AppCommand::Stop);
                glib::Propagation::Stop
            }
            Some(ShortcutAction::SpeedDown) => {
                let cur = current_speed.get();
                let new_speed = (cur - 0.25_f64).max(0.25);
                current_speed.set(new_speed);
                on_command(AppCommand::SetSpeed(new_speed));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::SpeedUp) => {
                let cur = current_speed.get();
                let new_speed = (cur + 0.25_f64).min(4.0);
                current_speed.set(new_speed);
                on_command(AppCommand::SetSpeed(new_speed));
                glib::Propagation::Stop
            }
            Some(ShortcutAction::OpenFile) => {
                open_button.emit_clicked();
                glib::Propagation::Stop
            }
            Some(ShortcutAction::TogglePlaylist) => {
                playlist_button.emit_clicked();
                glib::Propagation::Stop
            }
            Some(ShortcutAction::OpenSettings) => {
                settings_button.emit_clicked();
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
    let pointer_over_controls = Rc::new(Cell::new(false));

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
    let show_on_hover = show_controls.clone();
    let cancel_on_hover = cancel_timer.clone();
    let show = show_controls;
    let cancel = cancel_timer;
    let hide = hide_controls;
    let timer = hide_timer;
    let seek_bar = player_view.controls().seek_bar().clone();
    let w = wrapper;
    let is_hovered = pointer_over_controls.clone();
    let show_and_schedule = {
        let show = show.clone();
        let cancel = cancel.clone();
        let hide = hide.clone();
        let timer = timer.clone();
        let seek_bar = seek_bar.clone();
        let w = w.clone();
        let is_hovered = is_hovered.clone();
        move || {
            show();
            cancel();
            if is_hovered.get() {
                return;
            }
            let hc = hide.clone();
            let tr = timer.clone();
            let sb = seek_bar.clone();
            let wr = w.clone();
            let hovered = is_hovered.clone();
            let id =
                glib::timeout_add_local_once(std::time::Duration::from_millis(2200), move || {
                    tr.borrow_mut().take();
                    if hovered.get() || sb.is_dragging() {
                        return;
                    }
                    if wr.has_css_class("has-media") {
                        hc();
                    }
                });
            *timer.borrow_mut() = Some(id);
        }
    };
    let ss1 = show_and_schedule.clone();
    motion.connect_motion(move |_, _x, _y| {
        ss1();
    });
    let ss2 = show_and_schedule.clone();
    motion.connect_enter(move |_, _x, _y| {
        ss2();
    });
    player_view.widget().add_controller(motion);

    // Hovering the controls pins them in place. Leaving resumes the same
    // delayed hide behavior used by the rest of the player surface.
    let controls_motion = gtk::EventControllerMotion::new();
    let hovered_on_enter = pointer_over_controls.clone();
    controls_motion.connect_enter(move |_, _, _| {
        hovered_on_enter.set(true);
        show_on_hover();
        cancel_on_hover();
    });
    let hovered_on_leave = pointer_over_controls;
    let schedule_on_leave = show_and_schedule;
    controls_motion.connect_leave(move |_| {
        hovered_on_leave.set(false);
        schedule_on_leave();
    });
    w.add_controller(controls_motion);
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

/// Keep the floating controls stable. The playlist is a true overlay and must
/// not resize or shift the controls when it opens.
fn connect_controls_resize(_window: &adw::ApplicationWindow, player_view: &PlayerView) {
    let wrapper = player_view.controls_wrapper.clone();
    let overlay = player_view.widget().clone();
    let panel = player_view.playlist_panel.root.clone();

    // Initial sizing
    {
        let wrapper = wrapper.clone();
        let overlay = overlay.clone();
        let panel = panel.clone();
        glib::idle_add_local_once(move || {
            apply_player_layout(overlay.width(), &wrapper, &panel);
        });
    }

    let last_width: Rc<Cell<i32>> = Rc::new(Cell::new(0));
    let wrapper2 = wrapper;
    let overlay2 = overlay;
    let panel2 = panel;
    overlay2.add_tick_callback(move |ov, _| {
        let ow = ov.width();
        if ow != last_width.get() && ow > 100 {
            last_width.set(ow);
            apply_player_layout(ow, &wrapper2, &panel2);
        }
        glib::ControlFlow::Continue
    });
}

fn apply_player_layout(overlay_width: i32, controls: &gtk::Box, playlist: &gtk::Box) {
    if overlay_width <= 100 {
        return;
    }

    let preferred_panel_width = (overlay_width as f64 * 0.36).clamp(360.0, 720.0) as i32;
    let panel_width = preferred_panel_width.min(overlay_width.saturating_sub(440).max(320));
    playlist.set_size_request(panel_width, -1);

    let controls_width = ((overlay_width as f64 * 0.56).clamp(460.0, 820.0) as i32)
        .min(overlay_width.saturating_sub(28))
        .max(280);
    controls.set_size_request(controls_width, -1);
    controls.set_halign(gtk::Align::Center);
    controls.set_valign(gtk::Align::End);
    controls.set_margin_start(0);
    controls.set_margin_top(0);
    controls.set_margin_end(0);
    controls.set_margin_bottom(32);
}
