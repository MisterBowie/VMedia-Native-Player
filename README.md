<div align="center">

# 🎬 VMedia

**A modern, lightweight, native video player built with Rust**

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange?logo=rust)](https://www.rust-lang.org/)
[![GTK4](https://img.shields.io/badge/GTK4-Libadwaita-4A86CF?logo=gnome)](https://gtk.org/)
[![MPV](https://img.shields.io/badge/Backend-libmpv-690D8E)](https://mpv.io/)
[![License](https://img.shields.io/badge/License-MIT-green)](#license)

[English](#overview) · [中文文档](README_ZH.md)

---

<img src="docs/screenshots/preview.png" alt="VMedia Preview" width="720" />

</div>

## Overview

VMedia is a native Linux video player that combines the decoding power of **libmpv** with the modern UI capabilities of **GTK4 + Libadwaita**. It delivers a sleek, minimalist experience with a floating glass-effect control panel, hardware-accelerated video rendering, and smart playback resume.

## ✨ Features

### 🎥 Playback
- **Hardware-accelerated rendering** — OpenGL-based video output via `libmpv` render API
- **Wide format support** — Plays all formats supported by ffmpeg/mpv (MP4, MKV, AVI, WebM, FLAC, MP3, etc.)
- **Playback speed control** — 0.25x to 3.0x with quick-access speed selector
- **Smart resume** — Remembers playback position and restores on next launch
- **Seek** — Click or drag the progress bar; keyboard shortcuts for ±5s / ±30s jumps

### 🎛️ Control Panel
- **Floating glass panel** — Semi-transparent, blurred background with rounded corners
- **Auto-hide** — Hides after 1.5s of inactivity; reappears on mouse movement
- **Draggable** — Drag the control bar anywhere on the screen
- **Responsive width** — Scales to 60% of window width, capped at 600px max
- **Circular play button** — Minimalist design with smooth hover effects

### 🔊 Audio
- **Volume control** — Vertical popup slider on hover over volume icon
- **5-state volume icon** — Visual feedback: mute, 25%, 50%, 75%, 100%
- **Mute toggle** — Click volume icon or press `M`

### 📂 Playlist
- **Auto-populated** — Scans the directory of the opened file for media files
- **Full-height panel** — Slides in from the right with translucent background
- **Highlight current** — Active file is visually highlighted
- **Double-click to play** — Switch between files seamlessly

### 💾 Persistence
- **Remember last file** — Reopens the last played video on launch (paused)
- **Position memory** — Saves playback position every 5 seconds
- **Playlist restoration** — Restores the playlist from the last session

## 🏗️ Architecture

```
native-player/src/
├── main.rs                    # Entry point
├── app.rs                     # Application lifecycle & event loop
├── core/
│   ├── command.rs             # Command enum (OpenFile, Seek, Pause, etc.)
│   ├── event.rs               # Event enum (state change notifications)
│   ├── models.rs              # Data models (MediaInfo)
│   └── state.rs               # Application state (AppState)
├── player/
│   ├── libmpv.rs              # libmpv FFI wrapper
│   ├── mpv_controller.rs      # Command → mpv translation
│   ├── mpv_events.rs          # mpv → AppEvent translation
│   └── playback_state.rs      # Playback state tracking
├── infra/
│   ├── config.rs              # App configuration
│   ├── db.rs                  # Database (future use)
│   ├── logging.rs             # Tracing setup
│   ├── playback_history.rs    # JSON-based position & last-file persistence
│   └── xdg.rs                 # XDG directory paths
└── ui/
    ├── style.css              # Complete UI theme (glassmorphism, dark mode)
    ├── window.rs              # Window setup, shortcuts, autohide, drag logic
    ├── player_view.rs         # Main view (overlay, GLArea, controls)
    └── widgets/
        ├── player_controls.rs # Control bar assembly
        ├── seek_bar.rs        # Custom seek bar with drag awareness
        └── playlist_panel.rs  # Right-side sliding playlist
```

## 📦 Dependencies

| Dependency | Purpose |
|---|---|
| `gtk4` (0.10) | UI framework |
| `libadwaita` (0.8) | Adaptive layouts, dark theme |
| `libmpv` | Video/audio decoding & rendering |
| `serde` + `serde_json` | Playback history persistence |
| `tracing` | Structured logging |

### System Requirements

- **OS**: Linux (X11 / Wayland)
- **GTK**: ≥ 4.10
- **Libadwaita**: ≥ 1.4
- **libmpv**: ≥ 0.36
- **Rust**: 2024 Edition (≥ 1.85)

## 🚀 Quick Start

### Install System Dependencies

**Fedora / RHEL:**
```bash
sudo dnf install gtk4-devel libadwaita-devel mpv-libs-devel
```

**Ubuntu / Debian:**
```bash
sudo apt install libgtk-4-dev libadwaita-1-dev libmpv-dev
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libadwaita mpv
```

### Build & Run

```bash
cd native-player
cargo run
```

### Open a Video File

```bash
cargo run -- /path/to/video.mp4
```

Or use the **folder icon** in the control panel to open a file dialog.

## ⌨️ Keyboard Shortcuts

| Key | Action |
|---|---|
| `Space` | Play / Pause |
| `←` / `→` | Seek ±5 seconds |
| `Shift+←` / `Shift+→` | Seek ±30 seconds |
| `↑` / `↓` | Volume ±5% |
| `M` | Toggle mute |
| `F` / `F11` | Toggle fullscreen |
| `O` | Open file |
| `L` | Toggle playlist |
| `Esc` | Exit fullscreen |

## 📁 Data Storage

Playback history is stored at:
```
~/.local/share/vmedia/playback_history.json
```

Example:
```json
{
  "positions": {
    "/home/user/Videos/movie.mp4": 1234.5
  },
  "last_media": "/home/user/Videos/movie.mp4"
}
```

## 🗺️ Roadmap

- [x] Core playback (play, pause, seek, volume)
- [x] Floating glass control panel with auto-hide
- [x] Draggable controls
- [x] Playlist panel
- [x] Playback resume & persistence
- [ ] Subtitle management (external file loading, delay adjustment)
- [ ] Audio track switching
- [ ] A-B loop
- [ ] Screenshot capture
- [ ] Media library with poster thumbnails
- [ ] Settings panel (keybindings, theme)

## 📄 License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

<div align="center">
  <sub>Built with ❤️ using Rust, GTK4, and libmpv</sub>
</div>
