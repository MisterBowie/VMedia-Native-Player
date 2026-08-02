use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{error, info};

const HISTORY_FILENAME: &str = "playback_history.json";

/// JSON structure on disk.
#[derive(Serialize, Deserialize, Default)]
struct HistoryData {
    #[serde(default)]
    positions: HashMap<String, f64>,
    #[serde(default)]
    last_media: Option<String>,
}

/// Stores playback positions keyed by absolute file path.
/// Persists as JSON in XDG data directory.
#[derive(Clone)]
pub struct PlaybackHistory {
    data_path: PathBuf,
    positions: HashMap<String, f64>,
    last_media: Option<PathBuf>,
}

impl PlaybackHistory {
    pub fn load() -> Self {
        let data_dir = dirs_data_dir().join("vmedia");
        let _ = std::fs::create_dir_all(&data_dir);
        let data_path = data_dir.join(HISTORY_FILENAME);

        let data = if data_path.exists() {
            match std::fs::read_to_string(&data_path) {
                Ok(content) => {
                    // Try new format first, fall back to old HashMap-only format
                    serde_json::from_str::<HistoryData>(&content).unwrap_or_else(|_| {
                        // Legacy: file was just a HashMap<String, f64>
                        let positions: HashMap<String, f64> =
                            serde_json::from_str(&content).unwrap_or_default();
                        HistoryData {
                            positions,
                            last_media: None,
                        }
                    })
                }
                Err(e) => {
                    error!(?e, "failed to read playback history");
                    HistoryData::default()
                }
            }
        } else {
            HistoryData::default()
        };

        info!(count = data.positions.len(), last = ?data.last_media, "loaded playback history");

        Self {
            data_path,
            positions: data.positions,
            last_media: data.last_media.map(PathBuf::from),
        }
    }

    /// Get saved position for a file (in seconds).
    pub fn get_position(&self, path: &Path) -> Option<f64> {
        let key = path.to_string_lossy().to_string();
        self.positions.get(&key).copied().filter(|&p| p > 2.0)
    }

    /// Save position for a file (in seconds).
    pub fn save_position(&mut self, path: &Path, position_seconds: f64) {
        // Only save if > 2 seconds into the file
        if position_seconds < 2.0 {
            return;
        }
        let key = path.to_string_lossy().to_string();
        self.positions.insert(key, position_seconds);
        self.flush();
    }

    /// Remove position entry (e.g. when playback completes).
    pub fn remove_position(&mut self, path: &Path) {
        let key = path.to_string_lossy().to_string();
        if self.positions.remove(&key).is_some() {
            self.flush();
        }
    }

    /// Set the last played media file.
    pub fn set_last_media(&mut self, path: &Path) {
        self.last_media = Some(path.to_path_buf());
        self.flush();
    }

    /// Get the last played media file path.
    pub fn last_media(&self) -> Option<&Path> {
        self.last_media.as_deref()
    }

    fn flush(&self) {
        let data = HistoryData {
            positions: self.positions.clone(),
            last_media: self.last_media.as_ref().map(|p| p.to_string_lossy().to_string()),
        };
        match serde_json::to_string_pretty(&data) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&self.data_path, json) {
                    error!(?e, "failed to write playback history");
                }
            }
            Err(e) => {
                error!(?e, "failed to serialize playback history");
            }
        }
    }
}

fn dirs_data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_DATA_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/share")
    } else {
        PathBuf::from("/tmp")
    }
}
