use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaSource {
    pub path: PathBuf,
    pub display_name: String,
}

impl MediaSource {
    pub fn from_path(path: PathBuf) -> Self {
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| path.display().to_string());

        Self { path, display_name }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackKind {
    Audio,
    Subtitle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: i64,
    pub label: String,
    pub kind: TrackKind,
    pub selected: bool,
}

/// Media information extracted from the player backend.
#[derive(Debug, Clone, Default)]
pub struct MediaInfo {
    pub media_title: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub video_width: Option<i64>,
    pub video_height: Option<i64>,
}
