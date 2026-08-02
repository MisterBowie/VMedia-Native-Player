use std::{collections::BTreeMap, fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use super::xdg::XdgPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutAction {
    TogglePause,
    SeekBackward,
    SeekForward,
    VolumeUp,
    VolumeDown,
    ToggleFullscreen,
    ToggleMute,
    Screenshot,
    Stop,
    SpeedDown,
    SpeedUp,
    OpenFile,
    TogglePlaylist,
    OpenSettings,
}

impl ShortcutAction {
    pub const ALL: [Self; 14] = [
        Self::TogglePause,
        Self::SeekBackward,
        Self::SeekForward,
        Self::VolumeUp,
        Self::VolumeDown,
        Self::ToggleFullscreen,
        Self::ToggleMute,
        Self::Screenshot,
        Self::Stop,
        Self::SpeedDown,
        Self::SpeedUp,
        Self::OpenFile,
        Self::TogglePlaylist,
        Self::OpenSettings,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::TogglePause => "播放 / 暂停",
            Self::SeekBackward => "快退 5 秒",
            Self::SeekForward => "快进 5 秒",
            Self::VolumeUp => "提高音量",
            Self::VolumeDown => "降低音量",
            Self::ToggleFullscreen => "切换全屏",
            Self::ToggleMute => "静音 / 取消静音",
            Self::Screenshot => "截图",
            Self::Stop => "停止播放",
            Self::SpeedDown => "降低播放速度",
            Self::SpeedUp => "提高播放速度",
            Self::OpenFile => "打开媒体文件",
            Self::TogglePlaylist => "显示 / 隐藏播放列表",
            Self::OpenSettings => "打开快捷键设置",
        }
    }

    pub const fn default_accelerator(self) -> &'static str {
        match self {
            Self::TogglePause => "space",
            Self::SeekBackward => "Left",
            Self::SeekForward => "Right",
            Self::VolumeUp => "Up",
            Self::VolumeDown => "Down",
            Self::ToggleFullscreen => "f",
            Self::ToggleMute => "m",
            Self::Screenshot => "s",
            Self::Stop => "q",
            Self::SpeedDown => "bracketleft",
            Self::SpeedUp => "bracketright",
            Self::OpenFile => "<Control>o",
            Self::TogglePlaylist => "p",
            Self::OpenSettings => "<Control>comma",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSettings {
    bindings: BTreeMap<ShortcutAction, Option<String>>,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        let bindings = ShortcutAction::ALL
            .into_iter()
            .map(|action| (action, Some(action.default_accelerator().to_string())))
            .collect();
        Self { bindings }
    }
}

impl ShortcutSettings {
    pub fn load() -> Self {
        let path = Self::path();
        let Ok(contents) = fs::read_to_string(path) else {
            return Self::default();
        };

        let Ok(mut settings) = serde_json::from_str::<Self>(&contents) else {
            return Self::default();
        };
        settings.fill_missing_defaults();
        settings
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temporary_path = path.with_extension("json.tmp");
        let contents = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(&temporary_path, contents)?;
        fs::rename(temporary_path, path)
    }

    pub fn binding(&self, action: ShortcutAction) -> Option<&str> {
        self.bindings.get(&action).and_then(Option::as_deref)
    }

    pub fn set_binding(&mut self, action: ShortcutAction, accelerator: Option<String>) {
        self.bindings.insert(action, accelerator);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    fn fill_missing_defaults(&mut self) {
        for action in ShortcutAction::ALL {
            self.bindings
                .entry(action)
                .or_insert_with(|| Some(action.default_accelerator().to_string()));
        }
    }

    fn path() -> PathBuf {
        XdgPaths::new().config_dir.join("shortcuts.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_cover_every_action_without_conflicts() {
        let settings = ShortcutSettings::default();
        let mut bindings = std::collections::BTreeSet::new();

        for action in ShortcutAction::ALL {
            let binding = settings.binding(action).expect("default binding");
            assert!(bindings.insert(binding));
        }
    }

    #[test]
    fn serialized_settings_preserve_disabled_bindings() {
        let mut settings = ShortcutSettings::default();
        settings.set_binding(ShortcutAction::Screenshot, None);

        let json = serde_json::to_string(&settings).expect("serialize settings");
        let restored: ShortcutSettings = serde_json::from_str(&json).expect("restore settings");

        assert_eq!(restored.binding(ShortcutAction::Screenshot), None);
    }
}
