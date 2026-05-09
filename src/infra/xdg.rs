use std::path::PathBuf;

/// XDG Base Directory paths for VMedia.
///
/// - Config: `$XDG_CONFIG_HOME/vmedia/` (default `~/.config/vmedia/`)
/// - Data:   `$XDG_DATA_HOME/vmedia/`   (default `~/.local/share/vmedia/`)
/// - Cache:  `$XDG_CACHE_HOME/vmedia/`  (default `~/.cache/vmedia/`)
pub struct XdgPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl XdgPaths {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());

        let config_dir = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".config"))
            .join("vmedia");

        let data_dir = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".local/share"))
            .join("vmedia");

        let cache_dir = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".cache"))
            .join("vmedia");

        Self {
            config_dir,
            data_dir,
            cache_dir,
        }
    }

    /// Ensure all XDG directories exist.
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    /// Path for the SQLite database.
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("vmedia.db")
    }

    /// Path for screenshot output.
    pub fn screenshot_dir(&self) -> PathBuf {
        self.cache_dir.join("screenshots")
    }

    /// Path for poster image cache.
    pub fn poster_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("posters")
    }
}

impl Default for XdgPaths {
    fn default() -> Self {
        Self::new()
    }
}
