use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;

const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "ts", "mpg", "mpeg", "mp3", "flac",
    "wav", "ogg", "m4a", "aac", "wma",
];

/// A right-side sliding playlist panel.
#[derive(Clone)]
pub struct PlaylistPanel {
    /// The outer container (right-aligned, fills height).
    pub root: gtk::Box,
    list_box: gtk::ListBox,
    count_label: gtk::Label,
    duration_labels: Rc<std::cell::RefCell<Vec<gtk::Label>>>,
    item_icons: Rc<std::cell::RefCell<Vec<gtk::Image>>>,
    current_dir: std::cell::RefCell<Option<PathBuf>>,
    current_file: std::cell::RefCell<Option<PathBuf>>,
    file_paths: Rc<std::cell::RefCell<Vec<PathBuf>>>,
    on_open: Rc<std::cell::RefCell<Option<Rc<dyn Fn(PathBuf)>>>>,
}

impl PlaylistPanel {
    pub fn new() -> Self {
        // IINA-style playlist / chapter tabs.
        let playlist_tab = gtk::Button::builder()
            .label("播放列表")
            .css_classes(["flat", "playlist-tab", "active"])
            .build();

        let chapter_tab = gtk::Button::builder()
            .label("章节")
            .sensitive(false)
            .css_classes(["flat", "playlist-tab"])
            .build();

        let tabs = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .homogeneous(true)
            .css_classes(["playlist-tabs"])
            .build();
        tabs.append(&playlist_tab);
        tabs.append(&chapter_tab);

        // Scrollable list
        let list_box = gtk::ListBox::new();
        list_box.set_selection_mode(gtk::SelectionMode::Single);
        list_box.set_activate_on_single_click(false);
        list_box.add_css_class("playlist-list");

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&list_box)
            .css_classes(["playlist-scroll"])
            .build();

        let count_label = gtk::Label::builder()
            .label("共 0 项")
            .css_classes(["playlist-total"])
            .build();

        let footer_left = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(2)
            .build();
        for (icon, tooltip) in [
            ("media-playlist-repeat-symbolic", "循环播放"),
            ("media-playlist-shuffle-symbolic", "随机播放"),
            ("view-sort-ascending-symbolic", "排序"),
        ] {
            footer_left.append(
                &gtk::Button::builder()
                    .icon_name(icon)
                    .tooltip_text(tooltip)
                    .css_classes(["flat", "playlist-footer-button"])
                    .build(),
            );
        }

        let footer_right = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(2)
            .build();
        for (icon, tooltip) in [
            ("list-add-symbolic", "添加文件"),
            ("list-remove-symbolic", "移除所选项目"),
            ("user-trash-symbolic", "清空播放列表"),
        ] {
            footer_right.append(
                &gtk::Button::builder()
                    .icon_name(icon)
                    .tooltip_text(tooltip)
                    .css_classes(["flat", "playlist-footer-button"])
                    .build(),
            );
        }

        let footer = gtk::CenterBox::new();
        footer.set_start_widget(Some(&footer_left));
        footer.set_center_widget(Some(&count_label));
        footer.set_end_widget(Some(&footer_right));
        footer.add_css_class("playlist-footer");

        let root = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .width_request(440)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Fill)
            .vexpand(true)
            .visible(false)
            .css_classes(["playlist-panel"])
            .build();
        root.append(&tabs);
        root.append(&scroll);
        root.append(&footer);

        Self {
            root,
            list_box,
            count_label,
            duration_labels: Rc::new(std::cell::RefCell::new(Vec::new())),
            item_icons: Rc::new(std::cell::RefCell::new(Vec::new())),
            current_dir: std::cell::RefCell::new(None),
            current_file: std::cell::RefCell::new(None),
            file_paths: Rc::new(std::cell::RefCell::new(Vec::new())),
            on_open: Rc::new(std::cell::RefCell::new(None)),
        }
    }

    pub fn toggle(&self) {
        self.root.set_visible(!self.root.is_visible());
    }

    /// Scan the directory of `media_path` for media files and populate the list.
    /// Highlight `media_path` as the currently playing item.
    pub fn update_for_media(&self, media_path: &Path) {
        let Some(dir) = media_path.parent() else {
            return;
        };

        // Only rescan if directory changed
        let dir_changed = {
            let cur = self.current_dir.borrow();
            cur.as_deref() != Some(dir)
        };

        if dir_changed {
            self.scan_directory(dir);
            *self.current_dir.borrow_mut() = Some(dir.to_path_buf());
        }

        // Only re-highlight if the playing file actually changed
        let file_changed = {
            let cur = self.current_file.borrow();
            cur.as_deref() != Some(media_path)
        };
        if file_changed {
            self.highlight_file(media_path);
            *self.current_file.borrow_mut() = Some(media_path.to_path_buf());
        }
    }

    /// Set the callback for opening a file from the playlist.
    pub fn connect_activate<F: Fn(PathBuf) + 'static>(&self, on_open: F) {
        *self.on_open.borrow_mut() = Some(Rc::new(on_open));
    }

    fn scan_directory(&self, dir: &Path) {
        // Remove all existing rows
        while let Some(child) = self.list_box.first_child() {
            self.list_box.remove(&child);
        }
        self.duration_labels.borrow_mut().clear();
        self.item_icons.borrow_mut().clear();

        let mut files: Vec<PathBuf> = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut entries: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| MEDIA_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                })
                .collect();

            // Sort by filename
            entries.sort_by(|a, b| {
                a.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .cmp(&b.file_name().to_string_lossy().to_lowercase())
            });

            for (i, entry) in entries.iter().enumerate() {
                let path = entry.path();
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let label = gtk::Label::builder()
                    .label(&name)
                    .halign(gtk::Align::Start)
                    .hexpand(true)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .max_width_chars(35)
                    .css_classes(["playlist-item-label"])
                    .build();

                let item_icon = gtk::Image::from_icon_name("media-playback-start-symbolic");
                item_icon.set_pixel_size(14);
                item_icon.set_opacity(0.0);
                item_icon.add_css_class("playlist-item-icon");

                let duration_label = gtk::Label::builder()
                    .label("")
                    .width_chars(5)
                    .xalign(1.0)
                    .css_classes(["playlist-item-duration"])
                    .build();

                let content = gtk::Box::builder()
                    .orientation(gtk::Orientation::Horizontal)
                    .spacing(11)
                    .build();
                content.append(&item_icon);
                content.append(&label);
                content.append(&duration_label);

                let row = gtk::ListBoxRow::builder()
                    .child(&content)
                    .css_classes(["playlist-item"])
                    .build();

                // Add double-click gesture to each row
                let gesture = gtk::GestureClick::builder().button(1).build();
                let file_paths = self.file_paths.clone();
                let on_open = self.on_open.clone();
                let idx = i;
                gesture.connect_released(move |g, n, _, _| {
                    if n == 2 {
                        let paths = file_paths.borrow();
                        if let Some(path) = paths.get(idx) {
                            if let Some(ref cb) = *on_open.borrow() {
                                cb(path.clone());
                            }
                        }
                        g.set_state(gtk::EventSequenceState::Claimed);
                    }
                });
                row.add_controller(gesture);

                self.list_box.append(&row);
                self.item_icons.borrow_mut().push(item_icon);
                self.duration_labels.borrow_mut().push(duration_label);
                files.push(path);
            }
        }

        self.count_label.set_text(&format!("共 {} 项", files.len()));
        *self.file_paths.borrow_mut() = files;
    }

    fn highlight_file(&self, path: &Path) {
        let files = self.file_paths.borrow();
        let icons = self.item_icons.borrow();
        let mut idx = 0;
        let mut row = self.list_box.row_at_index(0);
        while let Some(r) = row {
            let is_current = files.get(idx).is_some_and(|p| p == path);
            if let Some(icon) = icons.get(idx) {
                icon.set_opacity(if is_current { 1.0 } else { 0.0 });
            }
            if is_current {
                self.list_box.select_row(Some(&r));
            }
            idx += 1;
            row = self.list_box.row_at_index(idx as i32);
        }
    }

    pub fn set_media_duration(&self, path: &Path, seconds: f64) {
        if seconds <= 0.0 {
            return;
        }

        let files = self.file_paths.borrow();
        let Some(index) = files.iter().position(|file| file == path) else {
            return;
        };
        if let Some(label) = self.duration_labels.borrow().get(index) {
            label.set_text(&format_duration(seconds));
        }
    }
}

fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0) as u64;
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}
