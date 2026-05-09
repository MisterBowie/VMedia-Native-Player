use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum AppCommand {
    // === 阶段 1：基础播控 ===
    OpenFile(PathBuf),
    TogglePause,
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetVolume(f64),
    SelectSubtitle(i64),
    SelectAudio(i64),
    SetFullscreen(bool),
    LoadExternalSubtitle(PathBuf),
    Stop,
    RefreshState,

    // === 阶段 2：增强播控 ===
    SetSpeed(f64),
    SetSubtitleDelay(f64),
    SetAudioDelay(f64),
    Screenshot,
    SetABLoop { a: Option<f64>, b: Option<f64> },
    ToggleMute,
}
