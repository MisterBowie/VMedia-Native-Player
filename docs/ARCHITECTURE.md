# VMedia Native Player — 技术架构文档

> 本文档描述项目的技术选型、分层架构、运行模型和核心 API 设计。

## 1. 项目定位

- 聚焦 **Linux-only 本地播放器**
- 产品体验参考 **IINA**，强调简洁、轻盈、以播放为中心
- 优先保证 **底层稳定性、可扩展性、可维护性**
- 不走 Jellyfin / Plex 式的平台化路线

## 2. 技术栈

| 层 | 技术 |
|---|---|
| UI | GTK4 + Libadwaita |
| 播放内核 | libmpv |
| 核心逻辑 | Rust（2024 Edition） |
| 数据持久化 | JSON（当前）→ SQLite（规划） |
| 主循环 | glib main loop |
| 媒体信息 | ffprobe / ffmpeg（规划） |

## 3. 设计原则

- **单进程**，不区分"前端 / 后端"
- UI 只负责展示和交互，不直接承载核心业务
- 所有播放器状态集中管理（`AppState`），避免分散在多个控件中
- 用**命令 / 事件模型**组织模块交互

## 4. 分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                        UI 层 (GTK4)                         │
│  window.rs · player_view.rs · widgets/ · style.css          │
├──────────────────────┬──────────────────────────────────────┤
│     Core 层          │           Player 层                  │
│  command.rs          │  libmpv.rs (FFI 封装)                │
│  event.rs            │  mpv_controller.rs                   │
│  state.rs            │  mpv_events.rs                       │
│  models.rs           │  playback_state.rs                   │
├──────────────────────┴──────────────────────────────────────┤
│                      Infra 层                               │
│  config.rs · db.rs · logging.rs · playback_history.rs       │
│  xdg.rs · dbus.rs(规划) · screensaver_inhibit.rs(规划)      │
└─────────────────────────────────────────────────────────────┘
```

### 4.1 Core 层

纯业务层，不依赖 GTK。

**职责**：播放状态模型、应用命令定义、应用事件定义、播放历史、设置状态。

### 4.2 Player 层

只负责与 libmpv 交互。

**职责**：初始化 mpv 实例、打开/停止媒体、暂停/播放、Seek、倍速、音量、全屏、字幕/音轨切换、截图、mpv 事件接收与转换。

### 4.3 Infra 层

基础设施。

**职责**：数据持久化、配置读写、文件系统、日志、XDG 路径管理、D-Bus 服务（规划）。

### 4.4 UI 层

GTK/Libadwaita 组件与窗口。

**职责**：主窗口、播放视图、控制条、快捷键绑定、右键菜单、播放列表面板。

## 5. 目录结构

```text
src/
  main.rs                      # 程序入口
  app.rs                       # 应用生命周期与事件循环

  core/
    command.rs                 # AppCommand 枚举
    event.rs                   # AppEvent 枚举
    state.rs                   # AppState 集中状态
    models.rs                  # 数据模型（MediaInfo 等）

  player/
    libmpv.rs                  # libmpv FFI 封装
    mpv_controller.rs          # 命令 → mpv 指令翻译
    mpv_events.rs              # mpv 事件 → AppEvent 翻译
    playback_state.rs          # 播放状态追踪

  infra/
    config.rs                  # 应用配置
    db.rs                      # 数据库（预留）
    logging.rs                 # 日志初始化
    playback_history.rs        # JSON 持久化（位置 + 上次文件）
    xdg.rs                     # XDG 目录路径

  ui/
    style.css                  # UI 主题（毛玻璃、暗色模式）
    window.rs                  # 窗口设置、快捷键、自动隐藏、拖动
    player_view.rs             # 主视图（叠加层、GLArea、控件）
    widgets/
      player_controls.rs       # 控制栏组装
      seek_bar.rs              # 自定义进度条
      playlist_panel.rs        # 右侧滑出播放列表
```

## 6. 运行时数据流

```
用户操作 (键盘/鼠标/控件)
    │
    ▼
AppCommand (枚举)
    │
    ▼
MpvController.handle_command()
    │
    ├──▶ libmpv FFI 调用
    │
    ▼
AppEvent (枚举)
    │
    ▼
AppState.apply_event()
    │
    ▼
UI.render(&AppState)  ──▶ GTK 更新界面
```

## 7. 核心 API

### 7.1 AppCommand

```rust
pub enum AppCommand {
    OpenFile(PathBuf),
    TogglePause,
    SeekRelative(f64),
    SeekAbsolute(f64),
    SetVolume(f64),
    SetSpeed(f64),
    ToggleMute,
    SelectSubtitle(i64),
    SelectAudio(i64),
    SetFullscreen(bool),
    LoadExternalSubtitle(PathBuf),
    Stop,
    Screenshot,
    SetABLoop { a: Option<f64>, b: Option<f64> },
    // 规划: ScanLibrary, UpdateSetting
}
```

### 7.2 PlaybackState

| 字段 | 类型 | 说明 |
|---|---|---|
| `current_media` | `Option<MediaSource>` | 当前媒体 |
| `is_paused` | `bool` | 暂停状态 |
| `position_seconds` | `f64` | 当前播放位置 |
| `duration_seconds` | `f64` | 总时长 |
| `volume` | `f64` | 音量 (0.0 ~ 1.0) |
| `speed` | `f64` | 播放速度 |
| `is_fullscreen` | `bool` | 全屏状态 |
| `is_muted` | `bool` | 静音状态 |

## 8. 数据持久化

### 当前实现（JSON）

```
~/.local/share/vmedia/playback_history.json
```

```json
{
  "positions": { "/path/to/video.mp4": 1234.5 },
  "last_media": "/path/to/video.mp4"
}
```

### 规划（SQLite）

MediaItem 字段：`path`, `title`, `duration`, `resolution`, `play_position`, `watched`, `last_played`, `poster_url`, `codec` 等。

## 9. Linux 桌面集成（规划）

| 功能 | 接口 | 状态 |
|---|---|---|
| MPRIS D-Bus | `org.mpris.MediaPlayer2` | 🔲 规划 |
| 屏保抑制 | `org.freedesktop.ScreenSaver` | 🔲 规划 |
| XDG 目录 | `$XDG_DATA_HOME/vmedia/` | ✅ 已实现 |
| .desktop 文件 | `vmedia.desktop` | 🔲 规划 |
| 命令行打开 | `vmedia file.mp4` | ✅ 已实现 |
